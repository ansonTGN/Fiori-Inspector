use crate::browser;
use crate::config::AppConfig;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum WorkflowStep {
    Goto {
        url: String,
    },
    WaitUi5 {
        timeout_secs: Option<u64>,
    },
    Wait {
        secs: u64,
    },
    Click {
        selector: String,
    },
    Input {
        selector: String,
        value: String,
        clear: Option<bool>,
    },
    Press {
        key: String,
    },
    Snapshot {
        save_as: String,
    },
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
    tokio::fs::create_dir_all(output_dir).await?;
    let mut page = browser::connect_browser(cfg).await?;
    run_steps(cfg, &mut page, workflow, output_dir).await
}

async fn run_steps(
    cfg: &AppConfig,
    page: &mut browser::CdpPage,
    workflow: &Workflow,
    output_dir: &Path,
) -> Result<()> {
    info!(
        workflow = workflow.name,
        steps = workflow.steps.len(),
        "Ejecutando workflow CDP sin ChromeDriver"
    );
    for (idx, step) in workflow.steps.iter().enumerate() {
        info!(step = idx + 1, action = ?step, "Ejecutando paso");
        match step {
            WorkflowStep::Goto { url } => {
                page.goto(url)
                    .await
                    .with_context(|| format!("No se pudo abrir URL: {url}"))?;
            }
            WorkflowStep::WaitUi5 { timeout_secs } => {
                browser::wait_for_ui5(
                    page,
                    timeout_secs.unwrap_or(cfg.fiori.ui5_timeout_secs),
                    cfg.fiori.ready_selector.as_deref(),
                )
                .await?;
            }
            WorkflowStep::Wait { secs } => sleep(Duration::from_secs(*secs)).await,
            WorkflowStep::Click { selector } => browser::click(page, selector).await?,
            WorkflowStep::Input {
                selector,
                value,
                clear,
            } => browser::input(page, selector, value, clear.unwrap_or(true)).await?,
            WorkflowStep::Press { key } => browser::press(page, key).await?,
            WorkflowStep::Snapshot { save_as } => {
                guard_relative(save_as)?;
                let snap = browser::extract_snapshot(page, cfg).await?;
                let path = output_dir.join(PathBuf::from(save_as));
                browser::write_snapshot(&path, &snap, cfg.output.pretty_json).await?;
            }
        }
    }
    Ok(())
}

fn guard_relative(path: &str) -> Result<()> {
    let p = Path::new(path);
    if p.is_absolute() || path.contains("..") {
        bail!("save_as debe ser una ruta relativa segura dentro de output_dir: {path}");
    }
    Ok(())
}
