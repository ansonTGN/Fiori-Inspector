use crate::models::*;
use anyhow::{Context, Result};
use chrono::Utc;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub async fn analyze_html_file(path: &Path, max_text_len: usize, max_nodes: usize) -> Result<PageSnapshot> {
    let html = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("No se pudo leer HTML: {}", path.display()))?;
    Ok(analyze_html(&html, Some(path.to_string_lossy().to_string()), max_text_len, max_nodes))
}

pub fn analyze_html(html: &str, url: Option<String>, max_text_len: usize, max_nodes: usize) -> PageSnapshot {
    let doc = Html::parse_document(html);
    let title = Selector::parse("title").ok().and_then(|sel| {
        doc.select(&sel)
            .next()
            .map(|n| collapse_text(&n.text().collect::<Vec<_>>().join(" "), max_text_len))
    });

    let all_selector = Selector::parse("body *").expect("valid selector");
    let mut nodes = Vec::new();
    let mut action_hints = Vec::new();
    let mut endpoints = BTreeSet::new();
    let re_odata = Regex::new(r#"(?i)(/sap/opu/odata[^\"'\s<>]+|/odata4/[^\"'\s<>]+)"#).unwrap();
    for cap in re_odata.captures_iter(html) {
        endpoints.insert(cap[1].to_string());
    }

    for (idx, el) in doc.select(&all_selector).take(max_nodes).enumerate() {
        let node = element_to_dom_node(idx, el, max_text_len);
        if let Some(hint) = action_hint_from_dom(&node) {
            action_hints.push(hint);
        }
        nodes.push(node);
    }

    let controls = nodes
        .iter()
        .filter_map(dom_node_to_pseudo_control)
        .collect::<Vec<_>>();

    let mut warnings = Vec::new();
    warnings.push("Análisis HTML estático: no puede ver controles UI5 no renderizados, bindings reales, modelos vivos ni estado post-login. Para Fiori real usa snapshot-browser.".to_string());

    let odata_endpoints = endpoints
        .into_iter()
        .map(|u| ODataEndpoint {
            service_root: infer_service_root(&u),
            entity_or_path: None,
            url: u,
            source: EndpointSource::HtmlReference,
        })
        .collect::<Vec<_>>();

    let metrics = SnapshotMetrics {
        control_count: controls.len(),
        dom_node_count: nodes.len(),
        endpoint_count: odata_endpoints.len(),
        visible_control_count: controls.len(),
        actionable_control_count: action_hints.len(),
    };

    PageSnapshot {
        schema_version: "fiori-dom-agent.snapshot.v1".to_string(),
        captured_at: Utc::now(),
        mode: SnapshotMode::StaticHtml,
        url,
        title,
        application: None,
        metrics,
        ui5: Ui5RuntimeInfo {
            detected: html.contains("sap-ui-core") || html.contains("sap.ui") || html.contains("sapUiBody"),
            version: None,
            bootstrapped: None,
            core_initialized: None,
            libraries: Vec::new(),
        },
        controls,
        dom_nodes: nodes,
        odata_endpoints,
        action_hints,
        warnings,
    }
}

fn element_to_dom_node(idx: usize, el: ElementRef<'_>, max_text_len: usize) -> DomNode {
    let tag = el.value().name().to_ascii_lowercase();
    let id = attr(el, "id");
    let role = attr(el, "role");
    let text = {
        let raw = el.text().collect::<Vec<_>>().join(" ");
        let collapsed = collapse_text(&raw, max_text_len);
        if collapsed.is_empty() { None } else { Some(collapsed) }
    };
    let classes = attr(el, "class")
        .map(|c| c.split_whitespace().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let interesting_attrs = [
        "id", "role", "aria-label", "aria-labelledby", "aria-describedby", "title", "type", "name",
        "data-sap-ui", "data-sap-ui-related", "data-sap-ui-fastnavgroup", "href", "value",
    ];
    let mut attributes = BTreeMap::new();
    for key in interesting_attrs {
        if let Some(value) = attr(el, key) {
            attributes.insert(key.to_string(), value);
        }
    }

    let mut selector_candidates = Vec::new();
    if let Some(id) = &id {
        selector_candidates.push(format!("#{}", css_escape(id)));
        if let Some(suffix) = stable_suffix(id) {
            selector_candidates.push(format!("[id$='{}']", suffix.replace('"', "\\\"")));
        }
    }
    if let Some(label) = attr(el, "aria-label") {
        selector_candidates.push(format!("[aria-label='{}']", label.replace('"', "\\\"")));
    }
    if is_actionable_tag(&tag) || role.as_deref().map(is_actionable_role).unwrap_or(false) {
        selector_candidates.push(tag.clone());
    }

    DomNode {
        node_id: hash_node(idx, &tag, id.as_deref().unwrap_or_default()),
        tag: tag.clone(),
        id,
        role,
        text,
        aria_label: attr(el, "aria-label"),
        title: attr(el, "title"),
        classes,
        attributes,
        selector_candidates,
        semantic: semantic_from_tag_and_classes(&tag, attr(el, "class").as_deref()),
        rect: None,
    }
}

fn dom_node_to_pseudo_control(node: &DomNode) -> Option<Ui5Control> {
    let is_sapish = node
        .classes
        .iter()
        .any(|c| c.starts_with("sapM") || c.starts_with("sapUi") || c.starts_with("sapUx"))
        || node.attributes.contains_key("data-sap-ui")
        || node.id.as_ref().map(|id| id.contains("---") || id.contains("--")).unwrap_or(false);

    if !is_sapish {
        return None;
    }

    let kind = infer_interactor(node);
    Some(Ui5Control {
        id: node.id.clone().unwrap_or_else(|| node.node_id.clone()),
        control_type: node.attributes.get("data-sap-ui").cloned().or_else(|| node.semantic.clone()),
        short_type: node.semantic.clone(),
        visible: Some(true),
        enabled: None,
        editable: None,
        selected: None,
        busy: None,
        text: node.text.clone(),
        title: node.title.clone(),
        value: node.attributes.get("value").cloned(),
        tooltip: node.title.clone(),
        selected_key: None,
        binding_context_path: None,
        parent_id: None,
        child_ids: Vec::new(),
        aggregations: Vec::new(),
        bindings: Vec::new(),
        models: Vec::new(),
        dom: Some(DomRef {
            dom_id: node.id.clone(),
            tag: Some(node.tag.clone()),
            role: node.role.clone(),
            aria_label: node.aria_label.clone(),
            aria_described_by: node.attributes.get("aria-describedby").cloned(),
            classes: node.classes.clone(),
            rect: None,
        }),
        selector_candidates: node.selector_candidates.clone(),
        interactor: kind,
        confidence: 0.55,
        risk_flags: vec!["pseudo_control_from_static_html".to_string()],
    })
}

fn action_hint_from_dom(node: &DomNode) -> Option<ActionHint> {
    infer_interactor(node).map(|kind| ActionHint {
        label: node
            .aria_label
            .clone()
            .or_else(|| node.text.clone())
            .or_else(|| node.title.clone())
            .unwrap_or_else(|| node.id.clone().unwrap_or_else(|| node.tag.clone())),
        kind,
        selector: node.selector_candidates.first().cloned().unwrap_or_else(|| node.tag.clone()),
        control_id: node.id.clone(),
        confidence: 0.60,
        rationale: "Elemento HTML accionable detectado por etiqueta, rol o clases SAPUI5.".to_string(),
    })
}

fn infer_interactor(node: &DomNode) -> Option<InteractorKind> {
    let tag = node.tag.as_str();
    let role = node.role.as_deref().unwrap_or_default();
    let classes = node.classes.join(" ");
    if tag == "button" || role == "button" || classes.contains("sapMBtn") {
        Some(InteractorKind::Button)
    } else if tag == "a" || role == "link" {
        Some(InteractorKind::Link)
    } else if tag == "input" || tag == "textarea" || classes.contains("sapMInput") {
        Some(InteractorKind::Input)
    } else if role == "combobox" || classes.contains("sapMComboBox") || classes.contains("sapMSlt") {
        Some(InteractorKind::ComboBox)
    } else if role == "checkbox" || classes.contains("sapMCb") {
        Some(InteractorKind::Checkbox)
    } else if role == "radio" || classes.contains("sapMRb") {
        Some(InteractorKind::RadioButton)
    } else if tag == "table" || role == "table" || role == "grid" || classes.contains("sapMListTbl") {
        Some(InteractorKind::Table)
    } else if role == "tab" {
        Some(InteractorKind::Tab)
    } else if role == "menuitem" {
        Some(InteractorKind::MenuItem)
    } else {
        None
    }
}

fn attr(el: ElementRef<'_>, name: &str) -> Option<String> {
    el.value().attr(name).map(|s| s.to_string()).filter(|s| !s.trim().is_empty())
}

fn collapse_text(s: &str, max: usize) -> String {
    let mut out = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.chars().count() > max {
        out = out.chars().take(max).collect::<String>();
        out.push('…');
    }
    out
}

fn hash_node(idx: usize, tag: &str, id: &str) -> String {
    let mut h = Sha256::new();
    h.update(format!("{idx}:{tag}:{id}"));
    format!("n_{:x}", h.finalize())[..18].to_string()
}

fn css_escape(id: &str) -> String {
    id.replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('.', "\\.")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('#', "\\#")
}

fn stable_suffix(id: &str) -> Option<String> {
    id.rsplit("--").next().filter(|s| !s.is_empty()).map(|s| format!("--{s}"))
}

fn is_actionable_tag(tag: &str) -> bool {
    matches!(tag, "button" | "a" | "input" | "select" | "textarea")
}

fn is_actionable_role(role: &str) -> bool {
    matches!(role, "button" | "link" | "textbox" | "combobox" | "checkbox" | "radio" | "tab" | "menuitem" | "grid" | "table")
}

fn semantic_from_tag_and_classes(tag: &str, classes: Option<&str>) -> Option<String> {
    let c = classes.unwrap_or_default();
    if c.contains("sapMBtn") || tag == "button" {
        Some("sap.m.Button".to_string())
    } else if c.contains("sapMInput") || tag == "input" {
        Some("sap.m.Input".to_string())
    } else if c.contains("sapMList") || c.contains("sapMListTbl") || tag == "table" {
        Some("sap.m.Table/List".to_string())
    } else if c.contains("sapMObj") {
        Some("sap.m.Object".to_string())
    } else if c.contains("sapUi") {
        Some("sap.ui.core.Element".to_string())
    } else {
        None
    }
}

fn infer_service_root(url: &str) -> Option<String> {
    let patterns = ["/sap/opu/odata/sap/", "/sap/opu/odata4/", "/odata4/"];
    for p in patterns {
        if let Some(pos) = url.to_ascii_lowercase().find(&p.to_ascii_lowercase()) {
            let tail = &url[pos + p.len()..];
            let service = tail.split('/').next().unwrap_or_default();
            if !service.is_empty() {
                return Some(format!("{}{}", &url[..pos + p.len()], service));
            }
        }
    }
    None
}
