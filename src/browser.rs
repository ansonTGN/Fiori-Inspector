use crate::config::AppConfig;
use crate::models::PageSnapshot;
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{debug, info, warn};

const UI5_PROBE_JS: &str = include_str!("js/ui5_probe.js");

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Página Chrome/Chromium controlada mediante Chrome DevTools Protocol.
///
/// Importante: esto NO usa ChromeDriver ni GeckoDriver. Se conecta directamente al
/// endpoint CDP de Chrome/Chromium, normalmente http://127.0.0.1:9222.
pub struct CdpPage {
    client: CdpClient,
    _browser_child: Option<Child>,
}

struct CdpClient {
    ws: Ws,
    next_id: u64,
    session_id: Option<String>,
}

impl CdpClient {
    async fn connect(ws_url: &str) -> Result<Self> {
        let (ws, _) = connect_async(ws_url)
            .await
            .with_context(|| format!("No se pudo abrir WebSocket CDP: {ws_url}"))?;
        Ok(Self {
            ws,
            next_id: 0,
            session_id: None,
        })
    }

    async fn command(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let mut payload = json!({
            "id": id,
            "method": method,
            "params": params,
        });
        if let Some(session_id) = &self.session_id {
            payload["sessionId"] = json!(session_id);
        }

        self.ws
            .send(Message::Text(payload.to_string().into()))
            .await
            .with_context(|| format!("No se pudo enviar comando CDP: {method}"))?;

        loop {
            let msg = self.ws.next().await.ok_or_else(|| {
                anyhow::anyhow!("CDP cerró la conexión mientras esperaba respuesta de {method}")
            })??;

            let text = match msg {
                Message::Text(t) => t.to_string(),
                Message::Binary(b) => String::from_utf8_lossy(&b).to_string(),
                Message::Close(frame) => bail!("CDP cerró la conexión: {:?}", frame),
                _ => continue,
            };

            let value: Value =
                serde_json::from_str(&text).context("Respuesta CDP no es JSON válido")?;
            if value.get("id").and_then(Value::as_u64) != Some(id) {
                // Evento CDP asíncrono. Lo ignoramos aquí.
                continue;
            }

            if let Some(error) = value.get("error") {
                bail!("Comando CDP {method} falló: {error}");
            }

            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct CdpVersion {
    #[serde(rename = "webSocketDebuggerUrl")]
    web_socket_debugger_url: String,
}

pub async fn connect_browser(cfg: &AppConfig) -> Result<CdpPage> {
    let mut child = None;

    if fetch_cdp_version(cfg).await.is_err() {
        if cfg.browser.auto_launch {
            info!(
                binary = cfg.browser.chrome_binary,
                "CDP no está activo; lanzando Chrome/Chromium"
            );
            child = Some(launch_chrome(cfg).await?);
            wait_until_cdp_ready(cfg, Duration::from_secs(15)).await?;
        } else {
            bail!(
                "No se pudo conectar con CDP en {}. Arranca Chrome/Chromium con --remote-debugging-port=9222 o activa auto_launch.",
                cfg.browser.cdp_url
            );
        }
    }

    let version = fetch_cdp_version(cfg).await?;
    let mut client = CdpClient::connect(&version.web_socket_debugger_url).await?;

    let target = client
        .command("Target.createTarget", json!({ "url": "about:blank" }))
        .await
        .context("No se pudo crear una pestaña CDP")?;
    let target_id = target
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("CDP no devolvió targetId"))?
        .to_string();

    let attached = client
        .command(
            "Target.attachToTarget",
            json!({ "targetId": target_id, "flatten": true }),
        )
        .await
        .context("No se pudo adjuntar a la pestaña CDP")?;
    let session_id = attached
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("CDP no devolvió sessionId"))?
        .to_string();
    client.session_id = Some(session_id);

    let _ = client.command("Page.enable", json!({})).await;
    let _ = client.command("Runtime.enable", json!({})).await;

    Ok(CdpPage {
        client,
        _browser_child: child,
    })
}

async fn fetch_cdp_version(cfg: &AppConfig) -> Result<CdpVersion> {
    let base = cfg.browser.cdp_url.trim_end_matches('/');
    let url = format!("{base}/json/version");
    reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .with_context(|| format!("No se pudo conectar con CDP en {url}"))?
        .error_for_status()
        .with_context(|| format!("CDP respondió con error en {url}"))?
        .json::<CdpVersion>()
        .await
        .context("La respuesta /json/version de CDP no tiene el formato esperado")
}

async fn launch_chrome(cfg: &AppConfig) -> Result<Child> {
    let parsed =
        url::Url::parse(&cfg.browser.cdp_url).context("browser.cdp_url no es una URL válida")?;
    let port = parsed.port_or_known_default().unwrap_or(9222);
    let user_data_dir = cfg
        .browser
        .user_data_dir
        .clone()
        .unwrap_or_else(|| "./.browser-profile-cdp".to_string());

    let mut cmd = Command::new(&cfg.browser.chrome_binary);
    cmd.arg(format!("--remote-debugging-port={port}"))
        .arg(format!("--user-data-dir={user_data_dir}"))
        .arg(format!(
            "--window-size={},{}",
            cfg.browser.window_width, cfg.browser.window_height
        ))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-dev-shm-usage")
        .arg("--disable-background-networking")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if cfg.browser.accept_insecure_certs {
        cmd.arg("--ignore-certificate-errors");
    }
    if cfg.browser.headless {
        cmd.arg("--headless=new");
    }

    cmd.spawn().with_context(|| {
        format!(
            "No se pudo lanzar '{}'. Ajusta browser.chrome_binary en config/local.toml. Ejemplos: google-chrome, chromium, chromium-browser.",
            cfg.browser.chrome_binary
        )
    })
}

async fn wait_until_cdp_ready(cfg: &AppConfig, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        if fetch_cdp_version(cfg).await.is_ok() {
            return Ok(());
        }
        if Instant::now() > deadline {
            bail!(
                "Chrome/Chromium se lanzó, pero CDP no respondió en {}",
                cfg.browser.cdp_url
            );
        }
        sleep(Duration::from_millis(500)).await;
    }
}

