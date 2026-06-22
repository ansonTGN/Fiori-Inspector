use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub schema_version: String,
    pub captured_at: DateTime<Utc>,
    pub mode: SnapshotMode,
    pub url: Option<String>,
    pub title: Option<String>,
    pub application: Option<FioriApplicationInfo>,
    pub metrics: SnapshotMetrics,
    pub ui5: Ui5RuntimeInfo,
    pub controls: Vec<Ui5Control>,
    pub dom_nodes: Vec<DomNode>,
    pub odata_endpoints: Vec<ODataEndpoint>,
    pub action_hints: Vec<ActionHint>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotMode {
    BrowserUi5,
    StaticHtml,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnapshotMetrics {
    pub control_count: usize,
    pub dom_node_count: usize,
    pub endpoint_count: usize,
    pub visible_control_count: usize,
    pub actionable_control_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ui5RuntimeInfo {
    pub detected: bool,
    pub version: Option<String>,
    pub bootstrapped: Option<bool>,
    pub core_initialized: Option<bool>,
    pub libraries: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FioriApplicationInfo {
    pub hash: Option<String>,
    pub semantic_object: Option<String>,
    pub action: Option<String>,
    pub component_id: Option<String>,
    pub component_name: Option<String>,
    pub manifest_sap_app: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ui5Control {
    pub id: String,
    pub control_type: Option<String>,
    pub short_type: Option<String>,
    pub visible: Option<bool>,
    pub enabled: Option<bool>,
    pub editable: Option<bool>,
    pub selected: Option<bool>,
    pub busy: Option<bool>,
    pub text: Option<String>,
    pub title: Option<String>,
    pub value: Option<String>,
    pub tooltip: Option<String>,
    pub selected_key: Option<String>,
    pub binding_context_path: Option<String>,
    pub parent_id: Option<String>,
    pub child_ids: Vec<String>,
    pub aggregations: Vec<AggregationInfo>,
    pub bindings: Vec<BindingInfo>,
    pub models: Vec<ModelInfo>,
    pub dom: Option<DomRef>,
    pub selector_candidates: Vec<String>,
    pub interactor: Option<InteractorKind>,
    pub confidence: f32,
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregationInfo {
    pub name: String,
    pub multiple: bool,
    pub type_name: Option<String>,
    pub child_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BindingInfo {
    pub property: String,
    pub path: Option<String>,
    pub model: Option<String>,
    pub parts: Vec<BindingPart>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BindingPart {
    pub path: Option<String>,
    pub model: Option<String>,
    pub type_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: Option<String>,
    pub class_name: Option<String>,
    pub service_url: Option<String>,
    pub default_binding_mode: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomRef {
    pub dom_id: Option<String>,
    pub tag: Option<String>,
    pub role: Option<String>,
    pub aria_label: Option<String>,
    pub aria_described_by: Option<String>,
    pub classes: Vec<String>,
    pub rect: Option<Rect>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomNode {
    pub node_id: String,
    pub tag: String,
    pub id: Option<String>,
    pub role: Option<String>,
    pub text: Option<String>,
    pub aria_label: Option<String>,
    pub title: Option<String>,
    pub classes: Vec<String>,
    pub attributes: BTreeMap<String, String>,
    pub selector_candidates: Vec<String>,
    pub semantic: Option<String>,
    pub rect: Option<Rect>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ODataEndpoint {
    pub url: String,
    pub service_root: Option<String>,
    pub entity_or_path: Option<String>,
    pub source: EndpointSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointSource {
    PerformanceEntry,
    ModelServiceUrl,
    HtmlReference,
}

impl Default for EndpointSource {
    fn default() -> Self {
        Self::PerformanceEntry
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractorKind {
    Button,
    Link,
    Input,
    ComboBox,
    Checkbox,
    RadioButton,
    Table,
    Row,
    Tab,
    MenuItem,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHint {
    pub label: String,
    pub kind: InteractorKind,
    pub selector: String,
    pub control_id: Option<String>,
    pub confidence: f32,
    pub rationale: String,
}
