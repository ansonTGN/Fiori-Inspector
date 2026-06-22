use crate::browser;
use crate::config::AppConfig;
use crate::models::{
    ActionHint, BindingInfo, InteractorKind, ODataEndpoint, PageSnapshot, Ui5Control,
};
use crate::static_html;
use anyhow::{Context, Result};
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
struct StudioState {
    cfg: AppConfig,
    snapshots: Arc<RwLock<BTreeMap<String, SnapshotRecord>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub snapshot: PageSnapshot,
    pub report: StudioReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub url: Option<String>,
    pub title: Option<String>,
    pub mode: String,
    pub control_count: usize,
    pub actionable_count: usize,
    pub endpoint_count: usize,
    pub quality_score: u8,
    pub ui5_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeHtmlRequest {
    pub html: String,
    pub url: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSnapshotRequest {
    pub url: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioReport {
    pub quality_score: u8,
    pub maturity_level: String,
    pub executive_summary: String,
    pub detected_patterns: Vec<String>,
    pub top_actions: Vec<ActionHint>,
    pub high_value_controls: Vec<ControlCard>,
    pub binding_inventory: Vec<BindingCard>,
    pub endpoint_inventory: Vec<ODataEndpoint>,
    pub automation_risks: Vec<RiskCard>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlCard {
    pub id: String,
    pub label: String,
    pub control_type: String,
    pub selector: Option<String>,
    pub visible: Option<bool>,
    pub interactor: Option<InteractorKind>,
    pub confidence: f32,
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingCard {
    pub control_id: String,
    pub control_label: String,
    pub control_type: String,
    pub bindings: Vec<BindingInfo>,
    pub context_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskCard {
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExport {
    pub yaml: String,
}

pub async fn serve(cfg: AppConfig, bind: &str, static_dir: PathBuf) -> Result<()> {
    let state = StudioState {
        cfg,
        snapshots: Arc::new(RwLock::new(BTreeMap::new())),
    };

    let app = axum::Router::new()
        .route("/api/health", get(health))
        .route("/api/snapshots", get(list_snapshots))
        .route("/api/snapshots/static", post(analyze_static_html))
        .route("/api/snapshots/browser", post(snapshot_browser))
        .route("/api/snapshots/:id", get(get_snapshot))
        .route("/api/snapshots/:id/report", get(get_report))
        .route("/api/snapshots/:id/workflow", get(generate_workflow))
        .nest_service(
            "/",
            ServeDir::new(static_dir).append_index_html_on_directories(true),
        )
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = bind
        .parse()
        .with_context(|| format!("Bind address no válido: {bind}"))?;
    let listener = TcpListener::bind(addr).await?;
    info!("Fiori Inspector Studio escuchando en http://{addr}");
    println!("\nFiori Inspector Studio");
    println!("──────────────────────");
    println!("Abre: http://{addr}");
    println!("No se usa ChromeDriver: la captura viva usa Chrome DevTools Protocol. Si CDP no está activo, se intentará lanzar Chrome automáticamente.\n");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "name": "fiori-inspector-studio",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn list_snapshots(State(state): State<StudioState>) -> Json<Vec<SnapshotSummary>> {
    let map = state.snapshots.read().await;
    let mut out = map.values().map(summary_from_record).collect::<Vec<_>>();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(out)
}

async fn analyze_static_html(
    State(state): State<StudioState>,
    Json(req): Json<AnalyzeHtmlRequest>,
) -> std::result::Result<Json<SnapshotRecord>, ApiError> {
    if req.html.trim().is_empty() {
        return Err(ApiError::bad_request("El HTML está vacío."));
    }
    let snapshot = static_html::analyze_html(
        &req.html,
        req.url.clone(),
        state.cfg.extraction.max_text_len,
        state.cfg.extraction.max_dom_nodes,
    );
    let record = make_record(
        snapshot,
        req.name.or_else(|| Some("HTML estático".to_string())),
    );
    state
        .snapshots
        .write()
        .await
        .insert(record.id.clone(), record.clone());
    Ok(Json(record))
}

async fn snapshot_browser(
    State(state): State<StudioState>,
    Json(req): Json<BrowserSnapshotRequest>,
) -> std::result::Result<Json<SnapshotRecord>, ApiError> {
    if !(req.url.starts_with("http://") || req.url.starts_with("https://")) {
        return Err(ApiError::bad_request(
            "La URL debe empezar por http:// o https://.",
        ));
    }
    let snapshot = browser::snapshot_browser(&state.cfg, &req.url, None)
        .await
        .map_err(ApiError::internal)?;
    let record = make_record(snapshot, req.name.or_else(|| Some(req.url.clone())));
    state
        .snapshots
        .write()
        .await
        .insert(record.id.clone(), record.clone());
    Ok(Json(record))
}

async fn get_snapshot(
    State(state): State<StudioState>,
    AxumPath(id): AxumPath<String>,
) -> std::result::Result<Json<SnapshotRecord>, ApiError> {
    let map = state.snapshots.read().await;
    let record = map
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::not_found("Snapshot no encontrado."))?;
    Ok(Json(record))
}

async fn get_report(
    State(state): State<StudioState>,
    AxumPath(id): AxumPath<String>,
) -> std::result::Result<Json<StudioReport>, ApiError> {
    let map = state.snapshots.read().await;
    let record = map
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::not_found("Snapshot no encontrado."))?;
    Ok(Json(record.report))
}

async fn generate_workflow(
    State(state): State<StudioState>,
    AxumPath(id): AxumPath<String>,
) -> std::result::Result<Json<WorkflowExport>, ApiError> {
    let map = state.snapshots.read().await;
    let record = map
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::not_found("Snapshot no encontrado."))?;
    Ok(Json(WorkflowExport {
        yaml: workflow_from_snapshot(&record.snapshot),
    }))
}

fn make_record(snapshot: PageSnapshot, name: Option<String>) -> SnapshotRecord {
    let report = build_report(&snapshot);
    SnapshotRecord {
        id: Uuid::new_v4().to_string(),
        name: name.unwrap_or_else(|| {
            snapshot
                .title
                .clone()
                .unwrap_or_else(|| "Snapshot Fiori".to_string())
        }),
        created_at: Utc::now(),
        snapshot,
        report,
    }
}

fn summary_from_record(record: &SnapshotRecord) -> SnapshotSummary {
    SnapshotSummary {
        id: record.id.clone(),
        name: record.name.clone(),
        created_at: record.created_at,
        url: record.snapshot.url.clone(),
        title: record.snapshot.title.clone(),
        mode: format!("{:?}", record.snapshot.mode),
        control_count: record.snapshot.metrics.control_count,
        actionable_count: record.snapshot.metrics.actionable_control_count,
        endpoint_count: record.snapshot.metrics.endpoint_count,
        quality_score: record.report.quality_score,
        ui5_detected: record.snapshot.ui5.detected,
    }
}

pub fn build_report(snapshot: &PageSnapshot) -> StudioReport {
    let top_actions = snapshot
        .action_hints
        .iter()
        .filter(|a| a.confidence >= 0.55)
        .take(60)
        .cloned()
        .collect::<Vec<_>>();

    let high_value_controls = snapshot
        .controls
        .iter()
        .filter_map(control_card)
        .take(120)
        .collect::<Vec<_>>();
    let binding_inventory = snapshot
        .controls
        .iter()
        .filter_map(binding_card)
        .take(100)
        .collect::<Vec<_>>();
    let automation_risks = risks(snapshot);
    let detected_patterns = detected_patterns(snapshot);
    let recommendations = recommendations(snapshot, &automation_risks);
    let quality_score = quality_score(snapshot, &automation_risks);
    let maturity_level = match quality_score {
        85..=100 => "Alta: buen candidato para automatización robusta".to_string(),
        65..=84 => "Media: automatizable con controles de estabilidad".to_string(),
        40..=64 => "Baja-media: requiere saneamiento de selectores".to_string(),
        _ => "Baja: conviene priorizar OData/API o stable IDs".to_string(),
    };
    let executive_summary = format!(
        "Se han identificado {} controles, {} acciones candidatas y {} endpoints OData. UI5 detectado: {}. Nivel de madurez: {}.",
        snapshot.metrics.control_count,
        snapshot.metrics.actionable_control_count,
        snapshot.metrics.endpoint_count,
        if snapshot.ui5.detected { "sí" } else { "no" },
        maturity_level
    );

    StudioReport {
        quality_score,
        maturity_level,
        executive_summary,
        detected_patterns,
        top_actions,
        high_value_controls,
        binding_inventory,
        endpoint_inventory: snapshot.odata_endpoints.clone(),
        automation_risks,
        recommendations,
    }
}

fn control_card(c: &Ui5Control) -> Option<ControlCard> {
    let actionable = c.interactor.is_some() || !c.selector_candidates.is_empty();
    let bound = !c.bindings.is_empty() || c.binding_context_path.is_some();
    let visible = c.visible.unwrap_or(true);
    if !(actionable || bound || visible) {
        return None;
    }
    Some(ControlCard {
        id: c.id.clone(),
        label: c
            .text
            .clone()
            .or_else(|| c.title.clone())
            .or_else(|| c.value.clone())
            .unwrap_or_default(),
        control_type: c
            .control_type
            .clone()
            .or_else(|| c.short_type.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        selector: c.selector_candidates.first().cloned(),
        visible: c.visible,
        interactor: c.interactor.clone(),
        confidence: c.confidence,
        risk_flags: c.risk_flags.clone(),
    })
}

fn binding_card(c: &Ui5Control) -> Option<BindingCard> {
    if c.bindings.is_empty() && c.binding_context_path.is_none() {
        return None;
    }
    Some(BindingCard {
        control_id: c.id.clone(),
        control_label: c
            .text
            .clone()
            .or_else(|| c.title.clone())
            .or_else(|| c.value.clone())
            .unwrap_or_default(),
        control_type: c
            .control_type
            .clone()
            .or_else(|| c.short_type.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        bindings: c.bindings.clone(),
        context_path: c.binding_context_path.clone(),
    })
}

fn detected_patterns(snapshot: &PageSnapshot) -> Vec<String> {
    let mut patterns = Vec::new();
    let types = snapshot
        .controls
        .iter()
        .filter_map(|c| c.control_type.as_deref())
        .collect::<BTreeSet<_>>();
    if types
        .iter()
        .any(|t| t.contains("Table") || t.contains("List"))
    {
        patterns.push("Pantalla con tablas/listas: conviene automatizar por contexto de fila y binding, no por índice visual.".to_string());
    }
    if types.iter().any(|t| t.contains("Smart")) {
        patterns.push("Uso probable de Smart Controls: la semántica puede estar en metadatos OData y anotaciones.".to_string());
    }
    if !snapshot.odata_endpoints.is_empty() {
        patterns.push("Se han detectado endpoints OData; muchas acciones UI pueden transformarse en llamadas API más estables.".to_string());
    }
    if snapshot.ui5.detected {
        patterns
            .push("SAPUI5 detectado: priorizar árbol lógico UI5 frente a HTML plano.".to_string());
    } else {
        patterns.push(
            "No se ha confirmado SAPUI5; el análisis puede estar basado solo en DOM estático."
                .to_string(),
        );
    }
    patterns
}

fn risks(snapshot: &PageSnapshot) -> Vec<RiskCard> {
    let mut risks = Vec::new();
    if !snapshot.ui5.detected {
        risks.push(RiskCard {
            severity: "alta".to_string(),
            title: "UI5 no confirmado".to_string(),
            detail: "El análisis puede no incluir árbol de controles, bindings ni modelos vivos.".to_string(),
            remediation: "Ejecutar captura con CDP en una sesión Fiori real y esperar a sap.ui.getCore().isInitialized().".to_string(),
        });
    }
    let dynamic_ids = snapshot
        .controls
        .iter()
        .filter(|c| c.id.contains("__") || c.id.len() > 90)
        .count();
    if dynamic_ids > 0 {
        risks.push(RiskCard {
            severity: "media".to_string(),
            title: "IDs potencialmente dinámicos".to_string(),
            detail: format!(
                "{} controles parecen tener IDs generados o excesivamente largos.",
                dynamic_ids
            ),
            remediation:
                "Preferir stable IDs, sufijos semánticos, aria-labels, bindings o llamadas OData."
                    .to_string(),
        });
    }
    if snapshot.metrics.actionable_control_count == 0 {
        risks.push(RiskCard {
            severity: "media".to_string(),
            title: "No hay acciones candidatas".to_string(),
            detail: "No se detectaron botones, inputs, links o selectores de acción claros.".to_string(),
            remediation: "Revisar si la captura se hizo antes del login, dentro de un iframe, o sobre una pantalla incompleta.".to_string(),
        });
    }
    if snapshot.odata_endpoints.is_empty() {
        risks.push(RiskCard {
            severity: "baja".to_string(),
            title: "Sin endpoints OData detectados".to_string(),
            detail: "No se han observado servicios en performance entries, modelos o HTML.".to_string(),
            remediation: "Capturar tras ejecutar una búsqueda o navegación que fuerce lectura/escritura de datos.".to_string(),
        });
    }
    for warning in &snapshot.warnings {
        risks.push(RiskCard {
            severity: "info".to_string(),
            title: "Aviso del extractor".to_string(),
            detail: warning.clone(),
            remediation: "Revisar el contexto de captura y repetir con una sesión completamente inicializada si procede.".to_string(),
        });
    }
    risks
}

fn recommendations(snapshot: &PageSnapshot, risks: &[RiskCard]) -> Vec<String> {
    let mut out = Vec::new();
    out.push("Construir automatizaciones declarativas YAML: goto → wait_ui5 → acción → snapshot → verificación.".to_string());
    out.push("Usar selectores por stable ID o sufijo semántico; evitar coordenadas y posiciones absolutas.".to_string());
    if !snapshot.odata_endpoints.is_empty() {
        out.push("Mapear acciones críticas a OData/REST cuando sea posible; usar UI solo para discovery o validación funcional.".to_string());
    }
    if !snapshot.controls.iter().any(|c| !c.bindings.is_empty()) {
        out.push("Repetir la captura con sesión viva para obtener bindings; el HTML estático raramente contiene todo el modelo UI5.".to_string());
    }
    if risks.iter().any(|r| r.severity == "alta") {
        out.push(
            "No pasar a automatización productiva hasta resolver los riesgos de severidad alta."
                .to_string(),
        );
    }
    out
}

fn quality_score(snapshot: &PageSnapshot, risks: &[RiskCard]) -> u8 {
    let mut score: i32 = 50;
    if snapshot.ui5.detected {
        score += 15;
    }
    if snapshot.metrics.control_count > 0 {
        score += 10;
    }
    if snapshot.metrics.actionable_control_count > 0 {
        score += 10;
    }
    if snapshot.metrics.endpoint_count > 0 {
        score += 8;
    }
    if snapshot
        .controls
        .iter()
        .any(|c| !c.bindings.is_empty() || c.binding_context_path.is_some())
    {
        score += 7;
    }
    for r in risks {
        match r.severity.as_str() {
            "alta" => score -= 20,
            "media" => score -= 10,
            "baja" => score -= 4,
            _ => {}
        }
    }
    score.clamp(0, 100) as u8
}

fn workflow_from_snapshot(snapshot: &PageSnapshot) -> String {
    let mut out = String::new();
    out.push_str("name: \"Workflow Fiori generado desde Inspector Studio\"\n");
    out.push_str("description: \"Borrador generado automáticamente. Revisar selectores, validaciones y datos sensibles antes de uso productivo.\"\n\n");
    out.push_str("steps:\n");
    if let Some(url) = &snapshot.url {
        out.push_str(&format!("  - action: goto\n    url: {:?}\n", url));
        out.push_str("  - action: wait_ui5\n    timeout_secs: 90\n");
    }
    for action in snapshot.action_hints.iter().take(8) {
        match action.kind {
            InteractorKind::Input | InteractorKind::ComboBox => {
                out.push_str(&format!("  - action: input\n    selector: {:?}\n    value: \"<valor>\"\n    clear: true\n", action.selector));
            }
            InteractorKind::Button
            | InteractorKind::Link
            | InteractorKind::Tab
            | InteractorKind::MenuItem => {
                out.push_str(&format!(
                    "  - action: click\n    selector: {:?}\n",
                    action.selector
                ));
            }
            _ => {}
        }
    }
    out.push_str("  - action: snapshot\n    save_as: \"resultado_final.json\"\n");
    out
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn internal(error: anyhow::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}