pub async fn snapshot_browser(
    cfg: &AppConfig,
    url: &str,
    output: Option<&Path>,
) -> Result<PageSnapshot> {
    let mut page = connect_browser(cfg).await?;
    page.goto(url)
        .await
        .with_context(|| format!("No se pudo abrir URL: {url}"))?;

    if cfg.fiori.manual_login_wait_secs > 0 {
        info!(
            secs = cfg.fiori.manual_login_wait_secs,
            "Esperando login/manual steps"
        );
        sleep(Duration::from_secs(cfg.fiori.manual_login_wait_secs)).await;
    }

    if cfg.fiori.wait_for_ui5 {
        wait_for_ui5(
            &mut page,
            cfg.fiori.ui5_timeout_secs,
            cfg.fiori.ready_selector.as_deref(),
        )
        .await?;
    }

    let snapshot = extract_snapshot(&mut page, cfg).await?;
    if let Some(path) = output {
        write_snapshot(path, &snapshot, cfg.output.pretty_json).await?;
    }
    Ok(snapshot)
}

impl CdpPage {
    pub async fn goto(&mut self, url: &str) -> Result<()> {
        self.client
            .command("Page.navigate", json!({ "url": url }))
            .await
            .with_context(|| format!("CDP no pudo navegar a {url}"))?;
        wait_for_document_ready(self, 45).await
    }

    pub async fn evaluate(&mut self, expression: &str) -> Result<Value> {
        let result = self
            .client
            .command(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true,
                    "userGesture": true,
                }),
            )
            .await
            .context("Falló Runtime.evaluate vía CDP")?;

        if let Some(exception) = result.get("exceptionDetails") {
            bail!("JavaScript evaluado por CDP lanzó excepción: {exception}");
        }

        Ok(result.get("result").cloned().unwrap_or(Value::Null))
    }
}

async fn wait_for_document_ready(page: &mut CdpPage, timeout_secs: u64) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let value = page
            .evaluate("document.readyState === 'complete' || document.readyState === 'interactive'")
            .await;
        if let Ok(v) = value {
            if v.get("value").and_then(Value::as_bool).unwrap_or(false) {
                return Ok(());
            }
        }
        if Instant::now() > deadline {
            warn!("Timeout esperando document.readyState; se continúa con la captura");
            return Ok(());
        }
        sleep(Duration::from_millis(400)).await;
    }
}

