mod browser;
mod config;
mod models;
mod report;
mod static_html;
mod studio;
mod workflow;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::AppConfig;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "fiori-inspector-studio")]
#[command(version)]
#[command(about = "Estudio interactivo Rust para analizar SAP Fiori/SAPUI5 y preparar automatizaciones profesionales")]
#[command(long_about = "Fiori Inspector Studio es una aplicación local para analizar SAP Fiori/SAPUI5.

Uso recomendado para principiantes:
  cargo run

Eso abrirá la interfaz web en http://127.0.0.1:7820.

Para capturar una sesión Fiori viva necesitas ChromeDriver:
  chromedriver --port=9515

Después usa la interfaz web, que incluye una guía visual paso a paso.")]
struct Cli {
    #[arg(short, long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Abre la interfaz web local estilo Apple para análisis interactivo.
    Serve {
        #[arg(long, default_value = "127.0.0.1:7820")]
        bind: String,
        #[arg(long, value_name = "DIR", default_value = "static")]
        static_dir: PathBuf,
    },

    /// Analiza una app Fiori viva usando WebDriver y extrae DOM + controles UI5.
    SnapshotBrowser {
        #[arg(long)]
        url: String,
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Analiza un HTML guardado. Útil para evidencias, pero menos potente que browser-ui5.
    AnalyzeHtml {
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Ejecuta un workflow YAML de navegación, interacción y snapshots.
    RunWorkflow {
        #[arg(short, long, value_name = "FILE")]
        workflow: PathBuf,
        #[arg(short, long, value_name = "DIR", default_value = "runs/latest")]
        output_dir: PathBuf,
    },

    /// Imprime resumen de un snapshot JSON.
    Summary {
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,
    },

    /// Imprime árbol lógico de controles UI5.
    Tree {
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,
        #[arg(long, default_value_t = 6)]
        max_depth: usize,
    },

    /// Lista selectores recomendados para automatización.
    Actions {
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,
        #[arg(long)]
        contains: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();
    let cfg = AppConfig::from_file_or_default(cli.config.as_deref()).await?;

    // Experiencia por defecto: si la persona ejecuta solo `cargo run`,
    // abrimos directamente el estudio visual. Esto evita que usuarios no técnicos
    // se queden bloqueados ante la ayuda de terminal.
    let command = cli.command.unwrap_or_else(|| {
        println!("\nFiori Inspector Studio se va a abrir en modo interactivo.");
        println!("Abre el navegador en: http://127.0.0.1:7820");
        println!("Para salir, pulsa Ctrl+C en esta terminal.\n");
        Commands::Serve {
            bind: "127.0.0.1:7820".to_string(),
            static_dir: PathBuf::from("static"),
        }
    });

    match command {
        Commands::Serve { bind, static_dir } => {
            studio::serve(cfg, &bind, static_dir).await?;
        }
        Commands::SnapshotBrowser { url, output } => {
            let snapshot = browser::snapshot_browser(&cfg, &url, output.as_deref()).await?;
            report::print_summary(&snapshot);
        }
        Commands::AnalyzeHtml { input, output } => {
            let snapshot = static_html::analyze_html_file(&input, cfg.extraction.max_text_len, cfg.extraction.max_dom_nodes).await?;
            if let Some(path) = output {
                browser::write_snapshot(&path, &snapshot, cfg.output.pretty_json).await?;
            }
            report::print_summary(&snapshot);
        }
        Commands::RunWorkflow { workflow, output_dir } => {
            workflow::run_workflow_file(&cfg, &workflow, &output_dir).await?;
            println!("Workflow finalizado. Resultados en: {}", output_dir.display());
        }
        Commands::Summary { input } => {
            let snapshot = report::read_snapshot(&input).await?;
            report::print_summary(&snapshot);
        }
        Commands::Tree { input, max_depth } => {
            let snapshot = report::read_snapshot(&input).await?;
            report::print_tree(&snapshot, max_depth);
        }
        Commands::Actions { input, contains } => {
            let snapshot = report::read_snapshot(&input).await?;
            report::print_actions(&snapshot, contains.as_deref());
        }
    }

    Ok(())
}
