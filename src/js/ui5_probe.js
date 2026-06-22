return (function (cfg) {
  "use strict";

  cfg = cfg || {};
  const maxTextLen = cfg.max_text_len || 240;
  const includeHidden = !!cfg.include_hidden_controls;
  const maxControls = cfg.max_controls || 5000;
  const maxDomNodes = cfg.max_dom_nodes || 3000;
  const includeDomNodes = cfg.include_dom_nodes !== false;
  const includePerformanceUrls = cfg.include_performance_urls !== false;

  function nowIso() { return new Date().toISOString(); }
  function safe(fn, fallback) { try { return fn(); } catch (e) { return fallback; } }
  function isFn(obj, name) { return obj && typeof obj[name] === "function"; }
  function str(v) { return v === undefined || v === null ? null : String(v); }
  function compactText(s) {
    if (s === undefined || s === null) return null;
    let t = String(s).replace(/\s+/g, " ").trim();
    if (!t) return null;
    if (t.length > maxTextLen) t = t.slice(0, maxTextLen) + "…";
    return t;
  }
  function cssEscape(value) {
    if (window.CSS && CSS.escape) return CSS.escape(value);
    return String(value).replace(/([ #;?%&,.+*~':"!^$[\]()=>|/@])/g, "\\$1");
  }
  function classList(el) {
    if (!el || !el.classList) return [];
    return Array.prototype.slice.call(el.classList).filter(Boolean);
  }
  function rectOf(el) {
    if (!el || !el.getBoundingClientRect) return null;
    const r = el.getBoundingClientRect();
    return { x: r.x, y: r.y, width: r.width, height: r.height };
  }
  function isVisibleDom(el) {
    if (!el) return false;
    const r = el.getBoundingClientRect ? el.getBoundingClientRect() : null;
    const cs = window.getComputedStyle ? getComputedStyle(el) : null;
    return !!r && r.width > 0 && r.height > 0 && (!cs || (cs.visibility !== "hidden" && cs.display !== "none" && Number(cs.opacity || 1) !== 0));
  }
  function domRef(el) {
    if (!el) return null;
    return {
      dom_id: el.id || null,
      tag: el.tagName ? el.tagName.toLowerCase() : null,
      role: el.getAttribute ? el.getAttribute("role") : null,
      aria_label: el.getAttribute ? el.getAttribute("aria-label") : null,
      aria_described_by: el.getAttribute ? el.getAttribute("aria-describedby") : null,
      classes: classList(el),
      rect: rectOf(el)
    };
  }
  function inferInteractor(type, dom, text) {
    const t = (type || "").toLowerCase();
    const role = (dom && dom.role || "").toLowerCase();
    const tag = (dom && dom.tag || "").toLowerCase();
    const cls = (dom && dom.classes || []).join(" ");
    if (t.includes("button") || tag === "button" || role === "button" || cls.includes("sapMBtn")) return "button";
    if (t.includes("link") || tag === "a" || role === "link") return "link";
    if (t.includes("input") || t.includes("textarea") || tag === "input" || tag === "textarea") return "input";
    if (t.includes("combobox") || t.includes("select") || role === "combobox") return "combo_box";
    if (t.includes("checkbox") || role === "checkbox") return "checkbox";
    if (t.includes("radio") || role === "radio") return "radio_button";
    if (t.includes("table") || t.includes("list") || role === "grid" || role === "table") return "table";
    if (role === "row") return "row";
    if (role === "tab") return "tab";
    if (role === "menuitem") return "menu_item";
    if (text && (tag === "span" || tag === "div")) return "unknown";
    return null;
  }
  function selectorCandidates(id, dom, text) {
    const out = [];
    if (id) {
      out.push("#" + cssEscape(id));
      const suffix = id.includes("--") ? "--" + id.split("--").pop() : null;
      if (suffix && suffix.length > 2) out.push("[id$='" + suffix.replace(/'/g, "\\'") + "']");
    }
    if (dom && dom.dom_id && dom.dom_id !== id) out.push("#" + cssEscape(dom.dom_id));
    if (dom && dom.aria_label) out.push("[aria-label='" + dom.aria_label.replace(/'/g, "\\'") + "']");
    if (dom && dom.role && text) out.push("[role='" + dom.role + "']");
    return Array.from(new Set(out));
  }
  function call0(o, m) { return isFn(o, m) ? safe(() => o[m](), null) : null; }
  function boolCall(o, m) { const v = call0(o, m); return v === null ? null : !!v; }
  function stringCall(o, m) { const v = call0(o, m); return compactText(v); }
  function metadataName(o) { return safe(() => o.getMetadata().getName(), null); }
  function shortType(name) { return name ? name.split(".").slice(-1)[0] : null; }

  function bindingInfo(ctrl) {
    const result = [];
    const props = safe(() => Object.keys(ctrl.getMetadata().getAllProperties()), []);
    props.forEach(function (p) {
      const bi = safe(() => ctrl.getBindingInfo(p), null);
      if (!bi) return;
      const item = { property: p, path: bi.path || null, model: bi.model || null, parts: [] };
      if (Array.isArray(bi.parts)) {
        item.parts = bi.parts.map(function (part) {
          return { path: part.path || null, model: part.model || null, type_name: safe(() => part.type.getName(), null) };
        });
      }
      result.push(item);
    });
    return result;
  }

  function modelInfo(ctrl) {
    const out = [];
    function pushModel(name, model) {
      if (!model) return;
      out.push({
        name: name || null,
        class_name: metadataName(model),
        service_url: safe(() => model.sServiceUrl, null) || safe(() => model.getServiceUrl(), null) || null,
        default_binding_mode: safe(() => String(model.getDefaultBindingMode()), null)
      });
    }
    pushModel(null, safe(() => ctrl.getModel(), null));
    const mModels = safe(() => ctrl.oModels || {}, {});
    Object.keys(mModels || {}).forEach(k => pushModel(k, mModels[k]));
    return out.filter((v, idx, arr) => idx === arr.findIndex(x => x.name === v.name && x.class_name === v.class_name && x.service_url === v.service_url));
  }

  function aggregationInfo(ctrl) {
    const out = [];
    const aggs = safe(() => ctrl.getMetadata().getAllAggregations(), {});
    Object.keys(aggs || {}).forEach(function (name) {
      const meta = aggs[name] || {};
      const val = safe(() => ctrl.getAggregation(name), null);
      if (!val) return;
      const children = Array.isArray(val) ? val : [val];
      const childIds = children.map(c => safe(() => c.getId(), null)).filter(Boolean);
      if (childIds.length === 0) return;
      out.push({ name: name, multiple: !!meta.multiple, type_name: meta.type || null, child_ids: childIds });
    });
    return out;
  }

  function contextPath(ctrl) {
    return safe(() => {
      const ctx = ctrl.getBindingContext && ctrl.getBindingContext();
      return ctx && ctx.getPath ? ctx.getPath() : null;
    }, null);
  }

  function getAllElements() {
    const sapObj = window.sap;
    if (!sapObj || !sapObj.ui) return [];
    const Element = safe(() => sap.ui.core.Element, null);
    const registryAll = safe(() => Element && Element.registry && Element.registry.all && Element.registry.all(), null);
    if (registryAll) {
      if (Array.isArray(registryAll)) return registryAll;
      return Object.keys(registryAll).map(k => registryAll[k]).filter(Boolean);
    }
    const core = safe(() => sap.ui.getCore(), null);
    const mElements = safe(() => core && core.mElements, null);
    if (mElements) return Object.keys(mElements).map(k => mElements[k]).filter(Boolean);
    return [];
  }

  function readControl(ctrl) {
    const id = safe(() => ctrl.getId(), null);
    if (!id) return null;
    const type = metadataName(ctrl);
    const el = safe(() => ctrl.getDomRef && ctrl.getDomRef(), null) || document.getElementById(id);
    const dom = domRef(el);
    const visible = isFn(ctrl, "getVisible") ? boolCall(ctrl, "getVisible") : (el ? isVisibleDom(el) : null);
    if (!includeHidden && visible === false) return null;
    const text = stringCall(ctrl, "getText") || stringCall(ctrl, "getLabel") || compactText(el && el.innerText);
    const value = stringCall(ctrl, "getValue") || stringCall(ctrl, "getSelectedKey");
    const interactor = inferInteractor(type, dom, text || value);
    const bindings = bindingInfo(ctrl);
    const aggs = aggregationInfo(ctrl);
    const models = modelInfo(ctrl);
    const parentId = safe(() => ctrl.getParent() && ctrl.getParent().getId(), null);
    const childIds = Array.from(new Set(aggs.flatMap(a => a.child_ids || [])));
    const selectors = selectorCandidates(id, dom, text || value);
    const riskFlags = [];
    if (id && /__xmlview|__component|application-/.test(id)) riskFlags.push("generated_or_framework_id_possible");
    if (!selectors.length) riskFlags.push("no_selector_candidate");
    if (!bindings.length && !models.length && !dom) riskFlags.push("low_semantic_information");

    return {
      id: id,
      control_type: type,
      short_type: shortType(type),
      visible: visible,
      enabled: isFn(ctrl, "getEnabled") ? boolCall(ctrl, "getEnabled") : null,
      editable: isFn(ctrl, "getEditable") ? boolCall(ctrl, "getEditable") : null,
      selected: isFn(ctrl, "getSelected") ? boolCall(ctrl, "getSelected") : null,
      busy: isFn(ctrl, "getBusy") ? boolCall(ctrl, "getBusy") : null,
      text: text,
      title: stringCall(ctrl, "getTitle"),
      value: value,
      tooltip: stringCall(ctrl, "getTooltip"),
      selected_key: stringCall(ctrl, "getSelectedKey"),
      binding_context_path: contextPath(ctrl),
      parent_id: parentId,
      child_ids: childIds,
      aggregations: aggs,
      bindings: bindings,
      models: models,
      dom: dom,
      selector_candidates: selectors,
      interactor: interactor,
      confidence: type ? 0.92 : 0.50,
      risk_flags: riskFlags
    };
  }

  function readDomNodes() {
    if (!includeDomNodes) return [];
    const nodes = [];
    const els = Array.prototype.slice.call(document.querySelectorAll("body *")).slice(0, maxDomNodes);
    els.forEach(function (el, idx) {
      const attrs = {};
      ["id", "role", "aria-label", "aria-labelledby", "aria-describedby", "title", "type", "name", "href", "value", "data-sap-ui", "data-sap-ui-related", "data-sap-ui-fastnavgroup"].forEach(function (a) {
        const v = el.getAttribute && el.getAttribute(a);
        if (v !== null && v !== undefined && v !== "") attrs[a] = String(v);
      });
      const id = el.id || null;
      const tag = el.tagName ? el.tagName.toLowerCase() : "unknown";
      const role = el.getAttribute ? el.getAttribute("role") : null;
      const text = compactText(el.innerText || el.textContent);
      const d = domRef(el);
      nodes.push({
        node_id: "dom_" + idx,
        tag: tag,
        id: id,
        role: role,
        text: text,
        aria_label: el.getAttribute ? el.getAttribute("aria-label") : null,
        title: el.getAttribute ? el.getAttribute("title") : null,
        classes: classList(el),
        attributes: attrs,
        selector_candidates: selectorCandidates(id, d, text),
        semantic: null,
        rect: rectOf(el)
      });
    });
    return nodes;
  }

  function endpointsFromPerformance() {
    if (!includePerformanceUrls || !performance || !performance.getEntriesByType) return [];
    const urls = new Set();
    performance.getEntriesByType("resource").forEach(function (e) {
      const n = e.name || "";
      if (/\/sap\/opu\/odata|\/odata4\//i.test(n)) urls.add(n);
    });
    return Array.from(urls).map(function (u) {
      return { url: u, service_root: inferServiceRoot(u), entity_or_path: null, source: "performance_entry" };
    });
  }

  function inferServiceRoot(u) {
    const lower = u.toLowerCase();
    const patterns = ["/sap/opu/odata/sap/", "/sap/opu/odata4/", "/odata4/"];
    for (const p of patterns) {
      const pos = lower.indexOf(p);
      if (pos >= 0) {
        const start = pos + p.length;
        const rest = u.slice(start);
        const service = rest.split(/[/?#]/)[0];
        if (service) return u.slice(0, start) + service;
      }
    }
    return null;
  }

  function appInfo() {
    const hash = location.hash || null;
    const m = hash && hash.match(/^#([^/-]+)-([^?&/]+)/);
    let manifestSapApp = null;
    let componentId = null;
    let componentName = null;
    safe(() => {
      const comps = sap && sap.ui && sap.ui.core && sap.ui.core.Component && sap.ui.core.Component.registry && sap.ui.core.Component.registry.all && sap.ui.core.Component.registry.all();
      const arr = comps ? Object.keys(comps).map(k => comps[k]) : [];
      const comp = arr[0];
      if (comp) {
        componentId = comp.getId && comp.getId();
        componentName = comp.getMetadata && comp.getMetadata().getName && comp.getMetadata().getName();
        manifestSapApp = comp.getManifestEntry && comp.getManifestEntry("/sap.app");
      }
    }, null);
    return {
      hash: hash,
      semantic_object: m ? m[1] : null,
      action: m ? m[2] : null,
      component_id: componentId,
      component_name: componentName,
      manifest_sap_app: manifestSapApp
    };
  }

  function runtimeInfo() {
    const detected = !!(window.sap && sap.ui);
    return {
      detected: detected,
      version: safe(() => sap.ui.version, null) || safe(() => sap.ui.getVersionInfo().version, null),
      bootstrapped: detected,
      core_initialized: safe(() => sap.ui.getCore().isInitialized(), null),
      libraries: safe(() => Object.keys(sap.ui.getCore().getLoadedLibraries()), [])
    };
  }

  const ui5 = runtimeInfo();
  const rawElements = getAllElements().slice(0, maxControls);
  const controls = rawElements.map(readControl).filter(Boolean);
  const domNodes = readDomNodes();
  const endpoints = endpointsFromPerformance();
  const seenEndpoints = new Set(endpoints.map(e => e.url));
  controls.forEach(c => (c.models || []).forEach(m => {
    if (m.service_url && !seenEndpoints.has(m.service_url)) {
      endpoints.push({ url: m.service_url, service_root: inferServiceRoot(m.service_url), entity_or_path: null, source: "model_service_url" });
      seenEndpoints.add(m.service_url);
    }
  }));

  const actionHints = controls
    .filter(c => c.interactor && c.selector_candidates && c.selector_candidates.length)
    .map(c => ({
      label: c.text || c.title || c.value || c.tooltip || c.id,
      kind: c.interactor,
      selector: c.selector_candidates[0],
      control_id: c.id,
      confidence: c.confidence || 0.8,
      rationale: "Control SAPUI5 accionable detectado desde el árbol lógico UI5 y DOM renderizado."
    }));

  const snapshot = {
    schema_version: "fiori-dom-agent.snapshot.v1",
    captured_at: nowIso(),
    mode: "browser_ui5",
    url: location.href,
    title: document.title || null,
    application: appInfo(),
    metrics: {
      control_count: controls.length,
      dom_node_count: domNodes.length,
      endpoint_count: endpoints.length,
      visible_control_count: controls.filter(c => c.visible !== false).length,
      actionable_control_count: actionHints.length
    },
    ui5: ui5,
    controls: controls,
    dom_nodes: domNodes,
    odata_endpoints: endpoints,
    action_hints: actionHints,
    warnings: []
  };

  if (!ui5.detected) snapshot.warnings.push("No se detectó SAPUI5 en window.sap.ui. Puede ser una página no Fiori, estar antes del login, o bloquear ejecución de JavaScript.");
  if (!controls.length && ui5.detected) snapshot.warnings.push("SAPUI5 detectado, pero no se obtuvieron controles. Espera más tiempo o revisa si la app está en iframe.");
  return JSON.stringify(snapshot);
})(arguments[0] || {});