pub async fn extract_snapshot(page: &mut CdpPage, cfg: &AppConfig) -> Result<PageSnapshot> {
    let probe_cfg = json!({
        "max_text_len": cfg.extraction.max_text_len,
        "include_hidden_controls": cfg.extraction.include_hidden_controls,
        "include_dom_nodes": cfg.extraction.include_dom_nodes,
        "include_performance_urls": cfg.extraction.include_performance_urls,
        "max_controls": cfg.extraction.max_controls,
        "max_dom_nodes": cfg.extraction.max_dom_nodes,
    });

    // El probe original está escrito para WebDriver: empieza con `return ... arguments[0]`.
    // Para CDP lo envolvemos dentro de una función local y sustituimos arguments[0]
    // por una variable explícita para evitar conflictos con el objeto especial `arguments`.
    let arg = serde_json::to_string(&probe_cfg)?;
    let probe_js = UI5_PROBE_JS.replace("arguments[0]", "__fioriInspectorArgs[0]");
    let expression = format!(
        "(function() {{ var __fioriInspectorArgs = [{}]; {} }})()",
        arg, probe_js
    );
    let value = page
        .evaluate(&expression)
        .await
        .context("Falló la ejecución del extractor JavaScript dentro de Chrome/CDP")?;
    let raw = value
        .get("value")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("El extractor CDP no devolvió un JSON string"))?;
    let snapshot: PageSnapshot = serde_json::from_str(raw)
        .context("El JSON del snapshot UI5 no coincide con el modelo Rust")?;
    Ok(snapshot)
}

pub async fn wait_for_ui5(
    page: &mut CdpPage,
    timeout_secs: u64,
    ready_selector: Option<&str>,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let selector_json = serde_json::to_string(&ready_selector)?;
    let expression = format!(
        r#"(function(selector) {{
            const selectorOk = !selector || !!document.querySelector(selector);
            const ui5Ok = !!(window.sap && sap.ui && sap.ui.getCore && sap.ui.getCore().isInitialized && sap.ui.getCore().isInitialized());
            return selectorOk && ui5Ok;
        }})({selector_json})"#
    );

    loop {
        match page.evaluate(&expression).await {
            Ok(v) => {
                let ok = v.get("value").and_then(Value::as_bool).unwrap_or(false);
                if ok {
                    debug!("SAPUI5 listo");
                    return Ok(());
                }
            }
            Err(e) => debug!(error = ?e, "wait_for_ui5 todavía no listo"),
        }
        if Instant::now() > deadline {
            bail!("Timeout esperando SAPUI5 inicializado. Puede que la página esté en login, use iframe, o no sea SAPUI5/Fiori.");
        }
        sleep(Duration::from_millis(750)).await;
    }
}

pub async fn click(page: &mut CdpPage, selector: &str) -> Result<()> {
    click_target(page, Some(selector), None, None).await
}

pub async fn input(page: &mut CdpPage, selector: &str, value: &str, clear: bool) -> Result<()> {
    input_target(page, Some(selector), None, value, clear).await
}

pub async fn wait_for_target(
    page: &mut CdpPage,
    selector: Option<&str>,
    control_id: Option<&str>,
    text: Option<&str>,
    timeout_secs: u64,
    require_visible: bool,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let selector_json = serde_json::to_string(&selector)?;
    let control_id_json = serde_json::to_string(&control_id)?;
    let text_json = serde_json::to_string(&text)?;
    let expression = format!(
        r#"(function(selector, controlId, text, requireVisible) {{
            const result = window.__fioriInspectorFindTarget(selector, controlId, text);
            if (!result || !result.el) return false;
            if (!requireVisible) return true;
            const r = result.el.getBoundingClientRect ? result.el.getBoundingClientRect() : null;
            const cs = window.getComputedStyle ? getComputedStyle(result.el) : null;
            return !!r && r.width > 0 && r.height > 0 && (!cs || (cs.visibility !== 'hidden' && cs.display !== 'none'));
        }})({selector_json}, {control_id_json}, {text_json}, {require_visible})"#
    );
    install_runtime_helpers(page).await?;
    loop {
        if let Ok(v) = page.evaluate(&expression).await {
            if v.get("value").and_then(Value::as_bool).unwrap_or(false) {
                return Ok(());
            }
        }
        if Instant::now() > deadline {
            bail!(
                "Timeout esperando target. selector={:?}, control_id={:?}, text={:?}",
                selector,
                control_id,
                text
            );
        }
        sleep(Duration::from_millis(400)).await;
    }
}

