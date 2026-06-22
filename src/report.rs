use crate::models::{InteractorKind, PageSnapshot, Ui5Control};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

pub async fn read_snapshot(path: &Path) -> Result<PageSnapshot> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("No se pudo leer snapshot: {}", path.display()))?;
    serde_json::from_str(&raw).context("Snapshot JSON no válido")
}

pub fn print_summary(snapshot: &PageSnapshot) {
    println!("URL: {}", snapshot.url.as_deref().unwrap_or("-"));
    println!("Título: {}", snapshot.title.as_deref().unwrap_or("-"));
    println!("Modo: {:?}", snapshot.mode);
    println!("UI5 detectado: {}", snapshot.ui5.detected);
    if let Some(version) = &snapshot.ui5.version {
        println!("UI5 versión: {version}");
    }
    println!("Controles: {}", snapshot.metrics.control_count);
    println!("Nodos DOM: {}", snapshot.metrics.dom_node_count);
    println!("Accionables: {}", snapshot.metrics.actionable_control_count);
    println!("Endpoints OData: {}", snapshot.metrics.endpoint_count);

    if !snapshot.warnings.is_empty() {
        println!("\nAvisos:");
        for w in &snapshot.warnings {
            println!("  - {w}");
        }
    }
}

pub fn print_tree(snapshot: &PageSnapshot, max_depth: usize) {
    let mut by_parent: BTreeMap<Option<String>, Vec<&Ui5Control>> = BTreeMap::new();
    for c in &snapshot.controls {
        by_parent.entry(c.parent_id.clone()).or_default().push(c);
    }

    let roots = by_parent.get(&None).cloned().unwrap_or_default();
    for root in roots {
        print_control(root, &by_parent, 0, max_depth);
    }
}

pub fn print_actions(snapshot: &PageSnapshot, filter: Option<&str>) {
    let filter_lc = filter.map(|s| s.to_ascii_lowercase());
    for h in &snapshot.action_hints {
        let haystack =
            format!("{} {} {:?}", h.label, h.selector, h.control_id).to_ascii_lowercase();
        if let Some(f) = &filter_lc {
            if !haystack.contains(f) {
                continue;
            }
        }
        println!("- [{}] {}", kind_label(&h.kind), h.label);
        println!("  selector: {}", h.selector);
        if let Some(id) = &h.control_id {
            println!("  control_id: {id}");
        }
        println!("  confidence: {:.2}", h.confidence);
    }
}

fn print_control(
    c: &Ui5Control,
    by_parent: &BTreeMap<Option<String>, Vec<&Ui5Control>>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth {
        return;
    }
    let indent = "  ".repeat(depth);
    let label = c
        .text
        .as_deref()
        .or(c.title.as_deref())
        .or(c.value.as_deref())
        .unwrap_or("");
    let ty = c.control_type.as_deref().unwrap_or("unknown");
    let vis = match c.visible {
        Some(true) => "visible",
        Some(false) => "hidden",
        None => "?",
    };
    println!("{indent}- {} | {} | {} | {}", c.id, ty, vis, label);
    if let Some(children) = by_parent.get(&Some(c.id.clone())) {
        for child in children {
            print_control(child, by_parent, depth + 1, max_depth);
        }
    }
}

fn kind_label(kind: &InteractorKind) -> &'static str {
    match kind {
        InteractorKind::Button => "button",
        InteractorKind::Link => "link",
        InteractorKind::Input => "input",
        InteractorKind::ComboBox => "combo",
        InteractorKind::Checkbox => "checkbox",
        InteractorKind::RadioButton => "radio",
        InteractorKind::Table => "table",
        InteractorKind::Row => "row",
        InteractorKind::Tab => "tab",
        InteractorKind::MenuItem => "menuitem",
        InteractorKind::Unknown => "unknown",
    }
}
