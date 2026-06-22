const state = {
  current: null,
  workflow: '',
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
  } catch (_) {
    state.workflow = '';
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
  const items = (report()?.top_actions || []).filter(a => passesFilter(a.label, a.selector, a.control_id, a.kind));
  const box = $('actionsList');
  if (!items.length) {
    box.className = 'object-list empty';
    box.textContent = 'Sin acciones que coincidan con el filtro.';
    return;
  }
  box.className = 'object-list';
  box.innerHTML = items.map(a => `
    <div class="object-card">
      <div class="object-title">
        <strong>${escapeHtml(a.label || a.control_id || 'Acción')}</strong>
        <span class="pill subtle">${escapeHtml(a.kind || 'acción')}</span>
      </div>
      <div class="object-meta">
        <span>confianza ${(Number(a.confidence || 0) * 100).toFixed(0)}%</span>
        ${a.control_id ? `<span>control ${escapeHtml(truncate(a.control_id, 90))}</span>` : ''}
      </div>
      <div class="selector-row">
        <code>${escapeHtml(a.selector)}</code>
        <button class="mini copy" data-copy="${escapeHtml(a.selector)}">Copiar</button>
      </div>
      <p class="muted">${escapeHtml(a.rationale || '')}</p>
    </div>
  `).join('');
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
  $('workflowYaml').textContent = state.workflow || 'Carga una captura para generar un workflow.';
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
  if (!state.workflow) return toast('No hay workflow');
  downloadText(`fiori_workflow_${record().id}.yaml`, 'text/yaml', state.workflow);
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
  $('copySummaryBtn').addEventListener('click', copySummary);
  $('copyWorkflowBtn').addEventListener('click', () => state.workflow ? copyText(state.workflow, 'Workflow copiado') : toast('No hay workflow'));
  $('expandTreeBtn').addEventListener('click', () => toast('El árbol ya muestra hasta 12 niveles. Usa el buscador para aislar ramas.'));
}

wireEvents();
checkHealth();
loadSnapshots();