pub async fn click_target(
    page: &mut CdpPage,
    selector: Option<&str>,
    control_id: Option<&str>,
    text: Option<&str>,
) -> Result<()> {
    let selector_json = serde_json::to_string(&selector)?;
    let control_id_json = serde_json::to_string(&control_id)?;
    let text_json = serde_json::to_string(&text)?;
    install_runtime_helpers(page).await?;
    let expression = format!(
        r#"(function(selector, controlId, text) {{
            const result = window.__fioriInspectorFindTarget(selector, controlId, text);
            if (!result || !result.el) throw new Error('No se encontró target para click');
            const el = result.el;
            el.scrollIntoView && el.scrollIntoView({{ block: 'center', inline: 'center' }});
            el.focus && el.focus();
            if (result.ctrl && typeof result.ctrl.firePress === 'function') {{
                result.ctrl.firePress();
                return {{ ok: true, mode: 'ui5_firePress' }};
            }}
            if (result.ctrl && typeof result.ctrl.ontap === 'function') {{
                result.ctrl.ontap({{ srcControl: result.ctrl }});
                return {{ ok: true, mode: 'ui5_ontap' }};
            }}
            el.click();
            return {{ ok: true, mode: 'dom_click' }};
        }})({selector_json}, {control_id_json}, {text_json})"#
    );
    page.evaluate(&expression).await.with_context(|| {
        format!(
            "Falló click target selector={:?} control_id={:?} text={:?}",
            selector, control_id, text
        )
    })?;
    Ok(())
}

pub async fn input_target(
    page: &mut CdpPage,
    selector: Option<&str>,
    control_id: Option<&str>,
    value: &str,
    clear: bool,
) -> Result<()> {
    let selector_json = serde_json::to_string(&selector)?;
    let control_id_json = serde_json::to_string(&control_id)?;
    let value_json = serde_json::to_string(value)?;
    install_runtime_helpers(page).await?;
    let expression = format!(
        r#"(function(selector, controlId, value, clear) {{
            const result = window.__fioriInspectorFindTarget(selector, controlId, null);
            if (!result || !result.el) throw new Error('No se encontró target para input');
            const el = result.el;
            el.scrollIntoView && el.scrollIntoView({{ block: 'center', inline: 'center' }});
            el.focus && el.focus();
            if (result.ctrl && typeof result.ctrl.setValue === 'function') {{
                result.ctrl.setValue(value);
                if (typeof result.ctrl.fireLiveChange === 'function') result.ctrl.fireLiveChange({{ value }});
                if (typeof result.ctrl.fireChange === 'function') result.ctrl.fireChange({{ value }});
                return {{ ok: true, mode: 'ui5_setValue' }};
            }}
            if (clear && 'value' in el) el.value = '';
            const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
            const desc = Object.getOwnPropertyDescriptor(proto, 'value');
            if (desc && desc.set) desc.set.call(el, value); else el.value = value;
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            el.dispatchEvent(new Event('blur', {{ bubbles: true }}));
            return {{ ok: true, mode: 'dom_input' }};
        }})({selector_json}, {control_id_json}, {value_json}, {clear})"#
    );
    page.evaluate(&expression).await.with_context(|| {
        format!(
            "Falló input target selector={:?} control_id={:?}",
            selector, control_id
        )
    })?;
    Ok(())
}

