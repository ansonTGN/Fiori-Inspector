use crate::models::{ActionHint, InteractorKind, PageSnapshot, Ui5Control};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorAssessment {
    pub selector: String,
    pub score: u8,
    pub level: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationTarget {
    pub label: String,
    pub kind: InteractorKind,
    pub selector: Option<String>,
    pub control_id: Option<String>,
    pub risk_level: String,
    pub selector_assessments: Vec<SelectorAssessment>,
}

pub fn target_from_action(action: &ActionHint, controls: &[Ui5Control]) -> AutomationTarget {
    let control = action
        .control_id
        .as_ref()
        .and_then(|id| controls.iter().find(|c| &c.id == id));
    let selectors = control
        .map(|c| c.selector_candidates.clone())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| vec![action.selector.clone()]);
    let mut assessments = selectors
        .iter()
        .map(|s| {
            assess_selector(
                s,
                action.control_id.as_deref(),
                control.map(|c| c.risk_flags.as_slice()).unwrap_or(&[]),
            )
        })
        .collect::<Vec<_>>();
    assessments.sort_by(|a, b| b.score.cmp(&a.score));
    let selector = assessments
        .first()
        .map(|a| a.selector.clone())
        .or_else(|| Some(action.selector.clone()));
    let risk_level = match assessments.first().map(|a| a.score).unwrap_or(0) {
        80..=100 => "bajo",
        55..=79 => "medio",
        _ => "alto",
    }
    .to_string();

    AutomationTarget {
        label: action.label.clone(),
        kind: action.kind.clone(),
        selector,
        control_id: action.control_id.clone(),
        risk_level,
        selector_assessments: assessments,
    }
}

pub fn assess_selector(
    selector: &str,
    control_id: Option<&str>,
    risk_flags: &[String],
) -> SelectorAssessment {
    let mut score: i32 = 50;
    let mut reasons = Vec::new();
    if selector.starts_with("[id$='--") || selector.starts_with("[id$=\"--") {
        score += 28;
        reasons.push("usa sufijo de stable ID UI5".to_string());
    }
    if selector.starts_with("[aria-label=") {
        score += 22;
        reasons.push("usa atributo accesible".to_string());
    }
    if selector.starts_with('#') {
        score += 8;
        reasons.push("usa ID directo".to_string());
    }
    if selector.contains("#__") || selector.contains("#__button") || selector.contains("#__xmlview")
    {
        score -= 32;
        reasons.push("parece ID generado por framework".to_string());
    }
    if selector == "button"
        || selector == "input"
        || selector == "a"
        || selector == "div"
        || selector == "span"
    {
        score -= 30;
        reasons.push("selector demasiado genérico".to_string());
    }
    if control_id.is_some() {
        score += 8;
        reasons.push("hay control_id UI5 asociado".to_string());
    }
    if risk_flags.iter().any(|r| r.contains("generated")) {
        score -= 10;
        reasons.push("el control tiene riesgo de ID dinámico".to_string());
    }
    if selector.len() > 120 {
        score -= 10;
        reasons.push("selector largo y difícil de mantener".to_string());
    }

    let score = score.clamp(0, 100) as u8;
    let level = match score {
        80..=100 => "recomendado",
        55..=79 => "usable_con_validación",
        _ => "riesgoso",
    }
    .to_string();

    SelectorAssessment {
        selector: selector.to_string(),
        score,
        level,
        reason: if reasons.is_empty() {
            "sin señales específicas".to_string()
        } else {
            reasons.join("; ")
        },
    }
}

pub fn production_workflow_from_snapshot(snapshot: &PageSnapshot) -> String {
    let mut out = String::new();
    out.push_str("name: \"Automatización Fiori productiva generada desde Inspector Studio\"\n");
    out.push_str("version: \"1.0\"\n");
    out.push_str("environment: \"dev\"\n");
    out.push_str("description: \"Plantilla robusta. Revisar datos, permisos, validaciones y selectores antes de uso productivo.\"\n\n");
    out.push_str("variables:\n");
    out.push_str("  ejemplo_valor: \"REEMPLAZAR\"\n");
    out.push_str("  usuario_ejecucion: \"{{env.USER}}\"\n\n");
    out.push_str("defaults:\n");
    out.push_str("  timeout_secs: 30\n");
    out.push_str("  retry:\n");
    out.push_str("    attempts: 3\n");
    out.push_str("    delay_ms: 800\n");
    out.push_str("  capture_before_each_step: false\n");
    out.push_str("  capture_after_each_step: false\n\n");
    out.push_str("steps:\n");
    if let Some(url) = &snapshot.url {
        out.push_str("  - action: goto\n");
        out.push_str("    name: \"Abrir aplicación Fiori\"\n");
        out.push_str(&format!("    url: {:?}\n", url));
        out.push_str("\n  - action: wait_ui5\n");
        out.push_str("    name: \"Esperar runtime SAPUI5\"\n");
        out.push_str("    timeout_secs: 90\n\n");
        out.push_str("  - action: snapshot\n");
        out.push_str("    name: \"Evidencia inicial\"\n");
        out.push_str("    save_as: \"01_inicio.json\"\n\n");
    }

    let mut added = 0usize;
    for action in snapshot.action_hints.iter().take(20) {
        let target = target_from_action(action, &snapshot.controls);
        if target.risk_level == "alto" {
            continue;
        }
        added += 1;
        match target.kind {
            InteractorKind::Input | InteractorKind::ComboBox => {
                out.push_str("  - action: input\n");
                out.push_str(&format!(
                    "    name: {:?}\n",
                    format!("Rellenar {}", target.label)
                ));
                if let Some(control_id) = &target.control_id {
                    out.push_str(&format!("    control_id: {:?}\n", control_id));
                } else if let Some(selector) = &target.selector {
                    out.push_str(&format!("    selector: {:?}\n", selector));
                }
                out.push_str("    value: \"{{vars.ejemplo_valor}}\"\n");
                out.push_str("    clear: true\n");
                out.push_str("    assert:\n");
                out.push_str("      exists: true\n");
                out.push_str("      visible: true\n\n");
            }
            InteractorKind::Button
            | InteractorKind::Link
            | InteractorKind::Tab
            | InteractorKind::MenuItem => {
                out.push_str("  - action: click\n");
                out.push_str(&format!(
                    "    name: {:?}\n",
                    format!("Ejecutar {}", target.label)
                ));
                if let Some(control_id) = &target.control_id {
                    out.push_str(&format!("    control_id: {:?}\n", control_id));
                } else if let Some(selector) = &target.selector {
                    out.push_str(&format!("    selector: {:?}\n", selector));
                }
                out.push_str("    retry:\n");
                out.push_str("      attempts: 3\n");
                out.push_str("      delay_ms: 800\n");
                out.push_str("    assert:\n");
                out.push_str("      ui5_ready: true\n\n");
            }
            _ => {}
        }
        if added >= 8 {
            break;
        }
    }

    out.push_str("  - action: snapshot\n");
    out.push_str("    name: \"Evidencia final\"\n");
    out.push_str("    save_as: \"99_resultado_final.json\"\n\n");
    out.push_str("  - action: screenshot\n");
    out.push_str("    name: \"Captura visual final\"\n");
    out.push_str("    save_as: \"99_resultado_final.png\"\n");
    out
}
