use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub browser: BrowserConfig,
    pub fiori: FioriConfig,
    pub extraction: ExtractionConfig,
    pub output: OutputConfig,
}

/// Configuración del navegador vivo.
///
/// Esta versión NO usa ChromeDriver ni GeckoDriver. La captura viva se realiza mediante
/// Chrome DevTools Protocol (CDP), conectando con Chrome/Chromium en un puerto local.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Endpoint HTTP de CDP, normalmente http://127.0.0.1:9222.
    pub cdp_url: String,
    /// Binario de Chrome/Chromium que se lanzará si `auto_launch = true`.
    pub chrome_binary: String,
    /// Si es true, la app intenta lanzar Chrome/Chromium automáticamente si CDP no está activo.
    pub auto_launch: bool,
    /// Abre Chrome sin interfaz gráfica. Para análisis Fiori con SSO/login manual se recomienda false.
    pub headless: bool,
    /// Ignora errores de certificados en entornos corporativos/desarrollo.
    pub accept_insecure_certs: bool,
    pub window_width: u32,
    pub window_height: u32,
    /// Perfil aislado para no mezclar cookies/sesiones personales.
    pub user_data_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FioriConfig {
    pub wait_for_ui5: bool,
    pub ui5_timeout_secs: u64,
    pub manual_login_wait_secs: u64,
    pub ready_selector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    pub max_text_len: usize,
    pub include_hidden_controls: bool,
    pub include_dom_nodes: bool,
    pub include_performance_urls: bool,
    pub max_controls: usize,
    pub max_dom_nodes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub pretty_json: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            browser: BrowserConfig::default(),
            fiori: FioriConfig::default(),
            extraction: ExtractionConfig::default(),
            output: OutputConfig::default(),
        }
    }
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            cdp_url: "http://127.0.0.1:9222".to_string(),
            chrome_binary: "google-chrome".to_string(),
            auto_launch: true,
            headless: false,
            accept_insecure_certs: true,
            window_width: 1600,
            window_height: 1000,
            user_data_dir: Some("./.browser-profile-cdp".to_string()),
        }
    }
}

impl Default for FioriConfig {
    fn default() -> Self {
        Self {
            wait_for_ui5: true,
            ui5_timeout_secs: 90,
            manual_login_wait_secs: 0,
            ready_selector: Some("body".to_string()),
        }
    }
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            max_text_len: 240,
            include_hidden_controls: false,
            include_dom_nodes: true,
            include_performance_urls: true,
            max_controls: 5000,
            max_dom_nodes: 3000,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self { pretty_json: true }
    }
}

impl AppConfig {
    pub async fn from_file_or_default(path: Option<&Path>) -> Result<Self> {
        match path {
            Some(path) => {
                let raw = tokio::fs::read_to_string(path).await.with_context(|| {
                    format!(
                        "No se pudo leer el archivo de configuración: {}",
                        path.display()
                    )
                })?;
                let cfg: AppConfig = toml::from_str(&raw).with_context(|| {
                    format!("El TOML de configuración no es válido: {}", path.display())
                })?;
                Ok(cfg)
            }
            None => Ok(AppConfig::default()),
        }
    }
}