pub async fn select_target(
    page: &mut CdpPage,
    selector: Option<&str>,
    control_id: Option<&str>,
    value: &str,
) -> Result<()> {
    let selector_json = serde_json::to_string(&selector)?;
    let control_id_json = serde_json::to_string(&control_id)?;
    let value_json = serde_json::to_string(value)?;
    install_runtime_helpers(page).await?;
    let expression = format!(
        r#"(function(selector, controlId, value) {{
            const result = window.__fioriInspectorFindTarget(selector, controlId, null);
            if (!result || !result.el) throw new Error('No se encontró target para select');
            const el = result.el;
            if (result.ctrl && typeof result.ctrl.setSelectedKey === 'function') {{
                result.ctrl.setSelectedKey(value);
                if (typeof result.ctrl.fireSelectionChange === 'function') result.ctrl.fireSelectionChange({{ selectedItem: result.ctrl.getSelectedItem && result.ctrl.getSelectedItem() }});
                if (typeof result.ctrl.fireChange === 'function') result.ctrl.fireChange({{ selectedItem: result.ctrl.getSelectedItem && result.ctrl.getSelectedItem(), selectedItemId: value }});
                return {{ ok: true, mode: 'ui5_setSelectedKey' }};
            }}
            if ('value' in el) {{
                el.value = value;
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return {{ ok: true, mode: 'dom_select' }};
            }}
            throw new Error('El target no permite selección directa');
        }})({selector_json}, {control_id_json}, {value_json})"#
    );
    page.evaluate(&expression).await.with_context(|| {
        format!(
            "Falló select target selector={:?} control_id={:?}",
            selector, control_id
        )
    })?;
    Ok(())
}

pub async fn assert_condition(
    page: &mut CdpPage,
    selector: Option<&str>,
    control_id: Option<&str>,
    text_contains: Option<&str>,
    url_contains: Option<&str>,
    exists: Option<bool>,
    visible: Option<bool>,
    ui5_ready: Option<bool>,
    timeout_secs: u64,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let selector_json = serde_json::to_string(&selector)?;
    let control_id_json = serde_json::to_string(&control_id)?;
    let text_json = serde_json::to_string(&text_contains)?;
    let url_json = serde_json::to_string(&url_contains)?;
    let exists_json = serde_json::to_string(&exists)?;
    let visible_json = serde_json::to_string(&visible)?;
    let ui5_json = serde_json::to_string(&ui5_ready)?;
    install_runtime_helpers(page).await?;
    let expression = format!(
        r#"(function(selector, controlId, textContains, urlContains, existsExpected, visibleExpected, ui5ReadyExpected) {{
            const checks = [];
            if (urlContains !== null && urlContains !== undefined) checks.push({{ ok: location.href.includes(urlContains), message: 'url_contains=' + urlContains }});
            if (ui5ReadyExpected !== null && ui5ReadyExpected !== undefined) {{
                const ready = !!(window.sap && sap.ui && sap.ui.getCore && sap.ui.getCore().isInitialized && sap.ui.getCore().isInitialized());
                checks.push({{ ok: ready === ui5ReadyExpected, message: 'ui5_ready=' + ui5ReadyExpected }});
            }}
            if (selector || controlId || textContains) {{
                const result = window.__fioriInspectorFindTarget(selector, controlId, null);
                const el = result && result.el;
                if (existsExpected !== null && existsExpected !== undefined) checks.push({{ ok: (!!el) === existsExpected, message: 'exists=' + existsExpected }});
                if (visibleExpected !== null && visibleExpected !== undefined) {{
                    const r = el && el.getBoundingClientRect ? el.getBoundingClientRect() : null;
                    const cs = el && window.getComputedStyle ? getComputedStyle(el) : null;
                    const isVisible = !!r && r.width > 0 && r.height > 0 && (!cs || (cs.visibility !== 'hidden' && cs.display !== 'none'));
                    checks.push({{ ok: isVisible === visibleExpected, message: 'visible=' + visibleExpected }});
                }}
                if (textContains) {{
                    const hay = (el && (el.innerText || el.textContent || el.value || el.getAttribute('aria-label') || '')) || document.body.innerText || '';
                    checks.push({{ ok: hay.includes(textContains), message: 'text_contains=' + textContains }});
                }}
            }}
            const failed = checks.filter(c => !c.ok);
            return {{ ok: failed.length === 0, failed: failed.map(f => f.message) }};
        }})({selector_json}, {control_id_json}, {text_json}, {url_json}, {exists_json}, {visible_json}, {ui5_json})"#
    );

    loop {
        let v = page.evaluate(&expression).await?;
        let obj = v.get("value").cloned().unwrap_or(Value::Null);
        if obj.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return Ok(());
        }
        if Instant::now() > deadline {
            bail!("Assertion fallida: {}", obj);
        }
        sleep(Duration::from_millis(400)).await;
    }
}

