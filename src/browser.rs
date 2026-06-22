use crate::config::AppConfig;
use crate::models::PageSnapshot;
use anyhow::{bail, Context, Result};
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};
use thirtyfour::prelude::*;
use tokio::time::sleep;
use tracing::{debug, info, warn};

const UI5_PROBE_JS: &str = include_str!("js/ui5_probe.js");

pub async fn connect_driver(cfg: &AppConfig) -> Result<WebDriver> {
    let browser = cfg.browser.browser.to_ascii_lowercase();

    // ChromeCapabilities y FirefoxCapabilities son tipos distintos en thirtyfour.
    // Por eso no deben mezclarse dentro del mismo `match` asignado a una variable.
    // Creamos el WebDriver dentro de cada rama para que Rust conserve el tipo concreto.
    let driver = match browser.as_str() {
        "firefox" => {
            let mut caps = DesiredCapabilities::firefox();
            caps.accept_insecure_certs(cfg.browser.accept_insecure_certs)?;

            if cfg.browser.headless {
                caps.add_arg("-headless")?;
            }

            WebDriver::new(&cfg.browser.webdriver_url, caps).await
        }
        _ => {
            let mut caps = DesiredCapabilities::chrome();
            caps.accept_insecure_certs(cfg.browser.accept_insecure_certs)?;
            caps.add_arg("--disable-dev-shm-usage")?;
            caps.add_arg("--no-sandbox")?;
            caps.add_arg(&format!(
                "--window-size={},{}",
                cfg.browser.window_width, cfg.browser.window_height
            ))?;

            if cfg.browser.headless {
                caps.add_arg("--headless=new")?;
            }

            if let Some(dir) = &cfg.browser.user_data_dir {
                caps.add_arg(&format!("--user-data-dir={dir}"))?;
            }

            WebDriver::new(&cfg.browser.webdriver_url, caps).await
        }
    }
    .with_context(|| {
        format!(
            "No se pudo conectar con WebDriver en {}. Arranca chromedriver/geckodriver primero.",
            cfg.browser.webdriver_url
        )
    })?;

    if browser != "firefox" {
        let _ = driver
            .set_window_rect(0, 0, cfg.browser.window_width, cfg.browser.window_height)
            .await;
    }

    Ok(driver)
}

pub async fn snapshot_browser(cfg: &AppConfig, url: &str, output: Option<&Path>) -> Result<PageSnapshot> {
    let driver = connect_driver(cfg).await?;
    let result = async {
        driver.goto(url).await.with_context(|| format!("No se pudo abrir URL: {url}"))?;
        if cfg.fiori.manual_login_wait_secs > 0 {
            info!(secs = cfg.fiori.manual_login_wait_secs, "Esperando login/manual steps");
            sleep(Duration::from_secs(cfg.fiori.manual_login_wait_secs)).await;
        }
        if cfg.fiori.wait_for_ui5 {
            wait_for_ui5(&driver, cfg.fiori.ui5_timeout_secs, cfg.fiori.ready_selector.as_deref()).await?;
        }
        let snapshot = extract_snapshot(&driver, cfg).await?;
        if let Some(path) = output {
            write_snapshot(path, &snapshot, cfg.output.pretty_json).await?;
        }
        Ok(snapshot)
    }
    .await;

    if let Err(e) = driver.quit().await {
        warn!(error = ?e, "No se pudo cerrar WebDriver limpiamente");
    }
    result
}

pub async fn extract_snapshot(driver: &WebDriver, cfg: &AppConfig) -> Result<PageSnapshot> {
    let args = vec![json!({
        "max_text_len": cfg.extraction.max_text_len,
        "include_hidden_controls": cfg.extraction.include_hidden_controls,
        "include_dom_nodes": cfg.extraction.include_dom_nodes,
        "include_performance_urls": cfg.extraction.include_performance_urls,
        "max_controls": cfg.extraction.max_controls,
        "max_dom_nodes": cfg.extraction.max_dom_nodes,
    })];
    let ret = driver
        .execute(UI5_PROBE_JS, args)
        .await
        .context("Falló la ejecución del extractor JavaScript dentro del navegador")?;
    let raw: String = ret.convert().context("El extractor no devolvió JSON string")?;
    let snapshot: PageSnapshot = serde_json::from_str(&raw).context("El JSON del snapshot UI5 no coincide con el modelo Rust")?;
    Ok(snapshot)
}

pub async fn wait_for_ui5(driver: &WebDriver, timeout_secs: u64, ready_selector: Option<&str>) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let js = r#"
            return (function(selector) {
                const selectorOk = !selector || !!document.querySelector(selector);
                const ui5Ok = !!(window.sap && sap.ui && sap.ui.getCore && sap.ui.getCore().isInitialized && sap.ui.getCore().isInitialized());
                return selectorOk && ui5Ok;
            })(arguments[0]);
        "#;
        let ret = driver.execute(js, vec![json!(ready_selector)]).await;
        match ret {
            Ok(v) => {
                let ok: bool = v.convert().unwrap_or(false);
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

pub async fn click(driver: &WebDriver, selector: &str) -> Result<()> {
    let el = driver
        .query(By::Css(selector))
        .first()
        .await
        .with_context(|| format!("No se encontró selector para click: {selector}"))?;
    el.click().await.with_context(|| format!("Falló click sobre selector: {selector}"))?;
    Ok(())
}

pub async fn input(driver: &WebDriver, selector: &str, value: &str, clear: bool) -> Result<()> {
    let el = driver
        .query(By::Css(selector))
        .first()
        .await
        .with_context(|| format!("No se encontró selector para input: {selector}"))?;
    if clear {
        let _ = el.clear().await;
    }
    el.send_keys(value).await.with_context(|| format!("Falló input sobre selector: {selector}"))?;
    Ok(())
}

pub async fn press(driver: &WebDriver, key: &str) -> Result<()> {
    let keys = match key.to_ascii_lowercase().as_str() {
        "enter" => Key::Enter,
        "tab" => Key::Tab,
        "escape" | "esc" => Key::Escape,
        "space" => Key::Space,
        other => bail!("Tecla no soportada todavía: {other}. Usa Enter, Tab, Escape o Space."),
    };
    driver.action_chain().send_keys(keys).perform().await.context("Falló envío de tecla")?;
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
