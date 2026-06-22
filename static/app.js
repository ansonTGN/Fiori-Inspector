const state = {
  current: null,
  workflow: '',
  workflowGenerated: '',
  workflowDirty: false,
  workflowValidation: null,
  workflowExecution: null,
  selectedStep: '',
  query: '',
};

const $ = (id) => document.getElementById(id);

function toast(message) {
  const el = $('toast');
  el.textContent = message;
  el.classList.add('show');
  setTimeout(() => el.classList.remove('show'), 2200);
}

async function api(path, options = {}) {
  const res = await fetch(path, {
    headers: { 'Content-Type': 'application/json', ...(options.headers || {}) },
    ...options,
  });
  if (!res.ok) {
    let msg = `${res.status} ${res.statusText}`;
    try {
      const body = await res.json();
      if (body.error) msg = body.error;
    } catch (_) {}
    throw new Error(msg);
  }
  return res.json();
}

function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}

function truncate(value, max = 120) {
  const s = String(value ?? '');
  return s.length > max ? `${s.slice(0, max)}…` : s;
}

function copyText(value, label = 'Copiado') {
  navigator.clipboard.writeText(value).then(() => toast(label)).catch(() => toast('No se pudo copiar'));
}

function downloadText(filename, mime, text) {
  const blob = new Blob([text], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

function setBusy(button, busy, labelBusy = 'Procesando…') {
  if (!button) return;
  if (busy) {
    button.dataset.originalText = button.textContent;
    button.textContent = labelBusy;
    button.disabled = true;
  } else {
    button.textContent = button.dataset.originalText || button.textContent;
    button.disabled = false;
  }
}

async function checkHealth() {
  const dot = $('healthDot');
  const text = $('healthText');
  try {
    const health = await api('/api/health');
    dot.className = 'dot ok';
    text.textContent = `${health.name} activo`;
  } catch (e) {
    dot.className = 'dot bad';
    text.textContent = 'Backend no disponible';
  }
}

async function loadSnapshots() {
  try {
    const list = await api('/api/snapshots');
    renderSnapshotList(list);
  } catch (e) {
    console.error(e);
  }
}

function renderSnapshotList(list) {
  const box = $('snapshotList');
  if (!list.length) {
    box.className = 'snapshot-list empty';
    box.textContent = 'Sin capturas todavía';
    return;
  }
  box.className = 'snapshot-list';
  box.innerHTML = list.map(item => `
    <div class="snapshot-item ${state.current?.id === item.id ? 'active' : ''}" data-id="${escapeHtml(item.id)}">
      <strong>${escapeHtml(item.name || item.title || 'Snapshot')}</strong>
      <span>${escapeHtml(item.mode)} · ${item.control_count} controles · ${item.quality_score}/100</span>
    </div>
  `).join('');
  box.querySelectorAll('.snapshot-item').forEach(el => {
    el.addEventListener('click', async () => {
      const record = await api(`/api/snapshots/${el.dataset.id}`);
      await setCurrent(record);
    });
  });
}

async function setCurrent(record) {
  state.current = record;
  try {
    const wf = await api(`/api/snapshots/${record.id}/workflow`);
    state.workflow = wf.yaml || '';
    state.workflowGenerated = state.workflow;
    state.workflowDirty = false;
    state.workflowValidation = null;
    state.workflowExecution = null;
    state.selectedStep = '';
  } catch (_) {
    state.workflow = '';
    state.workflowGenerated = '';
    state.workflowDirty = false;
    state.workflowValidation = null;
    state.workflowExecution = null;
    state.selectedStep = '';
  }
  renderAll();
  await loadSnapshots();
  showView('overview');
}

function showView(id) {
  document.querySelectorAll('.view').forEach(v => v.classList.remove('active-view'));
  document.querySelectorAll('.nav-item').forEach(v => v.classList.remove('active'));
  $(id)?.classList.add('active-view');
  document.querySelector(`.nav-item[data-view="${id}"]`)?.classList.add('active');
}

function record() { return state.current; }
function snapshot() { return state.current?.snapshot; }
function report() { return state.current?.report; }

function renderAll() {
  renderScore();
  renderMetrics();
  renderOverview();
  renderActions();
  renderTree();
  renderBindings();
  renderRisks();
  renderWorkflow();
}

function renderScore() {
  const score = report()?.quality_score;
  const ring = $('scoreRing');
  $('scoreValue').textContent = score == null ? '—' : score;
  const deg = score == null ? 0 : Math.round(score * 3.6);
  ring.style.background = `conic-gradient(var(--blue) ${deg}deg, rgba(0,0,0,.08) ${deg}deg)`;
}

function renderMetrics() {
  const s = snapshot();
  const r = report();
  const metrics = [
    ['Calidad', r?.quality_score ?? '—'],
    ['Controles', s?.metrics?.control_count ?? '—'],
    ['Acciones', s?.metrics?.actionable_control_count ?? '—'],
    ['DOM', s?.metrics?.dom_node_count ?? '—'],
    ['OData', s?.metrics?.endpoint_count ?? '—'],
  ];
  $('metricGrid').innerHTML = metrics.map(([label, value]) => `
    <div class="metric"><strong>${escapeHtml(value)}</strong><span>${escapeHtml(label)}</span></div>
  `).join('');
}

function renderOverview() {
  const r = report();
  $('executiveSummary').textContent = r?.executive_summary || 'Carga una captura para obtener el diagnóstico.';
  $('patterns').innerHTML = (r?.detected_patterns || []).map(p => `<span class="chip">${escapeHtml(p)}</span>`).join('');
}

function passesFilter(...values) {
  const q = state.query.trim().toLowerCase();
  if (!q) return true;
  return values.join(' ').toLowerCase().includes(q);
}

function renderActions() {
  const controls = new Map((report()?.high_value_controls || []).map(c => [c.id, c]));
  const items = (report()?.top_actions || []).filter(a => passesFilter(a.label, a.selector, a.control_id, a.kind));
  const box = $('actionsList');
  if (!items.length) {
    box.className = 'object-list empty';
    box.textContent = 'Sin acciones que coincidan con el filtro.';
    return;
  }
  box.className = 'object-list';
  box.innerHTML = items.map(a => {
    const c = a.control_id ? controls.get(a.control_id) : null;
    const recommended = c?.recommended_selector || a.selector;
    const score = c?.selector_quality?.score;
    const level = c?.selector_quality?.level || 'sin evaluar';
    return `
    <div class="object-card">
      <div class="object-title">
        <strong>${escapeHtml(a.label || a.control_id || 'Acción')}</strong>
        <span class="pill subtle">${escapeHtml(a.kind || 'acción')}</span>
      </div>
      <div class="object-meta">
        <span>confianza ${(Number(a.confidence || 0) * 100).toFixed(0)}%</span>
        ${score == null ? '' : `<span>selector ${score}/100 · ${escapeHtml(level)}</span>`}
        ${a.control_id ? `<span>control ${escapeHtml(truncate(a.control_id, 90))}</span>` : ''}
      </div>
      <div class="selector-row">
        <code>${escapeHtml(recommended)}</code>
        <button class="mini copy" data-copy="${escapeHtml(recommended)}">Copiar</button>
      </div>
      ${c?.selector_quality?.reason ? `<p class="muted">Selector: ${escapeHtml(c.selector_quality.reason)}</p>` : ''}
      <p class="muted">${escapeHtml(a.rationale || '')}</p>
    </div>`;
  }).join('');
  box.querySelectorAll('[data-copy]').forEach(btn => btn.addEventListener('click', () => copyText(btn.dataset.copy, 'Selector copiado')));
}

function renderTree() {
  const controls = (snapshot()?.controls || []).filter(c => passesFilter(c.id, c.text, c.title, c.value, c.control_type));
  const box = $('treeView');
  if (!controls.length) {
    box.className = 'tree empty';
    box.textContent = 'Sin controles que coincidan con el filtro.';
    return;
  }
  box.className = 'tree';
  const byParent = new Map();
  const byId = new Map(controls.map(c => [c.id, c]));
  for (const c of controls) {
    const parent = c.parent_id && byId.has(c.parent_id) ? c.parent_id : '__root__';
    if (!byParent.has(parent)) byParent.set(parent, []);
    byParent.get(parent).push(c);
  }
  const renderNode = (c, depth = 0, seen = new Set()) => {
    if (seen.has(c.id) || depth > 12) return '';
    seen.add(c.id);
    const children = byParent.get(c.id) || [];
    const label = c.text || c.title || c.value || '';
    return `<div class="tree-node" style="margin-left:${Math.min(depth * 14, 120)}px">
      <div class="tree-line">
        <span class="tree-type">${escapeHtml(shortType(c.control_type || c.short_type || 'control'))}</span>
        <span class="tree-label">${escapeHtml(truncate(label, 70))}</span>
        <span class="tree-id">${escapeHtml(truncate(c.id, 110))}</span>
      </div>
      ${children.map(child => renderNode(child, depth + 1, new Set(seen))).join('')}
    </div>`;
  };
  const roots = byParent.get('__root__') || [];
  box.innerHTML = roots.slice(0, 500).map(c => renderNode(c)).join('');
}

function shortType(value) {
  const s = String(value || '');
  return s.split('.').pop() || s;
}

function renderBindings() {
  const bindings = (report()?.binding_inventory || []).filter(b => passesFilter(b.control_id, b.control_label, b.control_type, JSON.stringify(b.bindings || []), b.context_path));
  const bBox = $('bindingsList');
  if (!bindings.length) {
    bBox.className = 'object-list empty';
    bBox.textContent = 'Sin bindings que coincidan con el filtro.';
  } else {
    bBox.className = 'object-list';
    bBox.innerHTML = bindings.map(b => `
      <div class="object-card">
        <div class="object-title"><strong>${escapeHtml(b.control_label || b.control_id)}</strong><span class="pill subtle">${escapeHtml(shortType(b.control_type))}</span></div>
        <div class="object-meta"><span>${escapeHtml(truncate(b.control_id, 100))}</span></div>
        ${b.context_path ? `<div class="selector-row"><code>${escapeHtml(b.context_path)}</code><button class="mini copy" data-copy="${escapeHtml(b.context_path)}">Copiar</button></div>` : ''}
        ${(b.bindings || []).map(x => `<div class="selector-row"><code>${escapeHtml(`${x.property}: ${x.model ? x.model + '>' : ''}${x.path || ''}`)}</code></div>`).join('')}
      </div>`).join('');
  }

  const endpoints = (report()?.endpoint_inventory || []).filter(e => passesFilter(e.url, e.service_root, e.entity_or_path, e.source));
  const oBox = $('odataList');
  if (!endpoints.length) {
    oBox.className = 'object-list empty';
    oBox.textContent = 'Sin endpoints que coincidan con el filtro.';
  } else {
    oBox.className = 'object-list';
    oBox.innerHTML = endpoints.map(e => `
      <div class="object-card">
        <div class="object-title"><strong>${escapeHtml(e.service_root || e.url)}</strong><span class="pill subtle">${escapeHtml(e.source || 'odata')}</span></div>
        <div class="selector-row"><code>${escapeHtml(e.url)}</code><button class="mini copy" data-copy="${escapeHtml(e.url)}">Copiar</button></div>
        ${e.entity_or_path ? `<div class="object-meta"><span>${escapeHtml(e.entity_or_path)}</span></div>` : ''}
      </div>`).join('');
  }
  document.querySelectorAll('#bindings [data-copy]').forEach(btn => btn.addEventListener('click', () => copyText(btn.dataset.copy, 'Copiado')));
}

function renderRisks() {
  const risks = (report()?.automation_risks || []).filter(r => passesFilter(r.severity, r.title, r.detail, r.remediation));
  const box = $('riskList');
  if (!risks.length) {
    box.className = 'object-list empty';
    box.textContent = 'Sin riesgos que coincidan con el filtro.';
  } else {
    box.className = 'object-list';
    box.innerHTML = risks.map(r => `
      <div class="object-card">
        <div class="object-title"><strong>${escapeHtml(r.title)}</strong><span class="sev sev-${escapeHtml(r.severity)}">${escapeHtml(r.severity)}</span></div>
        <p class="muted">${escapeHtml(r.detail)}</p>
        <div class="selector-row"><code>${escapeHtml(r.remediation)}</code></div>
      </div>`).join('');
  }
  const recs = report()?.recommendations || [];
  $('recommendations').innerHTML = recs.map(r => `<div class="recommendation">${escapeHtml(r)}</div>`).join('');
}

function renderWorkflow() {
  const editor = $('workflowEditor');
  if (editor && document.activeElement !== editor) {
    editor.value = state.workflow || '';
  }
  renderWorkflowValidation();
  renderWorkflowExecution();
}

function currentWorkflowYaml() {
  return $('workflowEditor')?.value ?? state.workflow ?? '';
}

function renderWorkflowValidation() {
  const validationBox = $('workflowValidation');
  const stepBox = $('workflowStepList');
  const select = $('workflowStepSelect');
  if (!validationBox || !stepBox || !select) return;

  const validation = state.workflowValidation;
  if (!validation) {
    validationBox.className = 'workflow-validation empty';
    validationBox.textContent = state.workflow ? 'Workflow generado. Pulsa Validar para revisar sintaxis, riesgos y pasos.' : 'Valida el YAML para ver errores, avisos y pasos ejecutables.';
    stepBox.className = 'workflow-step-list empty';
    stepBox.textContent = 'Sin pasos cargados.';
    select.innerHTML = '<option value="">Primero valida el workflow</option>';
    return;
  }

  const errors = validation.errors || [];
  const warnings = validation.warnings || [];
  validationBox.className = `workflow-validation ${validation.ok ? 'ok' : 'bad'}`;
  validationBox.innerHTML = `
    <strong>${validation.ok ? 'Workflow válido' : 'Workflow con errores'}</strong>
    <p>${escapeHtml(validation.workflow_name || 'Sin nombre')} · ${validation.step_count || 0} pasos · entorno ${escapeHtml(validation.environment || 'sin definir')}</p>
    ${errors.length ? `<div><b>Errores</b><ul>${errors.map(e => `<li>${escapeHtml(e)}</li>`).join('')}</ul></div>` : ''}
    ${warnings.length ? `<div><b>Avisos</b><ul>${warnings.map(w => `<li>${escapeHtml(w)}</li>`).join('')}</ul></div>` : ''}
  `;

  const steps = validation.steps || [];
  select.innerHTML = steps.length
    ? steps.map(s => `<option value="${s.index}" ${String(state.selectedStep) === String(s.index) ? 'selected' : ''}>${String(s.index).padStart(2, '0')} · ${escapeHtml(s.action)} · ${escapeHtml(truncate(s.name, 60))}</option>`).join('')
    : '<option value="">Sin pasos</option>';
  if (!state.selectedStep && steps.length) state.selectedStep = String(steps[0].index);

  if (!steps.length) {
    stepBox.className = 'workflow-step-list empty';
    stepBox.textContent = 'Sin pasos cargados.';
    return;
  }
  stepBox.className = 'workflow-step-list';
  stepBox.innerHTML = steps.map(step => `
    <div class="workflow-step-card ${String(state.selectedStep) === String(step.index) ? 'active' : ''}" data-step="${step.index}">
      <div class="step-head">
        <span class="step-index">${String(step.index).padStart(2, '0')}</span>
        <div class="step-title">
          <strong>${escapeHtml(step.name)}</strong>
          <span>${escapeHtml(step.action)}${step.optional ? ' · opcional' : ''}</span>
        </div>
        <span class="pill risk-${escapeHtml(step.risk_level)}">riesgo ${escapeHtml(step.risk_level)}</span>
      </div>
      <div class="step-target">
        ${step.control_id ? `<code>control_id: ${escapeHtml(step.control_id)}</code>` : ''}
        ${step.selector ? `<code>selector: ${escapeHtml(step.selector)}</code>` : ''}
        ${step.text ? `<code>text: ${escapeHtml(step.text)}</code>` : ''}
        ${step.value ? `<code>value: ${escapeHtml(step.value)}</code>` : ''}
      </div>
      <div class="step-notes">
        ${(step.notes || []).map(n => `<span>• ${escapeHtml(n)}</span>`).join('')}
      </div>
    </div>
  `).join('');
  stepBox.querySelectorAll('.workflow-step-card').forEach(card => {
    card.addEventListener('click', () => {
      state.selectedStep = card.dataset.step;
      renderWorkflowValidation();
    });
  });
}

function renderWorkflowExecution() {
  const box = $('workflowExecution');
  if (!box) return;
  const run = state.workflowExecution;
  if (!run) {
    box.className = 'workflow-execution empty';
    box.textContent = 'Todavía no se ha ejecutado ningún workflow desde el estudio.';
    return;
  }
  const report = run.report;
  const steps = report?.steps || [];
  const passed = steps.filter(s => s.status === 'passed').length;
  const failed = steps.filter(s => s.status === 'failed').length;
  const skipped = steps.filter(s => s.status === 'skipped_optional').length;
  box.className = `workflow-execution ${run.ok ? 'ok' : 'bad'}`;
  box.innerHTML = `
    <strong>${escapeHtml(run.message)}</strong>
    <p>Directorio de salida: <code>${escapeHtml(run.output_dir || 'no disponible')}</code></p>
    <div class="execution-grid">
      <div class="execution-metric"><strong>${escapeHtml(report?.status || (run.ok ? 'ok' : 'error'))}</strong><span>estado</span></div>
      <div class="execution-metric"><strong>${steps.length}</strong><span>pasos ejecutados</span></div>
      <div class="execution-metric"><strong>${passed}/${failed}/${skipped}</strong><span>ok / error / omitidos</span></div>
      <div class="execution-metric"><strong>${report?.duration_ms ?? '—'}</strong><span>ms</span></div>
    </div>
    ${steps.length ? `<div class="workflow-step-list">${steps.map(s => `
      <div class="workflow-step-card">
        <div class="step-head">
          <span class="step-index">${String(s.index).padStart(2, '0')}</span>
          <div class="step-title"><strong>${escapeHtml(s.name)}</strong><span>${escapeHtml(s.action)} · ${escapeHtml(s.status)} · ${s.duration_ms} ms · ${s.attempts} intento(s)</span></div>
        </div>
        ${s.error ? `<p class="muted">Error: ${escapeHtml(s.error)}</p>` : ''}
        ${(s.evidence || []).length ? `<div class="step-notes">${s.evidence.map(e => `<span>evidencia: ${escapeHtml(e)}</span>`).join('')}</div>` : ''}
      </div>`).join('')}</div>` : ''}
  `;
}

async function validateWorkflow() {
  const yaml = currentWorkflowYaml();
  if (!yaml.trim()) return toast('El workflow está vacío');
  const btn = $('validateWorkflowBtn');
  setBusy(btn, true, 'Validando…');
  try {
    const result = await api('/api/workflows/validate', {
      method: 'POST',
      body: JSON.stringify({ yaml }),
    });
    state.workflow = yaml;
    state.workflowValidation = result;
    renderWorkflow();
    toast(result.ok ? 'Workflow válido' : 'Workflow con errores');
  } catch (e) {
    toast(e.message);
  } finally {
    setBusy(btn, false);
  }
}

async function runWorkflow(untilStep = null) {
  const yaml = currentWorkflowYaml();
  if (!yaml.trim()) return toast('El workflow está vacío');
  const button = untilStep ? $('runUntilStepBtn') : $('runWorkflowBtn');
  setBusy(button, true, untilStep ? `Ejecutando hasta paso ${untilStep}…` : 'Ejecutando…');
  try {
    const result = await api('/api/workflows/run', {
      method: 'POST',
      body: JSON.stringify({ yaml, until_step: untilStep ? Number(untilStep) : null }),
    });
    state.workflow = yaml;
    state.workflowValidation = result.validation;
    state.workflowExecution = result;
    renderWorkflow();
    toast(result.ok ? 'Ejecución completada' : 'Ejecución con error');
  } catch (e) {
    toast(e.message);
  } finally {
    setBusy(button, false);
  }
}

async function captureBrowser() {
  const btn = $('captureBrowserBtn');
  const url = $('browserUrl').value.trim();
  if (!url) return toast('Introduce una URL Fiori');
  setBusy(btn, true, 'Capturando…');
  try {
    const record = await api('/api/snapshots/browser', {
      method: 'POST',
      body: JSON.stringify({ url, name: url }),
    });
    await setCurrent(record);
    toast('Captura completada');
  } catch (e) {
    toast(e.message);
  } finally {
    setBusy(btn, false);
  }
}

async function analyzeHtml() {
  const btn = $('analyzeHtmlBtn');
  setBusy(btn, true, 'Analizando…');
  try {
    let html = $('htmlPaste').value;
    const file = $('htmlFile').files?.[0];
    if (file) html = await file.text();
    if (!html.trim()) return toast('Carga o pega HTML');
    const record = await api('/api/snapshots/static', {
      method: 'POST',
      body: JSON.stringify({ html, url: file ? `file://${file.name}` : 'paste://html', name: file?.name || 'HTML pegado' }),
    });
    await setCurrent(record);
    toast('HTML analizado');
  } catch (e) {
    toast(e.message);
  } finally {
    setBusy(btn, false);
  }
}

function downloadJson() {
  if (!record()) return toast('No hay snapshot');
  downloadText(`fiori_snapshot_${record().id}.json`, 'application/json', JSON.stringify(record().snapshot, null, 2));
}

function downloadWorkflow() {
  const yaml = currentWorkflowYaml();
  if (!yaml.trim()) return toast('No hay workflow');
  const suffix = record()?.id || 'editado';
  downloadText(`fiori_workflow_${suffix}.yaml`, 'text/yaml', yaml);
}

function copySummary() {
  if (!report()) return toast('No hay resumen');
  const text = [
    'Fiori Inspector Studio',
    report().executive_summary,
    '',
    'Recomendaciones:',
    ...(report().recommendations || []).map(x => `- ${x}`),
  ].join('\n');
  copyText(text, 'Resumen copiado');
}

function copyExecutionReport() {
  if (!state.workflowExecution) return toast('No hay informe de ejecución');
  copyText(JSON.stringify(state.workflowExecution, null, 2), 'Informe copiado');
}

function wireEvents() {
  document.querySelectorAll('.nav-item').forEach(btn => btn.addEventListener('click', () => showView(btn.dataset.view)));
  $('openHelpBtn')?.addEventListener('click', () => showView('help'));
  $('helpStartBtn')?.addEventListener('click', () => showView('capture'));
  $('helpOpenCaptureBtn')?.addEventListener('click', () => showView('capture'));
  document.addEventListener('click', (event) => {
    const btn = event.target.closest('.copy-command');
    if (btn?.dataset?.copy) copyText(btn.dataset.copy, 'Comando copiado');
  });
  $('captureBrowserBtn').addEventListener('click', captureBrowser);
  $('analyzeHtmlBtn').addEventListener('click', analyzeHtml);
  $('globalSearch').addEventListener('input', (e) => { state.query = e.target.value; renderAll(); });
  $('downloadJsonBtn').addEventListener('click', downloadJson);
  $('downloadWorkflowBtn').addEventListener('click', downloadWorkflow);
  $('downloadWorkflowEditorBtn')?.addEventListener('click', downloadWorkflow);
  $('copySummaryBtn').addEventListener('click', copySummary);
  $('copyWorkflowBtn').addEventListener('click', () => { const yaml = currentWorkflowYaml(); yaml.trim() ? copyText(yaml, 'Workflow copiado') : toast('No hay workflow'); });
  $('validateWorkflowBtn')?.addEventListener('click', validateWorkflow);
  $('runWorkflowBtn')?.addEventListener('click', () => runWorkflow(null));
  $('runUntilStepBtn')?.addEventListener('click', () => { const step = $('workflowStepSelect')?.value || state.selectedStep; step ? runWorkflow(step) : toast('Selecciona un paso'); });
  $('workflowStepSelect')?.addEventListener('change', (e) => { state.selectedStep = e.target.value; renderWorkflowValidation(); });
  $('workflowEditor')?.addEventListener('input', (e) => { state.workflow = e.target.value; state.workflowDirty = state.workflow !== state.workflowGenerated; state.workflowValidation = null; });
  $('resetWorkflowBtn')?.addEventListener('click', () => { state.workflow = state.workflowGenerated || ''; state.workflowDirty = false; state.workflowValidation = null; state.workflowExecution = null; renderWorkflow(); toast('Workflow restaurado'); });
  $('copyExecutionReportBtn')?.addEventListener('click', copyExecutionReport);
  $('expandTreeBtn').addEventListener('click', () => toast('El árbol ya muestra hasta 12 niveles. Usa el buscador para aislar ramas.'));
}

wireEvents();
checkHealth();
loadSnapshots();