pub async fn capture_screenshot_png(page: &mut CdpPage) -> Result<Vec<u8>> {
    let result = page
        .client
        .command(
            "Page.captureScreenshot",
            json!({ "format": "png", "captureBeyondViewport": true }),
        )
        .await
        .context("Falló Page.captureScreenshot vía CDP")?;
    let data = result
        .get("data")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("CDP no devolvió imagen base64"))?;
    general_purpose::STANDARD
        .decode(data)
        .context("No se pudo decodificar screenshot base64")
}

async fn install_runtime_helpers(page: &mut CdpPage) -> Result<()> {
    let expression = r#"
        (function() {
            if (window.__fioriInspectorFindTarget) return true;
            window.__fioriInspectorFindTarget = function(selector, controlId, text) {
                function visible(el) {
                    if (!el) return false;
                    const r = el.getBoundingClientRect ? el.getBoundingClientRect() : null;
                    const cs = window.getComputedStyle ? getComputedStyle(el) : null;
                    return !!r && r.width > 0 && r.height > 0 && (!cs || (cs.visibility !== 'hidden' && cs.display !== 'none'));
                }
                function controlById(id) {
                    try {
                        if (window.sap && sap.ui && sap.ui.getCore) return sap.ui.getCore().byId(id);
                    } catch (_) {}
                    return null;
                }
                if (controlId) {
                    const ctrl = controlById(controlId);
                    const el = ctrl && ctrl.getDomRef && ctrl.getDomRef();
                    if (el) return { ctrl, el, source: 'control_id' };
                    const byDom = document.getElementById(controlId);
                    if (byDom) return { ctrl, el: byDom, source: 'dom_id' };
                }
                if (selector) {
                    const el = document.querySelector(selector);
                    if (el) return { ctrl: null, el, source: 'selector' };
                }
                if (text) {
                    const candidates = Array.from(document.querySelectorAll('button,a,input,textarea,[role="button"],[role="link"],[role="textbox"],[role="tab"],[aria-label]'));
                    const needle = String(text).trim().toLowerCase();
                    const el = candidates.find(e => {
                        const hay = [e.innerText, e.textContent, e.value, e.title, e.getAttribute('aria-label')].filter(Boolean).join(' ').toLowerCase();
                        return hay.includes(needle) && visible(e);
                    });
                    if (el) return { ctrl: null, el, source: 'text' };
                }
                return null;
            };
            return true;
        })();
    "#;
    page.evaluate(expression).await?;
    Ok(())
}

pub async fn press(page: &mut CdpPage, key: &str) -> Result<()> {
    let normalized = match key.to_ascii_lowercase().as_str() {
        "enter" => "Enter",
        "tab" => "Tab",
        "escape" | "esc" => "Escape",
        "space" => " ",
        other => bail!("Tecla no soportada todavía: {other}. Usa Enter, Tab, Escape o Space."),
    };
    let key_json = serde_json::to_string(normalized)?;
    let expression = format!(
        r#"(function(key) {{
            const el = document.activeElement || document.body;
            const opts = {{ key: key, bubbles: true, cancelable: true }};
            el.dispatchEvent(new KeyboardEvent('keydown', opts));
            el.dispatchEvent(new KeyboardEvent('keyup', opts));
            return true;
        }})({key_json})"#
    );
    page.evaluate(&expression)
        .await
        .context("Falló envío de tecla vía CDP")?;
    Ok(())
}

pub async fn write_snapshot(path: &Path, snapshot: &PageSnapshot, pretty: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let data = if pretty {
        serde_json::to_vec_pretty(snapshot)?
    } else {
        serde_json::to_vec(snapshot)?
    };
    tokio::fs::write(path, data)
        .await
        .with_context(|| format!("No se pudo escribir snapshot: {}", path.display()))?;
    Ok(())
}
