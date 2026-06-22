use crate::browser;
use crate::config::AppConfig;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Workflow declarativo orientado a operación real.
///
/// Se conserva compatibilidad con workflows simples generados por versiones anteriores:
/// `action: goto`, `action: click`, `action: input`, `action: snapshot`, etc.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Workflow {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub variables: BTreeMap<String, Value>,
    #[serde(default)]
    pub defaults: WorkflowDefaults,
    #[serde(default)]
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowDefaults {
    pub timeout_secs: Option<u64>,
    pub retry: Option<RetryPolicy>,
    pub capture_before_each_step: Option<bool>,
    pub capture_after_each_step: Option<bool>,
    pub stop_on_warning: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RetryPolicy {
    pub attempts: Option<u32>,
    pub delay_ms: Option<u64>,
}

/// Paso flexible para automatización productiva.
///
/// Campos admitidos por acción:
/// - goto: url
/// - wait_ui5: timeout_secs
/// - wait_for: selector | control_id | text, visible, timeout_secs
/// - click: selector | control_id | text, assert
/// - input: selector | control_id, value, clear, assert
/// - select: selector | control_id, value, assert
/// - press: key
/// - assert: assert
/// - snapshot: save_as
/// - screenshot: save_as
/// - wait: secs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowStep {
    pub action: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub selector: Option<String>,
    pub control_id: Option<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    pub value: Option<String>,
    pub key: Option<String>,
    pub save_as: Option<String>,
    pub secs: Option<u64>,
    pub timeout_secs: Option<u64>,
    pub clear: Option<bool>,
    pub visible: Option<bool>,
    pub optional: Option<bool>,
    pub retry: Option<RetryPolicy>,
    pub assert: Option<StepAssertion>,
    #[serde(default)]
    pub variables: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepAssertion {
    pub exists: Option<bool>,
    pub visible: Option<bool>,
    pub text_contains: Option<String>,
    pub url_contains: Option<String>,
    pub ui5_ready: Option<bool>,
    pub selector: Option<String>,
    pub control_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionReport {
    pub workflow_name: String,
    pub workflow_version: Option<String>,
    pub environment: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub duration_ms: u128,
    pub status: ExecutionStatus,
    pub steps: Vec<StepExecutionReport>,
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Passed,
    Failed,
    PassedWithWarnings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionReport {
    pub index: usize,
    pub name: String,
    pub action: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub duration_ms: u128,
    pub attempts: u32,
    pub status: StepStatus,
    pub target: StepTargetSummary,
    pub evidence: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Passed,
    Failed,
    SkippedOptional,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepTargetSummary {
    pub selector: Option<String>,
    pub control_id: Option<String>,
    pub text: Option<String>,
}

struct RunContext {
    variables: BTreeMap<String, Value>,
}

impl RunContext {
    fn new(workflow: &Workflow) -> Self {
        Self {
            variables: workflow.variables.clone(),
        }
    }

    fn with_step(&self, step: &WorkflowStep) -> Self {
        let mut variables = self.variables.clone();
        for (k, v) in &step.variables {
            variables.insert(k.clone(), v.clone());
        }
        Self { variables }
    }

    fn render(&self, value: &str) -> Result<String> {
        render_template(value, &self.variables)
    }
}

pub async fn run_workflow_file(
    cfg: &AppConfig,
    workflow_path: &Path,
    output_dir: &Path,
) -> Result<()> {
    let raw = tokio::fs::read_to_string(workflow_path)
        .await
        .with_context(|| format!("No se pudo leer workflow: {}", workflow_path.display()))?;
    let workflow: Workflow = serde_yaml::from_str(&raw)
        .with_context(|| format!("Workflow YAML no válido: {}", workflow_path.display()))?;
    run_workflow(cfg, &workflow, output_dir).await
}

pub async fn run_workflow(cfg: &AppConfig, workflow: &Workflow, output_dir: &Path) -> Result<()> {
    validate_workflow(workflow)?;
    tokio::fs::create_dir_all(output_dir).await?;

    let started_at = Utc::now();
    let started = Instant::now();
    let mut page = browser::connect_browser(cfg).await?;
    let base_ctx = RunContext::new(workflow);
    let mut reports = Vec::new();
    let mut failed = false;
    let mut warnings = false;

    info!(
        workflow = workflow.name,
        steps = workflow.steps.len(),
        "Ejecutando workflow CDP productivo"
    );

    for (idx, step) in workflow.steps.iter().enumerate() {
        let report = run_step_with_policy(
            cfg,
            &mut page,
            workflow,
            &base_ctx,
            step,
            idx + 1,
            output_dir,
        )
        .await;
        match report {
            Ok(r) => {
                match &r.status {
                    StepStatus::Failed => failed = true,
                    StepStatus::SkippedOptional => warnings = true,
                    StepStatus::Passed => {}
                }
                reports.push(r);
            }
            Err(e) => {
                failed = true;
                let now = Utc::now();
                reports.push(StepExecutionReport {
                    index: idx + 1,
                    name: step
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("Paso {}", idx + 1)),
                    action: step.action.clone(),
                    started_at: now,
                    finished_at: now,
                    duration_ms: 0,
                    attempts: 1,
                    status: StepStatus::Failed,
                    target: target_summary(step),
                    evidence: Vec::new(),
                    error: Some(e.to_string()),
                });
                break;
            }
        }

        if failed {
            break;
        }
    }

    let status = if failed {
        ExecutionStatus::Failed
    } else if warnings {
        ExecutionStatus::PassedWithWarnings
    } else {
        ExecutionStatus::Passed
    };

    let report = WorkflowExecutionReport {
        workflow_name: workflow.name.clone(),
        workflow_version: workflow.version.clone(),
        environment: workflow.environment.clone(),
        started_at,
        finished_at: Utc::now(),
        duration_ms: started.elapsed().as_millis(),
        status,
        steps: reports,
        output_dir: output_dir.display().to_string(),
    };

    let report_path = output_dir.join("execution_report.json");
    tokio::fs::write(&report_path, serde_json::to_vec_pretty(&report)?).await?;

    if matches!(&report.status, ExecutionStatus::Failed) {
        bail!("Workflow falló. Informe: {}", report_path.display());
    }

    println!("Workflow finalizado. Informe: {}", report_path.display());
    Ok(())
}

async fn run_step_with_policy(
    cfg: &AppConfig,
    page: &mut browser::CdpPage,
    workflow: &Workflow,
    base_ctx: &RunContext,
    step: &WorkflowStep,
    index: usize,
    output_dir: &Path,
) -> Result<StepExecutionReport> {
    let started_at = Utc::now();
    let started = Instant::now();
    let step_name = step
        .name
        .clone()
        .unwrap_or_else(|| format!("{} #{index}", step.action));
    let ctx = base_ctx.with_step(step);
    let retry = effective_retry(workflow, step);
    let attempts = retry
        .attempts
        .unwrap_or(cfg.automation.default_retry_attempts)
        .max(1);
    let delay_ms = retry.delay_ms.unwrap_or(cfg.automation.retry_delay_ms);
    let optional = step.optional.unwrap_or(false);
    let mut evidence = Vec::new();
    let mut last_error: Option<anyhow::Error> = None;

    info!(
        step = index,
        action = step.action,
        name = step_name,
        attempts,
        "Ejecutando paso"
    );

    if workflow.defaults.capture_before_each_step.unwrap_or(false) {
        let name = format!("evidence/{index:03}_before.json");
        evidence.push(name.clone());
        let snapshot = browser::extract_snapshot(page, cfg).await?;
        browser::write_snapshot(&output_dir.join(&name), &snapshot, cfg.output.pretty_json).await?;
    }

    for attempt in 1..=attempts {
        let result = execute_step_once(
            cfg,
            page,
            workflow,
            &ctx,
            step,
            index,
            output_dir,
            &mut evidence,
        )
        .await;
        match result {
            Ok(()) => {
                if workflow.defaults.capture_after_each_step.unwrap_or(false) {
                    let name = format!("evidence/{index:03}_after.json");
                    evidence.push(name.clone());
                    let snapshot = browser::extract_snapshot(page, cfg).await?;
                    browser::write_snapshot(
                        &output_dir.join(&name),
                        &snapshot,
                        cfg.output.pretty_json,
                    )
                    .await?;
                }

                return Ok(StepExecutionReport {
                    index,
                    name: step_name,
                    action: step.action.clone(),
                    started_at,
                    finished_at: Utc::now(),
                    duration_ms: started.elapsed().as_millis(),
                    attempts: attempt,
                    status: StepStatus::Passed,
                    target: target_summary(step),
                    evidence,
                    error: None,
                });
            }
            Err(e) => {
                warn!(step = index, attempt, error = ?e, "Intento fallido");
                last_error = Some(e);
                if attempt < attempts {
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    let error_message = last_error
        .map(|e| e.to_string())
        .unwrap_or_else(|| "Error desconocido".to_string());

    if cfg.automation.capture_snapshots_on_error {
        let safe_name = format!("evidence/{index:03}_error.json");
        if let Ok(snapshot) = browser::extract_snapshot(page, cfg).await {
            let _ = browser::write_snapshot(
                &output_dir.join(&safe_name),
                &snapshot,
                cfg.output.pretty_json,
            )
            .await;
            evidence.push(safe_name);
        }
        let png_name = format!("evidence/{index:03}_error.png");
        if let Ok(bytes) = browser::capture_screenshot_png(page).await {
            let _ = write_bytes_safe(output_dir, &png_name, &bytes).await;
            evidence.push(png_name);
        }
    }

    if optional {
        warn!(step = index, error = error_message, "Paso opcional omitido");
        Ok(StepExecutionReport {
            index,
            name: step_name,
            action: step.action.clone(),
            started_at,
            finished_at: Utc::now(),
            duration_ms: started.elapsed().as_millis(),
            attempts,
            status: StepStatus::SkippedOptional,
            target: target_summary(step),
            evidence,
            error: Some(error_message),
        })
    } else {
        error!(step = index, error = error_message, "Paso fallido");
        Ok(StepExecutionReport {
            index,
            name: step_name,
            action: step.action.clone(),
            started_at,
            finished_at: Utc::now(),
            duration_ms: started.elapsed().as_millis(),
            attempts,
            status: StepStatus::Failed,
            target: target_summary(step),
            evidence,
            error: Some(error_message),
        })
    }
}

async fn execute_step_once(
    cfg: &AppConfig,
    page: &mut browser::CdpPage,
    workflow: &Workflow,
    ctx: &RunContext,
    step: &WorkflowStep,
    index: usize,
    output_dir: &Path,
    evidence: &mut Vec<String>,
) -> Result<()> {
    let action = step.action.to_ascii_lowercase();
    let timeout = effective_timeout(cfg, workflow, step);
    match action.as_str() {
        "goto" => {
            let url = required_rendered(ctx, step.url.as_deref(), "url")?;
            page.goto(&url)
                .await
                .with_context(|| format!("No se pudo abrir URL: {url}"))?;
        }
        "wait_ui5" => {
            browser::wait_for_ui5(
                page,
                step.timeout_secs.unwrap_or(cfg.fiori.ui5_timeout_secs),
                cfg.fiori.ready_selector.as_deref(),
            )
            .await?;
        }
        "wait" => {
            sleep(Duration::from_secs(step.secs.unwrap_or(1))).await;
        }
        "wait_for" | "wait_selector" | "wait_control" => {
            let target = rendered_target(ctx, step)?;
            browser::wait_for_target(
                page,
                target.selector.as_deref(),
                target.control_id.as_deref(),
                target.text.as_deref(),
                timeout,
                step.visible.unwrap_or(true),
            )
            .await?;
        }
        "click" | "press_control" => {
            let target = rendered_target(ctx, step)?;
            browser::wait_for_target(
                page,
                target.selector.as_deref(),
                target.control_id.as_deref(),
                target.text.as_deref(),
                timeout,
                true,
            )
            .await?;
            browser::click_target(
                page,
                target.selector.as_deref(),
                target.control_id.as_deref(),
                target.text.as_deref(),
            )
            .await?;
            run_assertion_if_present(page, cfg, workflow, ctx, step, timeout).await?;
        }
        "input" | "set_value" => {
            let target = rendered_target(ctx, step)?;
            let value = required_rendered(ctx, step.value.as_deref(), "value")?;
            browser::wait_for_target(
                page,
                target.selector.as_deref(),
                target.control_id.as_deref(),
                target.text.as_deref(),
                timeout,
                true,
            )
            .await?;
            browser::input_target(
                page,
                target.selector.as_deref(),
                target.control_id.as_deref(),
                &value,
                step.clear.unwrap_or(true),
            )
            .await?;
            run_assertion_if_present(page, cfg, workflow, ctx, step, timeout).await?;
        }
        "select" | "select_key" => {
            let target = rendered_target(ctx, step)?;
            let value = required_rendered(ctx, step.value.as_deref(), "value")?;
            browser::wait_for_target(
                page,
                target.selector.as_deref(),
                target.control_id.as_deref(),
                target.text.as_deref(),
                timeout,
                true,
            )
            .await?;
            browser::select_target(
                page,
                target.selector.as_deref(),
                target.control_id.as_deref(),
                &value,
            )
            .await?;
            run_assertion_if_present(page, cfg, workflow, ctx, step, timeout).await?;
        }
        "press" => {
            let key = required_rendered(ctx, step.key.as_deref(), "key")?;
            browser::press(page, &key).await?;
        }
        "assert" | "verify" => {
            let assertion = step.assert.clone().unwrap_or_else(|| StepAssertion {
                selector: step.selector.clone(),
                control_id: step.control_id.clone(),
                text_contains: step.text.clone(),
                visible: step.visible,
                exists: Some(true),
                ..Default::default()
            });
            run_assertion(page, cfg, workflow, ctx, &assertion, timeout).await?;
        }
        "snapshot" => {
            let save_as = required_rendered(ctx, step.save_as.as_deref(), "save_as")?;
            let snap = browser::extract_snapshot(page, cfg).await?;
            let path = safe_output_path(output_dir, &save_as)?;
            browser::write_snapshot(&path, &snap, cfg.output.pretty_json).await?;
            evidence.push(save_as);
        }
        "screenshot" => {
            let save_as = required_rendered(ctx, step.save_as.as_deref(), "save_as")?;
            let bytes = browser::capture_screenshot_png(page).await?;
            write_bytes_safe(output_dir, &save_as, &bytes).await?;
            evidence.push(save_as);
        }
        other => bail!("Acción no soportada en paso {index}: {other}"),
    }
    Ok(())
}

async fn run_assertion_if_present(
    page: &mut browser::CdpPage,
    cfg: &AppConfig,
    workflow: &Workflow,
    ctx: &RunContext,
    step: &WorkflowStep,
    timeout: u64,
) -> Result<()> {
    if let Some(assertion) = &step.assert {
        run_assertion(page, cfg, workflow, ctx, assertion, timeout).await?;
    }
    Ok(())
}

async fn run_assertion(
    page: &mut browser::CdpPage,
    _cfg: &AppConfig,
    _workflow: &Workflow,
    ctx: &RunContext,
    assertion: &StepAssertion,
    timeout: u64,
) -> Result<()> {
    let selector = render_opt(ctx, assertion.selector.as_deref())?;
    let control_id = render_opt(ctx, assertion.control_id.as_deref())?;
    let text_contains = render_opt(ctx, assertion.text_contains.as_deref())?;
    let url_contains = render_opt(ctx, assertion.url_contains.as_deref())?;

    browser::assert_condition(
        page,
        selector.as_deref(),
        control_id.as_deref(),
        text_contains.as_deref(),
        url_contains.as_deref(),
        assertion.exists,
        assertion.visible,
        assertion.ui5_ready,
        timeout,
    )
    .await
}

fn validate_workflow(workflow: &Workflow) -> Result<()> {
    if workflow.name.trim().is_empty() {
        bail!("El workflow necesita un campo name no vacío.");
    }
    if workflow.steps.is_empty() {
        bail!("El workflow no contiene pasos.");
    }
    Ok(())
}

fn rendered_target(ctx: &RunContext, step: &WorkflowStep) -> Result<StepTargetSummary> {
    let target = StepTargetSummary {
        selector: render_opt(ctx, step.selector.as_deref())?,
        control_id: render_opt(ctx, step.control_id.as_deref())?,
        text: render_opt(ctx, step.text.as_deref())?,
    };
    if target.selector.is_none() && target.control_id.is_none() && target.text.is_none() {
        bail!(
            "El paso '{}' necesita selector, control_id o text.",
            step.action
        );
    }
    Ok(target)
}

fn target_summary(step: &WorkflowStep) -> StepTargetSummary {
    StepTargetSummary {
        selector: step.selector.clone(),
        control_id: step.control_id.clone(),
        text: step.text.clone(),
    }
}

fn effective_timeout(cfg: &AppConfig, workflow: &Workflow, step: &WorkflowStep) -> u64 {
    step.timeout_secs
        .or(workflow.defaults.timeout_secs)
        .unwrap_or(cfg.automation.default_timeout_secs)
}

fn effective_retry(workflow: &Workflow, step: &WorkflowStep) -> RetryPolicy {
    step.retry
        .clone()
        .or_else(|| workflow.defaults.retry.clone())
        .unwrap_or_default()
}

fn required_rendered(ctx: &RunContext, value: Option<&str>, field: &str) -> Result<String> {
    let raw = value.ok_or_else(|| anyhow::anyhow!("Falta campo requerido: {field}"))?;
    ctx.render(raw)
}

fn render_opt(ctx: &RunContext, value: Option<&str>) -> Result<Option<String>> {
    value.map(|v| ctx.render(v)).transpose()
}

fn render_template(input: &str, vars: &BTreeMap<String, Value>) -> Result<String> {
    let re = Regex::new(r"\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}")?;
    let mut out = String::new();
    let mut last = 0;
    for cap in re.captures_iter(input) {
        let m = cap.get(0).expect("match");
        let key = cap.get(1).expect("key").as_str();
        out.push_str(&input[last..m.start()]);
        out.push_str(&lookup_template_value(key, vars)?);
        last = m.end();
    }
    out.push_str(&input[last..]);
    Ok(out)
}

fn lookup_template_value(key: &str, vars: &BTreeMap<String, Value>) -> Result<String> {
    if let Some(env_key) = key.strip_prefix("env.") {
        return std::env::var(env_key)
            .with_context(|| format!("Variable de entorno no definida: {env_key}"));
    }
    let key = key.strip_prefix("vars.").unwrap_or(key);
    let value = get_dotted(vars, key)
        .ok_or_else(|| anyhow::anyhow!("Variable de workflow no definida: {key}"))?;
    Ok(match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    })
}

fn get_dotted<'a>(vars: &'a BTreeMap<String, Value>, path: &str) -> Option<&'a Value> {
    let mut parts = path.split('.');
    let first = parts.next()?;
    let mut cur = vars.get(first)?;
    for part in parts {
        cur = cur.get(part)?;
    }
    Some(cur)
}

fn safe_output_path(output_dir: &Path, path: &str) -> Result<PathBuf> {
    let p = Path::new(path);
    if p.is_absolute() || path.contains("..") {
        bail!("La ruta de salida debe ser relativa y segura dentro de output_dir: {path}");
    }
    Ok(output_dir.join(p))
}

async fn write_bytes_safe(output_dir: &Path, rel_path: &str, bytes: &[u8]) -> Result<()> {
    let path = safe_output_path(output_dir, rel_path)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, bytes)
        .await
        .with_context(|| format!("No se pudo escribir evidencia: {}", path.display()))
}
