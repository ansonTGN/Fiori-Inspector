use fiori_inspector_studio::static_html;

#[test]
fn static_analyzer_detects_sap_like_controls() {
    let html = r#"
    <html><body class="sapUiBody">
      <button id="app---Main--go" class="sapMBtn" aria-label="Ir">Ir</button>
      <input id="app---Main--mat-inner" class="sapMInputBaseInner" aria-label="Material" />
    </body></html>
    "#;
    let snap = static_html::analyze_html(html, None, 100, 100);
    assert!(snap.ui5.detected);
    assert!(snap.metrics.control_count >= 2);
    assert!(snap.metrics.actionable_control_count >= 2);
}
