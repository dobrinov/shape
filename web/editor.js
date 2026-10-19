/* diagram editor component.
 *
 *   var ed = DiagramEditor.create(element, {
 *     wasm,               // the wasm_bindgen namespace (has `Diagram`)
 *     source,             // DSL, JSON or Mermaid
 *     mode: "edit",       // or "view" (pan, zoom, comments, export only)
 *     theme: null,        // "light" | "dark" | null (follow the OS)
 *     author: "deyan",    // name used for new comments
 *     code: false,        // the code pane (edit mode); off for now
 *     panel: false,       // the inspector / comments panel; off for now
 *     shareUrl: null,     // playground URL, enables "Copy share link"
 *     ai: null,           // function (prompt) → Promise<string>: your model / harness.
 *                         // Without it, ✦ actions show the prompt to copy and take the reply.
 *     openBoard: null,    // function (path, id): follow a `board.shape#id` link (else a new tab)
 *     boards: null,       // function () → Promise<[path]>: boards to offer in "Link to a board…"
 *     onChange: function (source, ed) {},
 *     onSave: function (source, ed) {}   // Cmd/Ctrl+S
 *   });
 *
 *   ed.getSource(fmt)  ed.setSource(src)  ed.apply(ops)  ed.getDocument()
 *   ed.setBase(src|null)  ed.prompt(opts)  ed.brief()  ed.exportSVG()  ed.exportPNG(scale)
 *   ed.select(ids)  ed.goTo(id)  ed.fit()  ed.setTheme(t)  ed.addButton({label, title, onClick})  ed.on(evt, fn)  ed.destroy()
 *
 * Everything that changes the diagram goes through the core's ops API, so the
 * code pane, the canvas, undo and LLM replies all stay in step.
 */
(function (root) {
  "use strict";

  var SHAPES = ["box", "rounded", "pill", "circle", "diamond", "hexagon", "cylinder", "parallelogram", "document", "person", "cloud", "sticky", "text"];
  var COLORS = ["gray", "blue", "green", "yellow", "orange", "red", "purple", "pink", "teal"];
  var SWATCH = { gray: "#868e96", blue: "#3b6fd8", green: "#2f9e44", yellow: "#e0b000", orange: "#e8590c", red: "#e03131", purple: "#7048e8", pink: "#d6336c", teal: "#0c8599" };
  var MAC = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
  var MOD = MAC ? "⌘" : "Ctrl+";

  var ICON = {
    select: '<path transform="scale(1.5)" d="M3.5 2.2v10.6l2.9-2.7 1.9 4.2 1.8-.8-1.9-4.1h3.9z" fill="currentColor" stroke-width=".55"/>',
    hand: '<path transform="scale(1.5)" d="M5.2 8.6V3.4a1 1 0 0 1 2 0V7.4m0-.4V2.6a1 1 0 0 1 2 0V7m0-.2V3.4a1 1 0 0 1 2 0V8m0-2.6a1 1 0 0 1 2 0v4.2c0 2.6-1.9 4.6-4.4 4.6-1.8 0-2.9-.8-3.9-2.3L2.6 9.3a1 1 0 0 1 1.6-1.2l1 1.1" stroke-width=".8"/>',
    node: '<rect x="4" y="6" width="16" height="12" rx="2"/>',
    sticky: '<path d="M5 4h14v10l-5 6H5z"/><path d="M14 20v-6h5"/>',
    mindmap: '<path d="M15.4 21.5v-3.4c2.7-1.4 4.4-4.1 4.4-7.2A8 8 0 0011.7 3c-4 0-7.2 2.9-7.7 6.7L2.7 13a.6.6 0 00.5.9h1.3v2.4a2.3 2.3 0 002.3 2.3h1.6v2.9"/><g stroke-width="1.4"><circle cx="12.2" cy="10.6" r="1.5"/><path d="M12.2 9.1V6.9M13.5 9.8l2-1.1M13.4 11.6l1.7 1.3M11 11.6l-1.8 1.4"/></g><circle cx="12.2" cy="5.9" r="1.2" fill="currentColor" stroke="none"/><circle cx="16.4" cy="8.2" r="1.2" fill="currentColor" stroke="none"/><circle cx="16" cy="13.6" r="1.2" fill="currentColor" stroke="none"/><circle cx="8.4" cy="13.6" r="1.2" fill="currentColor" stroke="none"/>',
    connect: '<path d="M5 19L19 5"/><path d="M11 5h8v8"/>',
    comment: '<path d="M4 5h16v11H9l-5 4z"/>',
    undo: '<path d="M9 7L4 12l5 5"/><path d="M4 12h11a5 5 0 010 10h-3"/>',
    redo: '<path d="M15 7l5 5-5 5"/><path d="M20 12H9a5 5 0 000 10h3"/>',
    code: '<path d="M8 7l-5 5 5 5"/><path d="M16 7l5 5-5 5"/>',
    panel: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/>',
    theme: '<circle cx="12" cy="12" r="8"/><path d="M12 4a8 8 0 000 16z" fill="currentColor"/>',
    tidy: '<path d="M4 6h7M4 12h11M4 18h16"/>',
    decision: '<path d="M12 3l9 9-9 9-9-9z"/>',
    present: '<rect x="3.5" y="5" width="17" height="12" rx="2"/><path d="M12 17v3M8.5 20h7M10.5 9v4l3.5-2z"/>',
    schema: '<ellipse cx="9" cy="5" rx="6" ry="2.2"/><path d="M3 5v9c0 1.2 2.7 2.2 6 2.2M15 5v4"/><rect x="11" y="11" width="10" height="9" rx="1.5"/><path d="M11 14.5h10"/>',
    entity: '<ellipse cx="12" cy="6" rx="7" ry="2.6"/><path d="M5 6v12c0 1.4 3.1 2.6 7 2.6s7-1.2 7-2.6V6M5 12c0 1.4 3.1 2.6 7 2.6s7-1.2 7-2.6"/>',
    table: '<rect x="4" y="5" width="16" height="14" rx="2"/><path d="M4 5h16v4.5H4z" fill="currentColor" stroke="none"/><path d="M4 14.25h16M10 9.5V19M15 9.5V19"/>',
    chart: '<path d="M4 20h16"/><rect x="6" y="11" width="3" height="7" rx="0.5" fill="currentColor" stroke="none"/><rect x="11" y="6" width="3" height="12" rx="0.5" fill="currentColor" stroke="none"/><rect x="16" y="9" width="3" height="9" rx="0.5" fill="currentColor" stroke="none"/>',
    risks: '<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M4 12h16M12 4v16"/><circle cx="16" cy="8" r="2" fill="currentColor" stroke="none"/><circle cx="8.5" cy="15.5" r="1.6" fill="currentColor" stroke="none"/>',
    gantt: '<path d="M4 4v16h16"/><rect x="7" y="6.5" width="7" height="3" rx="1" fill="currentColor" stroke="none"/><rect x="11" y="11" width="8" height="3" rx="1" fill="currentColor" stroke="none"/><rect x="9" y="15.5" width="5" height="3" rx="1" fill="currentColor" stroke="none"/>',
    adr: '<rect x="5" y="3" width="14" height="18" rx="2"/><path d="M8 8h8M8 12h8M8 16h5"/><circle cx="17" cy="17" r="3.2" fill="currentColor" stroke="none"/>',
    sequence: '<rect x="3" y="3" width="6" height="4" rx="1"/><rect x="15" y="3" width="6" height="4" rx="1"/><path d="M6 7v14M18 7v14" stroke-dasharray="2 2"/><path d="M6 11h11M15 9l2 2-2 2M18 16H7M9 14l-2 2 2 2"/>',
    lanes: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9.5h18M3 14.5h18M8 4v16"/>',
    kanban: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16M15 4v16"/><rect x="4.8" y="7" width="2.4" height="2.4" rx=".6" fill="currentColor" stroke="none"/><rect x="4.8" y="11" width="2.4" height="2.4" rx=".6" fill="currentColor" stroke="none"/><rect x="10.8" y="7" width="2.4" height="2.4" rx=".6" fill="currentColor" stroke="none"/><rect x="16.8" y="7" width="2.4" height="2.4" rx=".6" fill="currentColor" stroke="none"/>',
    section: '<rect x="3" y="8" width="18" height="12" rx="2"/><rect x="3" y="3" width="9" height="3.5" rx="1.2" fill="currentColor" stroke="none"/>',
    fit: '<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>',
    setup: '<path d="M9 3v4M15 3v4"/><path d="M6.5 7h11v3.5a5.5 5.5 0 01-11 0z"/><path d="M12 16v5"/>',
    minimap: '<rect x="3.5" y="5" width="17" height="14" rx="2"/><rect x="11" y="10" width="6" height="5" rx="1"/>',
    magic: '<path d="M10 3.5c.5 3.9 2.6 6 6.5 6.5-3.9.5-6 2.6-6.5 6.5-.5-3.9-2.6-6-6.5-6.5 3.9-.5 6-2.6 6.5-6.5z" fill="currentColor" stroke="none"/><path d="M18 13.5c.25 1.9 1.1 2.75 3 3-1.9.25-2.75 1.1-3 3-.25-1.9-1.1-2.75-3-3 1.9-.25 2.75-1.1 3-3z" fill="currentColor" stroke="none"/>',
    history: '<path d="M4 12a8 8 0 102.3-5.6"/><path d="M4 4.5v3.5h3.5"/><path d="M12 8v4.5l3 2"/>',
    branch: '<circle cx="6" cy="5" r="2.2"/><circle cx="6" cy="19" r="2.2"/><circle cx="18" cy="8" r="2.2"/><path d="M6 7.2v9.6M18 10.2c0 4-6 3.5-11 6.5"/>',
    blocks: '<rect x="3.5" y="3.5" width="7" height="7" rx="1.5"/><rect x="13.5" y="3.5" width="7" height="7" rx="1.5"/><rect x="3.5" y="13.5" width="7" height="7" rx="1.5"/><rect x="13.5" y="13.5" width="7" height="7" rx="1.5"/>',
    screen: '<rect x="7" y="2.5" width="10" height="19" rx="2.5"/><path d="M10.5 18.5h3"/>',
    todo: '<rect x="3.5" y="4.5" width="5" height="5" rx="1.2"/><path d="M4.8 7l1 1 1.6-1.8"/><rect x="3.5" y="14.5" width="5" height="5" rx="1.2"/><path d="M12 7h8.5M12 17h8.5"/>',
    tablet: '<rect x="4" y="2.5" width="16" height="19" rx="2.5"/><path d="M11 18.5h2"/>',
    flow: '<rect x="2.5" y="5" width="6" height="11" rx="1.5"/><rect x="15.5" y="8" width="6" height="11" rx="1.5"/><path d="M8.5 10.5h4l-1.5-1.5M12.5 10.5l-1.5 1.5"/>',
    page: '<rect x="2.5" y="4" width="19" height="16" rx="2"/><path d="M2.5 8h19M5 6h.01M7 6h.01"/>',
    cycle: '<path d="M19.5 12a7.5 7.5 0 01-12.8 5.3M4.5 12a7.5 7.5 0 0112.8-5.3"/><path d="M17.6 3.6v3.3h-3.3M6.4 20.4v-3.3h3.3"/>',
    timeline: '<path d="M4.5 5H16a3.5 3.5 0 010 7H8a3.5 3.5 0 000 7h11.5"/><circle cx="4.5" cy="5" r="2.7" fill="currentColor" stroke="none"/><circle cx="19.5" cy="19" r="2.7" fill="currentColor" stroke="none"/><circle cx="10.5" cy="5" r="2" fill="currentColor" stroke="none"/><circle cx="12" cy="12" r="2" fill="currentColor" stroke="none"/><circle cx="13.5" cy="19" r="2" fill="currentColor" stroke="none"/>',
    ask: '<path d="M4 5.5A2.5 2.5 0 016.5 3h11A2.5 2.5 0 0120 5.5v8a2.5 2.5 0 01-2.5 2.5H10l-4.5 4v-4h0A1.5 1.5 0 014 14.5z"/><path d="M10 7.8a2 2 0 113 1.7c-.7.4-1 .8-1 1.5"/><circle cx="12" cy="13.2" r=".6" fill="currentColor"/>',
    rethink: '<rect x="3" y="4" width="7" height="6" rx="1.5"/><rect x="14" y="14" width="7" height="6" rx="1.5"/><path d="M14 7h3.5a2 2 0 012 2v2M10 17H6.5a2 2 0 01-2-2v-2"/><path d="M17.5 9.5l2 2 2-2M6.5 14.5l-2-2-2 2"/>',
    add: '<path d="M5 4h14v11l-5 5H5z"/><path d="M12 8v7M8.5 11.5h7"/>'
  };
  function icon(name) {
    return '<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">' + ICON[name] + "</svg>";
  }

  function h(tag, attrs, kids) {
    var el = document.createElement(tag);
    if (attrs) Object.keys(attrs).forEach(function (k) {
      var v = attrs[k];
      if (v == null || v === false) return;
      if (k === "class") el.className = v;
      else if (k === "html") el.innerHTML = v;
      else if (k === "text") el.textContent = v;
      else if (k.slice(0, 2) === "on") el.addEventListener(k.slice(2), v);
      else el.setAttribute(k, v === true ? "" : v);
    });
    (kids || []).forEach(function (c) { if (c != null) el.appendChild(typeof c === "string" ? document.createTextNode(c) : c); });
    return el;
  }

  function slug(s) {
    var out = String(s).toLowerCase().normalize("NFKD").replace(/[\u0300-\u036f]/g, "").replace(/[^\p{L}\p{N}_]+/gu, "-").replace(/^-+|-+$/g, "");
    return out || "node";
  }

  function download(name, data, type) {
    var blob = data instanceof Blob ? data : new Blob([data], { type: type || "text/plain" });
    var a = h("a", { href: URL.createObjectURL(blob), download: name });
    document.body.appendChild(a);
    a.click();
    setTimeout(function () { URL.revokeObjectURL(a.href); a.remove(); }, 1000);
  }

  function copyText(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) return navigator.clipboard.writeText(text);
    var ta = h("textarea", { style: "position:fixed;opacity:0" });
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    try { document.execCommand("copy"); } finally { ta.remove(); }
    return Promise.resolve();
  }

  function b64url(bytes) {
    var s = "";
    for (var i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
    return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  }

  /* Encode a diagram for a URL hash: deflate when the browser can. */
  function encodeShare(text) {
    var bytes = new TextEncoder().encode(text);
    if (typeof CompressionStream === "undefined") return Promise.resolve("t=" + b64url(bytes));
    var stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream("deflate-raw"));
    return new Response(stream).arrayBuffer().then(function (buf) { return "z=" + b64url(new Uint8Array(buf)); });
  }

  function decodeShare(hash) {
    var m = /(?:^|[#&])([zt])=([A-Za-z0-9_-]+)/.exec(hash || "");
    if (!m) return Promise.resolve(null);
    var bin = atob(m[2].replace(/-/g, "+").replace(/_/g, "/"));
    var bytes = new Uint8Array(bin.length);
    for (var i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    if (m[1] === "t") return Promise.resolve(new TextDecoder().decode(bytes));
    var stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
    return new Response(stream).text();
  }

  function create(el, opts) {
    opts = opts || {};
    if (!opts.wasm || !opts.wasm.Diagram) throw new Error("DiagramEditor: pass the initialised wasm module as `wasm`");
    var wasm = opts.wasm;
    var view = opts.mode === "view";
    var listeners = {};
    var st = {
      session: new wasm.Diagram(),
      r: null, doc: null, text: "", errors: [],
      zoom: 1, panX: 0, panY: 0, fitted: false,
      tool: "select", sel: [], hover: null, comment: null, composer: null, thinking: {},
      undo: [], redo: [], codeFormat: "dsl",
      dark: opts.theme ? opts.theme === "dark" : !!(root.matchMedia && root.matchMedia("(prefers-color-scheme: dark)").matches),
      showResolved: false, base: null, space: false,
      author: opts.author || lsGet("diagram.author") || ""
    };
    // theme files the board names (`theme: ./brand.theme`), read by the host
    Object.keys(opts.themeFiles || {}).forEach(function (name) {
      var r = JSON.parse(st.session.add_theme_file(name, opts.themeFiles[name]));
      if (r.error) setTimeout(function () { flash(r.error, true); });
    });

    function lsGet(k) { try { return localStorage.getItem(k); } catch (e) { return null; } }
    function lsSet(k, v) { try { localStorage.setItem(k, v); } catch (e) { /* private mode */ } }
    function emit(name, arg) { (listeners[name] || []).forEach(function (fn) { fn(arg, api); }); }

    // ------------------------------------------------------------- layout
    el.innerHTML = "";
    el.classList.add("dge");
    if (view) el.classList.add("dge-view");
    var toolbar = h("div", { class: "dge-toolbar", role: "toolbar" });
    var codeTabs = h("div", { class: "dge-tabs" });
    var gutter = h("div", { class: "dge-gutter", "aria-hidden": "true" });
    var textarea = h("textarea", { class: "dge-text", spellcheck: "false", autocapitalize: "off", autocomplete: "off", "aria-label": "Diagram source" });
    var errorsEl = h("div", { class: "dge-errors" });
    var codePane = h("div", { class: "dge-code" }, [codeTabs, h("div", { class: "dge-code-wrap" }, [gutter, textarea]), errorsEl]);
    var stage = h("div", { class: "dge-stage" });
    var overlay = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    overlay.setAttribute("class", "dge-overlay");
    var zoomLabel = h("button", { class: "dge-zoom-label", title: "Reset zoom (0)", onclick: function () { zoomTo(1); } }, ["100%"]);
    var zoomBar = h("div", { class: "dge-zoombar" }, [
      h("button", { title: "Zoom out (-)", onclick: function () { zoomTo(st.zoom / 1.2); } }, ["−"]),
      zoomLabel,
      h("button", { title: "Zoom in (+)", onclick: function () { zoomTo(st.zoom * 1.2); } }, ["+"]),
      h("button", { title: "Fit (F)", html: icon("fit"), onclick: function () { fit(); } }),
      h("button", { class: "dge-minimap-btn", title: "Minimap (M)", html: icon("minimap"), onclick: function () { toggleMinimap(); } })
    ]);
    /* The minimap: the whole board small, with the part in view marked;
       click or drag in it to move around. */
    var MM_W = 200, MM_H = 132;
    var mmBoard = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    mmBoard.setAttribute("width", MM_W);
    mmBoard.setAttribute("height", MM_H);
    var mmView = h("div", { class: "dge-minimap-view" });
    var minimap = h("div", { class: "dge-minimap", title: "Minimap: click or drag to move around", hidden: true }, [mmBoard, mmView]);
    var mmScale = function () {
      var v = st.r.view, s = Math.min(MM_W / v.w, MM_H / v.h);
      return { s: s, ox: (MM_W - v.w * s) / 2, oy: (MM_H - v.h * s) / 2, v: v };
    };
    function drawMinimap() {
      // not in a small editor (an embedded board): there it covers the board
      var on = minimapOn() && st.r && st.doc && st.doc.nodes.length > 0 && !st.presenting && canvas.clientWidth >= 900;
      minimap.hidden = !on;
      zoomBar.querySelector(".dge-minimap-btn").setAttribute("aria-pressed", String(minimapOn()));
      if (!on) return;
      // as tall as the board's shape needs, within limits
      MM_H = Math.round(Math.max(64, Math.min(160, MM_W * st.r.view.h / st.r.view.w)));
      mmBoard.setAttribute("height", MM_H);
      minimap.style.height = MM_H + "px";
      var m = mmScale(), out = "";
      var box = function (b, cls) {
        return '<rect class="' + cls + '" x="' + (m.ox + (b.x - m.v.x) * m.s).toFixed(1) + '" y="' + (m.oy + (b.y - m.v.y) * m.s).toFixed(1) + '" width="' + Math.max(1.5, b.w * m.s).toFixed(1) + '" height="' + Math.max(1.5, b.h * m.s).toFixed(1) + '" rx="' + (cls === "g" ? 2 : 1) + '"/>';
      };
      (st.r.groups || []).forEach(function (g) { out += box(g, "g"); });
      st.r.nodes.forEach(function (n) { out += box(n, "n"); });
      mmBoard.innerHTML = out;
      placeMinimapView();
    }
    function placeMinimapView() {
      if (minimap.hidden || !st.r) return;
      var m = mmScale();
      var x0 = m.v.x - st.panX / st.zoom, y0 = m.v.y - st.panY / st.zoom;
      var x = m.ox + (x0 - m.v.x) * m.s, y = m.oy + (y0 - m.v.y) * m.s;
      var w = canvas.clientWidth / st.zoom * m.s, hh = canvas.clientHeight / st.zoom * m.s;
      // clipped to the minimap
      var l = Math.max(0, x), t = Math.max(0, y), r = Math.min(MM_W, x + w), b = Math.min(MM_H, y + hh);
      mmView.style.cssText = "left:" + l + "px;top:" + t + "px;width:" + Math.max(0, r - l) + "px;height:" + Math.max(0, b - t) + "px";
    }
    function minimapGo(e) {
      var rect = mmBoard.getBoundingClientRect(), m = mmScale();
      var wx = m.v.x + (e.clientX - rect.left - m.ox) / m.s, wy = m.v.y + (e.clientY - rect.top - m.oy) / m.s;
      st.panX = canvas.clientWidth / 2 - (wx - m.v.x) * st.zoom;
      st.panY = canvas.clientHeight / 2 - (wy - m.v.y) * st.zoom;
      transform();
    }
    minimap.addEventListener("pointerdown", function (e) {
      e.preventDefault();
      e.stopPropagation();
      minimap.setPointerCapture(e.pointerId);
      minimapGo(e);
      var move = function (ev) { minimapGo(ev); };
      var up = function () { minimap.removeEventListener("pointermove", move); minimap.removeEventListener("pointerup", up); canvas.focus({ preventScroll: true }); };
      minimap.addEventListener("pointermove", move);
      minimap.addEventListener("pointerup", up);
    });
    function minimapOn() { return lsGet("diagram.minimap") !== "off"; }
    function toggleMinimap() { lsSet("diagram.minimap", minimapOn() ? "off" : "on"); drawMinimap(); }
    var hint = h("div", { class: "dge-hint" });
    /* The AI: a round button in the corner opens what it can do. */
    var aiItem = function (ico, name, more, tip, key, fn) {
      return h("button", { class: "dge-ai-item", title: tip, onclick: function () { closeAiMenu(); fn(); } }, [
        h("span", { class: "dge-ai-ico", html: icon(ico) }),
        h("span", { class: "dge-ai-text" }, [h("b", {}, [name]), h("span", {}, [more])]),
        key ? h("kbd", {}, [key]) : null
      ].filter(Boolean));
    };
    var aiMenu = h("div", { class: "dge-ai-menu", role: "menu", hidden: true }, [
      aiItem("ask", "Ask", "Questions answered with a diagram drawn for them",
        "Ask: a question about this board (all of it, canvases included), such as what blocks the launch or who owns what. The AI draws the answer on a temporary canvas, each piece linked to its place on the board. Save it as a canvas, comment on it, or rethink the board from it. Nothing changes until you do.",
        "", function () { openAsk(); }),
      aiItem("rethink", "Rethink", "Act on the comments and bring the board up to date",
        "Rethink: the AI reads the whole board, every canvas included. It acts on the open comments first (changing the board everywhere a comment affects it, then resolving the comment), then organises: related items in sections, duplicates merged, stale items updated. In git, what changed on this branch counts as new information too. Nothing is lost, and you see the changes before keeping them.",
        "", function () { rethink(); }),
      opts.ai ? null : aiItem("setup", "Connect your AI", "None is connected: set up the one you pay for",
        "Connect your AI: diagram uses the AI subscription you already have (Claude, ChatGPT or Gemini) through its command-line tool. Until one is connected, the actions below show the prompt to paste into any chat.",
        "", function () { closeAiMenu(); setupAI(); }),
      aiItem("add", "Add what you know", "Details, facts or notes: the AI puts them in place",
        "Add what you know (" + MOD + "K): write down details or information you have, about the selection or the whole canvas; the AI adds them where they belong, or makes the change you ask for. You see the changes before keeping them.",
        MOD + "K", function () { askAI(); }),
      opts.ai && opts.aiSetup ? h("button", { class: "dge-ai-foot", title: "Which AI is used, and whether it works", onclick: function () { closeAiMenu(); setupAI(); } }, ["Your AI: check it, or change it…"]) : null
    ].filter(Boolean));
    var aiFab = h("button", { class: "dge-ai-fab", "aria-haspopup": "menu", "aria-label": "AI", title: "AI features: Rethink this canvas, or Add what you know. They run on your own AI subscription, and you see every change before keeping it.", html: icon("magic"), onclick: function () {
      if (aiMenu.hidden) { aiMenu.hidden = false; aiFab.classList.add("on"); fitPopup(aiMenu, true); } else closeAiMenu();
    } });
    function closeAiMenu() { aiMenu.hidden = true; aiFab.classList.remove("on"); }
    /* Ask: questions about the board; answers point at places on it. */
    var askThread = h("div", { class: "dge-ask-thread" });
    var askInput = h("textarea", { rows: "2", placeholder: "Ask about this board, e.g. what is still open?" });
    var askPanel = h("div", { class: "dge-ask-panel", role: "dialog", "aria-label": "Ask about this board", hidden: true }, [
      h("header", {}, [
        h("span", { class: "dge-ai-ico", html: icon("ask") }),
        h("b", {}, ["Ask about this board"]),
        h("button", { class: "dge-ask-close", title: "Close (Esc)", "aria-label": "Close", onclick: function () { closeAsk(); } }, ["×"])
      ]),
      askThread,
      h("div", { class: "dge-ask-box" }, [askInput, h("button", { class: "primary", title: "Ask (Enter; ⇧Enter for a new line)", onclick: function () { sendQuestion(); } }, ["Ask"])])
    ]);
    askInput.addEventListener("keydown", function (e) {
      e.stopPropagation();
      if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendQuestion(); }
      if (e.key === "Escape") { e.preventDefault(); closeAsk(); }
    });
    // floating tool bar, bottom centre (as in schema): pointer and hand, then the blocks
    var tools = h("div", { class: "dge-tools", role: "toolbar", "aria-label": "Tools" });
    [tools, zoomBar, aiFab, aiMenu, askPanel].forEach(function (bar) {
      ["pointerdown", "dblclick", "contextmenu"].forEach(function (ev) { bar.addEventListener(ev, function (e) { e.stopPropagation(); }); });
    });
    var canvas = h("div", { class: "dge-canvas", tabindex: "0" }, [stage, tools, zoomBar, minimap, hint].concat(view ? [] : [aiMenu, askPanel, aiFab]));
    var inspector = h("div", { class: "dge-inspector" });
    var commentsEl = h("div", { class: "dge-comments" });
    var panel = h("aside", { class: "dge-panel" }, [inspector, commentsEl]);
    var body = h("div", { class: "dge-body" }, [codePane, canvas, panel]);
    var toast = h("div", { class: "dge-toast", role: "status" });
    var menu = h("div", { class: "dge-menu", hidden: true });
    var modal = h("div", { class: "dge-modal", hidden: true });
    el.appendChild(toolbar);
    el.appendChild(body);
    el.appendChild(toast);
    el.appendChild(menu);
    var bubble = h("div", { class: "dge-bubble", hidden: true });
    el.appendChild(bubble);
    el.appendChild(modal);

    var narrow = el.clientWidth && el.clientWidth < 760;
    // both sidebars are opt-in for now
    var hasCode = opts.code === true, hasPanel = opts.panel === true;
    var showCode = !view && hasCode && !narrow;
    var showPanel = hasPanel && !narrow && !view;

    function applyChrome() {
      el.setAttribute("data-theme", st.dark ? "dark" : "light");
      codePane.hidden = !showCode;
      panel.hidden = !showPanel;
      tools.querySelectorAll("[data-tool]").forEach(function (b) { b.setAttribute("aria-pressed", String(b.getAttribute("data-tool") === st.tool)); });
      var cb = toolbar.querySelector("[data-act=code]");
      if (cb) cb.classList.toggle("on", showCode);
      var pb = toolbar.querySelector("[data-act=panel]");
      if (pb) pb.classList.toggle("on", showPanel);
      canvas.setAttribute("data-tool", st.tool);
      canvas.classList.toggle("zooming", st.tool === "zoom");
    }

    function btn(attrs, label) {
      var b = h("button", attrs);
      b.innerHTML = label;
      return b;
    }

    /* The blocks the Blocks palette offers: [icon, name, what it is, act]. */
    function blockKinds() {
      var go = function (f) { return function () { closeBlocks(); canvas.focus({ preventScroll: true }); f(); }; };
      return [
        ["Think", [
          ["mindmap", "Mindmap", "T · branch with Tab", go(function () { setTool("mindmap"); })],
          ["timeline", "Timeline", "What happened when", go(addTimeline)],
          ["cycle", "Cycle", "Steps that come round again", go(addCycle)],
          ["adr", "Decision record", "Context, options, choice", go(addRecord)],
          ["risks", "Risk matrix", "Likelihood × impact", go(addRisks)]
        ]],
        ["Plan", [
          ["todo", "To-do list", "Checkboxes, owners, due dates", go(addTodo)],
          ["kanban", "Kanban", "Cards in columns", go(addKanban)],
          ["gantt", "Gantt chart", "Tasks and forecast", go(addGantt)],
          ["table", "Table", "Rows and columns", go(addTable)],
          ["chart", "Chart", "Bars, lines, pie…", function (e) { closeBlocks(); chartMenu(blocksBtn); }]
        ]],
        ["Wireframes", [
          ["screen", "Phone", "A mobile screen", go(function () { addScreen("mobile"); })],
          ["page", "Web page", "A page in a browser", go(function () { addScreen("web"); })],
          ["tablet", "Tablet", "A tablet screen", go(function () { addScreen("tablet"); })],
          ["flow", "Screen flow", "Three screens, linked", go(addFlow)]
        ]],
        ["Specify", [
          ["sequence", "Sequence diagram", "Who talks to whom", go(addSequence)],
          ["entity", "Database table", "Columns and keys", go(addEntity)],
          ["schema", "Database schema", "Tables from structure.sql, schema.rb…", go(addSchema)],
          ["lanes", "Swimlanes", "Steps across teams", go(addSwimlanes)],
          ["decision", "Decision point", "D · a diamond", go(function () { setTool("decision"); })],
          ["section", "Section", MOD + "G · around the selection", go(groupSelection)],

          ["canvas", "Canvas", "A board inside this one", go(function () {
            var r = canvas.getBoundingClientRect();
            st.menuAt = { x: r.left + r.width / 2, y: r.top + r.height / 2 };
            newCanvas(toDiagram(st.menuAt.x, st.menuAt.y));
          })]
        ]]
      ];
    }
    var blocksBtn = null, blocksMenu = null, boardMenu = null;
    function closeBlocks() { if (blocksMenu) blocksMenu.hidden = true; if (blocksBtn) blocksBtn.classList.remove("on"); }
    function toggleBlocks() {
      if (!blocksMenu.hidden) return closeBlocks();
      closeAiMenu();
      blocksMenu.hidden = false;
      blocksBtn.classList.add("on");
      fitPopup(blocksMenu, true);
    }
    function buildBlocks() {
      blocksMenu = h("div", { class: "dge-blocks", role: "menu", "aria-label": "Add a block", hidden: true });
      blockKinds().forEach(function (sec) {
        blocksMenu.appendChild(h("div", { class: "dge-blocks-head" }, [sec[0]]));
        var grid = h("div", { class: "dge-blocks-grid" });
        sec[1].forEach(function (k) {
          grid.appendChild(h("button", { role: "menuitem", "data-block": k[0], onclick: k[3] }, [
            h("span", { class: "dge-blocks-ico", html: icon(k[0] === "canvas" ? "section" : k[0]) }),
            h("span", { class: "dge-blocks-text" }, [h("b", {}, [k[1]]), h("span", {}, [k[2]])])
          ]));
        });
        blocksMenu.appendChild(grid);
      });
      ["pointerdown", "dblclick", "contextmenu"].forEach(function (ev) { blocksMenu.addEventListener(ev, function (e) { e.stopPropagation(); }); });
      canvas.appendChild(blocksMenu);
    }
    /* Perspectives: one audience's view of the board (only what you see
       changes). The board's type decides the built-in ones; the file can
       add its own. The pick is kept per board in this browser. */
    function perspectiveKey() { return "diagram.perspective." + (opts.file || opts.name || "board"); }
    function fillPerspectives(force) {
      var sel = toolbar.persp;
      if (!sel || !st.session) return;
      if (toolbar.folds) {
        toolbar.folds.hidden = !st.perspective;
        toolbar.folds.textContent = st.perspOpen ? "Expand all" : "Fold all";
        toolbar.folds.title = st.perspOpen ? "Open every folded section and block" : "Back to the high-level view: sections and blocks folded";
      }
      var main = st.answer ? st.answer.main.session : st.session;
      var list = JSON.parse(main.perspectives());
      st.perspectives = list;
      var cur = st.perspective ? st.perspective.id : "";
      var key = JSON.stringify(list) + cur;
      if (sel._key === key) return;
      sel._key = key;
      sel.innerHTML = "";
      sel.appendChild(h("option", { value: "" }, ["Full board"]));
      list.forEach(function (p) { sel.appendChild(h("option", { value: p.id }, [p.label])); });
      if (!view) {
        sel.appendChild(h("option", { disabled: true }, ["──────────"]));
        sel.appendChild(h("option", { value: "__new" }, ["New perspective…"]));
        if (list.length) sel.appendChild(h("option", { value: "__sort" }, ["✦ Sort into perspectives"]));
      }
      sel.value = cur;
    }
    function pickPerspective(id) {
      if (id === "__new") { toolbar.persp.value = st.perspective ? st.perspective.id : ""; return newPerspective(); }
      if (id === "__sort") { toolbar.persp.value = st.perspective ? st.perspective.id : ""; return sortPerspectives(); }
      st.perspective = (st.perspectives || []).find(function (p) { return p.id === id; }) || null;
      st.perspOpen = []; // it starts high level: everything folded
      try { if (id) localStorage.setItem(perspectiveKey(), id); else localStorage.removeItem(perspectiveKey()); } catch (e) { /* fine */ }
      fillPerspectives();
      render();
      fit();
    }
    function restorePerspective() {
      var id = null;
      try { id = localStorage.getItem(perspectiveKey()); } catch (e) { /* fine */ }
      fillPerspectives();
      if (id) st.perspective = (st.perspectives || []).find(function (p) { return p.id === id; }) || null;
      st.perspOpen = [];
      fillPerspectives();
    }
    /* A new type brings other perspectives: the board is rethought so each
       shows what its audience needs (after a warning). */
    var TYPE_NAMES = { sketch: "Sketch", product: "Product", project: "Project" };
    function changeType(kind) {
      var before = (st.answer ? st.answer.main.session : st.session).board_type();
      var set = function () { applyOps([{ op: "set", key: "type", value: kind }], false, true); fillPerspectives(); };
      if (kind === "sketch") return set(); // a sketch has no perspectives to fill
      boardMenu.hidden = true;
      if (toolbar.kind) toolbar.kind.value = before; // until confirmed
      var names = JSON.parse(wasm.builtin_perspectives(kind));
      showModal("Make this a " + TYPE_NAMES[kind] + " board?", h("div", { class: "dge-ai-setup" }, [
        h("p", {}, ["A " + TYPE_NAMES[kind].toLowerCase() + " board is seen by " + (names.length ? names.join(", ") : "its own audiences") + ". Each of these perspectives shows only what is tagged for it, so the board needs sorting for them."]),
        h("p", {}, [h("b", {}, ["The AI will rethink the board"]), " for its new perspectives: it tags every element for the audiences it matters to, and may reorganise sections. You see the changes before keeping them."])
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { title: "Change the type and leave the board as it is", onclick: function () { closeModal(); set(); } }, ["Only change the type"]),
        h("button", { class: "primary", onclick: function () {
          closeModal();
          set();
          if (st.preview) return flash("Keep or discard the AI's changes first, then Rethink", true);
          runRethink();
        } }, ["Change and rethink"])
      ]);
    }
    /* Opening a board with perspectives: whose eyes to look through. */
    function askPerspective() {
      var list = st.perspectives || [];
      if (!list.length || view || !modal.hidden) return;
      var last = null;
      try { last = localStorage.getItem(perspectiveKey()); } catch (e) { /* fine */ }
      var pick = function (id) { closeModal(); toolbar.persp.value = id; pickPerspective(id); };
      var card = function (id, label, more) {
        return h("button", { class: "dge-persp-card" + ((last || "") === id ? " on" : ""), onclick: function () { pick(id); } }, [
          h("b", {}, [label]), h("span", {}, [more + ((last || "") === id ? " · last time" : "")])
        ]);
      };
      showModal("How would you like to view this board?", h("div", {}, [
        h("p", { class: "dge-muted" }, ["Each perspective shows the part of the board one audience needs. Switch any time from the top bar."]),
        h("div", { class: "dge-persp-cards" }, [card("", "Full board", "Everything on it")].concat(list.map(function (p) {
          // what it shows, briefly: its blocks (the first few), else its tag
          var show = (p.show || []).map(function (k) { return SHORT_KINDS[k] || k; }), tag = (p.tags || [])[0] || p.id;
          var more = wasm.perspective_purpose(p.id) || (show.length ? show.slice(0, 3).join(", ") + (show.length > 3 ? " and more" : "") : "What is tagged “" + tag + "”");
          return card(p.id, p.label, more.charAt(0).toUpperCase() + more.slice(1));
        })))
      ]), []);
    }
    var SHORT_KINDS = { todo: "to-dos", mindmap: "mindmaps", timeline: "timelines", adr: "decisions", risks: "risks", kanban: "kanban", gantt: "plans", table: "tables", chart: "charts", entity: "data model", sequence: "sequences", lanes: "swimlanes", sticky: "stickies", screen: "screens", cycle: "cycles" };
    var KIND_NAMES = { mindmap: "Mindmaps", timeline: "Timelines", adr: "Decision records", risks: "Risk matrices", kanban: "Kanban boards", gantt: "Gantt charts", table: "Tables", chart: "Charts", entity: "Database tables", sequence: "Sequence diagrams", lanes: "Swimlanes", sticky: "Stickies", screen: "Wireframe screens", todo: "To-do lists" };
    function newPerspective() {
      var name = h("input", { class: "dge-input", placeholder: "e.g. Legal, Support, Investors" });
      var tags = h("input", { class: "dge-input", placeholder: "e.g. legal" });
      var details = h("input", { type: "checkbox", checked: true });
      var kinds = Object.keys(KIND_NAMES).map(function (k) { return [k, h("input", { type: "checkbox", value: k })]; });
      var go = function () {
        var label = name.value.trim();
        if (!label) return name.focus();
        var id = slug(label) || "view", t = tags.value.split(/[\s,]+/).filter(Boolean);
        if (!t.length) t = [id];
        var res = applyOps([{ op: "perspective", id: id, label: label, tags: t, show: kinds.filter(function (k) { return k[1].checked; }).map(function (k) { return k[0]; }), details: details.checked }], false, true);
        if (!res.ok) return;
        closeModal();
        fillPerspectives();
        pickPerspective(id);
        flash("Elements tagged " + t[0] + " show in “" + label + "”; ✦ Sort into perspectives tags them for you");
      };
      showModal("New perspective", h("div", { class: "dge-persp-form" }, [
        h("label", {}, [h("span", {}, ["Name"]), name]),
        h("label", {}, [h("span", {}, ["Shows elements tagged"]), tags]),
        h("div", {}, [h("span", { class: "dge-muted" }, ["…and these blocks, whole:"]), h("div", { class: "dge-persp-kinds" }, kinds.map(function (k) { return h("label", {}, [k[1], KIND_NAMES[k[0]]]); }))]),
        h("label", { class: "dge-persp-check" }, [details, "Show details (off: titles only)"])
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: go }, ["Add"])
      ]);
      setTimeout(function () { name.focus(); }, 0);
    }
    /* A perspective that shows nothing: nothing on the board is tagged for
       it yet. Say so, and offer to sort the board, or the full board. */
    var empty = null;
    function emptyView(on) {
      if (!on) { if (empty) empty.hidden = true; return; }
      if (!empty) {
        // clicks go through to the board (to add a box), but for its buttons
        empty = h("div", { class: "dge-empty-view" });
        ["pointerdown", "dblclick", "contextmenu"].forEach(function (ev) { empty.addEventListener(ev, function (e) { if (e.target.closest("button")) e.stopPropagation(); }); });
        canvas.appendChild(empty);
      }
      empty.hidden = false;
      empty.innerHTML = "";
      empty.appendChild(h("b", {}, ["Nothing here is marked for " + st.perspective.label + " yet"]));
      empty.appendChild(h("p", {}, ["A perspective shows what is tagged for it and the blocks it cares about. The AI can go through the board and tag everything for the people who need it."]));
      empty.appendChild(h("div", { class: "dge-empty-actions" }, [
        h("button", { class: "primary", onclick: function () { sortPerspectives(); } }, ["✦ Sort the board into perspectives"]),
        h("button", { onclick: function () { toolbar.persp.value = ""; pickPerspective(""); } }, ["Show the full board"])
      ]));
    }
    /* A perspective starts high level: its sections and blocks folded into
       summary cards. A card opens in place; the bar's button opens or folds
       them all. */
    function unfold(gid) {
      if (!st.perspOpen) return;
      if (st.perspOpen.indexOf(gid) < 0) st.perspOpen.push(gid);
      render();
      fillPerspectives(true);
      var g = st.r.groups.find(function (x) { return x.id === gid; });
      if (g) focusOn(g, 1, 1.2, 0.9);
    }
    function toggleFolds() {
      st.perspOpen = st.perspOpen ? null : [];
      render();
      fillPerspectives(true);
      fit();
    }
    /* The AI tags each element with the perspectives it belongs to. */
    function sortPerspectives() {
      var ins = perspectiveInstructions();
      if (!ins) return flash("This board has no perspectives yet: set its type (Board) or add one", true);
      runRethink(ins);
    }
    function perspectiveInstructions() {
      var list = st.perspectives || [];
      if (!list.length) return null;
      var lines = list.map(function (p) { return "- " + p.label + ": tag `" + (p.tags[0] || p.id) + "`" + (p.show.length ? " (it already shows " + p.show.join(", ") + " blocks whole)" : ""); }).join("\n");
      return ("Sort the board into its perspectives, the audiences that look at it:\n\n" + lines + "\n\nAdd to each box's `tags` the tags of every perspective it matters to (`update_node` with the box's existing tags plus the new ones; a box can belong to several, and inside canvases too). Tag mindmap centres, not their topics. Change nothing else: no labels, sections or arrows. In the note, say how many elements each perspective now shows.");
    }
    /* AI prompts…: the prompts the AI buttons send, built for this board as
       it is now, to read (and give feedback on). Nothing is sent. */
    function showPrompts() {
      boardMenu.hidden = true;
      var dated = function (p) { return "Today is " + isoDay(today()) + ".\n\n" + p; };
      var since = opts.since ? Promise.resolve(opts.since()).catch(function () { return null; }) : Promise.resolve(null);
      since.then(function (v) {
        var rethinkWith = function (ins) {
          var r = JSON.parse(st.session.rethink_prompt(v ? v.source : undefined, v ? v.label : "", ins));
          return r.error ? "(" + r.error + ")" : dated(r.prompt);
        };
        var ins = perspectiveInstructions();
        var ids = st.sel.filter(kindOf);
        var tabs = [
          ["Rethink", "Sent by ✦ → Rethink: the whole board, every canvas" + (v ? ", and what is new since " + v.label : "") + ".", function () { return rethinkWith(undefined); }],
          ["Perspectives", "Sent by ✦ Sort into perspectives: Rethink, told to tag the board for its perspectives.", function () { return ins ? rethinkWith(ins) : "This board has no perspectives yet: set its type (Board) or add one."; }],
          ["Ask", "Sent by ✦ → Ask, with your question where it says “Your question”.", function () { return dated((st.answer ? st.answer.main.session : st.session).answer_board_prompt("[]", "Your question", isoDay(today()))); }],
          ["Add what you know", "Sent by ✦ → Add what you know, with " + (ids.length ? "the " + ids.length + " selected" : "the whole canvas (or what you select)") + " and what you write.", function () { return dated(st.session.ask_prompt(JSON.stringify(ids), "What you write")); }]
        ];
        var bar = h("div", { class: "dge-prompt-tabs", role: "tablist" });
        var note = h("p", { class: "dge-muted" });
        var text = h("textarea", { class: "dge-preview", readonly: true, rows: 22, "aria-label": "Prompt" });
        var pick = function (i) {
          Array.prototype.forEach.call(bar.children, function (b, j) { b.className = i === j ? "on" : ""; b.setAttribute("aria-selected", i === j); });
          note.textContent = tabs[i][1];
          text.value = tabs[i][2]();
          text.scrollTop = 0;
        };
        tabs.forEach(function (t, i) { bar.appendChild(h("button", { role: "tab", onclick: function () { pick(i); } }, [t[0]])); });
        showModal("AI prompts", h("div", { class: "dge-prompt" }, [
          h("p", { class: "dge-muted" }, ["What the AI buttons send for this board as it is now (nothing is sent from here)."]),
          bar, note, text
        ]), [
          h("button", { onclick: function () { copyText(text.value).then(function () { flash("Copied the prompt"); }); } }, ["Copy"]),
          h("button", { class: "primary", onclick: closeModal }, ["Close"])
        ]);
        pick(0);
      });
    }
    /* Board: flow direction, arrow routing, theme and tidy up, out of the bar. */
    function toggleBoardMenu(anchor) {
      if (boardMenu.hidden === false) { boardMenu.hidden = true; return; }
      var r = anchor.getBoundingClientRect(), host = el.getBoundingClientRect();
      boardMenu.hidden = false;
      boardMenu.style.top = (r.bottom - host.top + 6) + "px";
      fitPopup(boardMenu);
      boardMenu.style.left = Math.max(8, Math.min(r.right - host.left - boardMenu.offsetWidth, host.width - boardMenu.offsetWidth - 8)) + "px";
    }
    function buildToolbar() {
      toolbar.innerHTML = "";
      tools.innerHTML = "";
      tools.hidden = view;
      // the host's own things first (the app's name, its actions)
      if (opts.lead) toolbar.appendChild(h("div", { class: "dge-group dge-lead" }, [].concat(opts.lead)));
      // where you are: above the canvas, when you are inside one (or an answer)
      if (toolbar.crumbs) toolbar.crumbs.remove();
      toolbar.crumbs = h("nav", { class: "dge-crumbs", "aria-label": "Where you are", hidden: true });
      el.appendChild(toolbar.crumbs); // not in the canvas: its pointer handling would take the clicks
      toolbar.persp = h("select", { class: "dge-persp", title: "Perspective: show the board as one audience sees it (only changes what you see)", onchange: function (e) { pickPerspective(e.target.value); } });
      toolbar.appendChild(toolbar.persp);
      toolbar.folds = h("button", { class: "dge-bar-btn dge-folds", "data-act": "folds", hidden: true, onclick: function () { toggleFolds(); } });
      toolbar.appendChild(toolbar.folds);
      toolbar.branch = h("button", { class: "dge-branch", hidden: true, onclick: function () { toggleBranchChanges(); } });
      toolbar.appendChild(toolbar.branch);
      if (opts.status) [].concat(opts.status).forEach(function (s) { toolbar.appendChild(s); });
      toolbar.baseChip = h("span", { class: "dge-chip", hidden: true });
      toolbar.appendChild(toolbar.baseChip);
      toolbar.appendChild(h("div", { class: "dge-spacer" }));
      toolbar.extra = h("div", { class: "dge-group dge-extra" });
      toolbar.appendChild(toolbar.extra);
      (st.extraButtons || []).forEach(function (b) { toolbar.extra.appendChild(b); });
      var right = h("div", { class: "dge-group" });
      if (!view) {
        [
          [["select", "Select (V): click or drag a box around things; drag them to move them. Hold Space to pan."], ["hand", "Hand (H): drag anywhere to pan"]],
          [["node", "Box (N): click to place one"], ["sticky", "Sticky note (S): click to place one"], ["connect", "Connect (X): drag from one block to another, or drag a block's handle"]]
        ].forEach(function (set, i) {
          if (i) tools.appendChild(h("span", { class: "dge-tools-sep" }));
          set.forEach(function (t) {
            tools.appendChild(btn({ class: "dge-tool", "data-tool": t[0], title: t[1], "aria-pressed": "false", onclick: function () { setTool(t[0]); canvas.focus({ preventScroll: true }); } }, icon(t[0])));
          });
        });
        tools.appendChild(h("span", { class: "dge-tools-sep" }));
        blocksBtn = h("button", { class: "dge-tool dge-blocks-btn", "data-act": "blocks", "aria-haspopup": "menu", title: "Blocks: mindmaps, timelines, kanban, gantt, tables, charts, sequence and ER diagrams, decision records, risk matrices, swimlanes, sections and canvases", onclick: function () { toggleBlocks(); }, html: icon("blocks") + "<span>Blocks</span>" });
        tools.appendChild(blocksBtn);
        tools.appendChild(h("span", { class: "dge-tools-sep" }));
        tools.appendChild(btn({ class: "dge-tool", "data-tool": "comment", title: "Comment (C): click a block to comment on it, or anywhere on the canvas", "aria-pressed": "false", onclick: function () { setTool("comment"); canvas.focus({ preventScroll: true }); } }, icon("comment")));
        if (!blocksMenu) buildBlocks();

        right.appendChild(btn({ title: "Undo (" + MOD + "Z)", onclick: undo }, icon("undo")));
        right.appendChild(btn({ title: "Redo (" + MOD + "⇧Z)", onclick: redo }, icon("redo")));
        var dir = h("select", { title: "Direction", class: "dge-select", onchange: function (e) { applyOps([{ op: "set", key: "direction", value: e.target.value }]); } });
        [["right", "→ Right"], ["down", "↓ Down"], ["left", "← Left"], ["up", "↑ Up"]].forEach(function (d) { dir.appendChild(h("option", { value: d[0] }, [d[1]])); });
        var routing = h("select", { title: "Edge style", class: "dge-select", onchange: function (e) { applyOps([{ op: "set", key: "routing", value: e.target.value }]); } });
        [["orthogonal", "Elbow"], ["curved", "Curved"], ["straight", "Straight"]].forEach(function (d) { routing.appendChild(h("option", { value: d[0] }, [d[1]])); });
        var themeSel = h("select", { title: "Theme of this board", class: "dge-select", onchange: function (e) {
          var v = e.target.value;
          if (v === "__edit") { e.target.value = currentTheme(); boardMenu.hidden = true; editTheme(); return; }
          applyOps([{ op: "set", key: "theme", value: v || "light" }]);
        } });
        toolbar.dirSel = dir; // not `dir`: elements have a `dir` (text direction) of their own
        toolbar.routing = routing;
        toolbar.theme = themeSel;
        if (boardMenu) boardMenu.remove();
        var row = function (label, ctl) { return h("label", { class: "dge-board-row" }, [h("span", {}, [label]), ctl]); };
        var kindSel = h("select", { title: "What the board is for: it decides the built-in perspectives", class: "dge-select", onchange: function (e) { changeType(e.target.value); } });
        [["sketch", "Sketch: just a diagram"], ["product", "Product: understand it"], ["project", "Project: run it"]].forEach(function (d) { kindSel.appendChild(h("option", { value: d[0] }, [d[1]])); });
        toolbar.kind = kindSel;
        boardMenu = h("div", { class: "dge-board-menu", role: "dialog", "aria-label": "Board", hidden: true }, [
          row("Type", kindSel), row("Flow", dir), row("Arrows", routing), row("Theme", themeSel),
          h("button", { class: "dge-board-tidy", onclick: function () { boardMenu.hidden = true; applyOps([{ op: "unpin" }]); fitSoon(); } }, [h("span", { html: icon("tidy") }), "Tidy up the layout"]),
          h("button", { class: "dge-board-tidy", "data-act": "prompts", title: "The prompts the AI buttons send, to review", onclick: showPrompts }, [h("span", { html: icon("magic") }), "AI prompts…"])
        ]);
        el.appendChild(boardMenu);
        right.appendChild(h("button", { class: "dge-bar-btn", "data-act": "board", title: "Board: flow direction, arrow style, theme, tidy up", onclick: function (e) { toggleBoardMenu(e.currentTarget); }, html: icon("tidy") + "<span>Board</span>" }));
        setTimeout(fillThemes); // the first render came before the toolbar
      }
      if (opts.history && !view) right.appendChild(btn({ "data-act": "history", title: "History: the board's versions; see what changed since one, or bring it back", onclick: function (e) { toggleHistory(e.currentTarget); } }, icon("history")));
      right.appendChild(btn({ "data-act": "deck", title: view ? "Present: the board's slides one at a time (← →, Esc)" : "Presentation: the slides and their order, transitions and notes; then Present", onclick: function () { if (view) present(); else if (deck) closeDeck(); else openDeck(); } }, icon("present")));
      if (!view && hasCode) right.appendChild(btn({ "data-act": "code", title: "Code (" + MOD + "/)", onclick: function () { showCode = !showCode; applyChrome(); } }, icon("code")));
      if (hasPanel) right.appendChild(btn({ "data-act": "panel", title: "Inspector & comments", onclick: function () { showPanel = !showPanel; applyChrome(); renderPanel(); } }, icon("panel")));
      right.appendChild(btn({ title: "Light / dark", onclick: function () { st.dark = !st.dark; applyChrome(); render(); } }, icon("theme")));
      right.appendChild(h("button", { class: "dge-share", title: "Share: a link, an image, the source", onclick: function (e) { openExport(e.currentTarget); } }, ["Share"]));
      toolbar.appendChild(right);
      updateTrail();
    }

    function buildTabs() {
      codeTabs.innerHTML = "";
      [["dsl", "Diagram"], ["json", "JSON"], ["mermaid", "Mermaid"]].forEach(function (t) {
        codeTabs.appendChild(h("button", { class: st.codeFormat === t[0] ? "on" : "", onclick: function () { st.codeFormat = t[0]; buildTabs(); syncCode(true); } }, [t[1]]));
      });
      codeTabs.appendChild(h("a", { class: "dge-help", href: "https://github.com/dobrinov/shape/blob/main/crates/core/LANGUAGE.md", target: "_blank", rel: "noopener" }, ["syntax ↗"]));
    }

    // --------------------------------------------------------- doc state
    function refreshDoc() {
      st.doc = JSON.parse(st.session.document());
      updateTrail();
    }

    /* Canvases inside the board: the one open, and the way back up. */
    /* Where you are: the board, then the canvases you went into. */
    function updateTrail() {
      if (st.answer) {
        // the board you were on, then the answer
        if (!toolbar.crumbs) return;
        var mt = JSON.parse(st.answer.main.session.canvas_trail());
        toolbar.crumbs.innerHTML = "";
        toolbar.crumbs.hidden = false;
        mt.forEach(function (c, i) {
          if (i) toolbar.crumbs.appendChild(h("span", { class: "dge-crumb-sep", "aria-hidden": "true" }, ["/"]));
          toolbar.crumbs.appendChild(h("button", { title: "Back to “" + c.title + "”", onclick: function () { closeAnswer(); openCanvas(c.path); } }, [i === 0 && c.title === "Board" && opts.name ? opts.name : c.title]));
        });
        toolbar.crumbs.appendChild(h("span", { class: "dge-crumb-sep", "aria-hidden": "true" }, ["/"]));
        toolbar.crumbs.appendChild(h("strong", { class: "dge-crumb-answer", title: "A temporary answer, not saved" }, ["✦ " + (st.doc && st.doc.title || st.answer.question)]));
        st.canvasPath = []; // the answer is a board of its own
        return;
      }
      var t = JSON.parse(st.session.canvas_trail());
      st.canvasPath = t[t.length - 1].path;
      if (!toolbar.crumbs) return;
      toolbar.crumbs.innerHTML = "";
      if (t.length === 1 && t[0].title === "Board" && opts.name) t[0].title = opts.name;
      toolbar.crumbs.hidden = t.length < 2;
      t.forEach(function (c, i) {
        if (i) toolbar.crumbs.appendChild(h("span", { class: "dge-crumb-sep", "aria-hidden": "true" }, ["/"]));
        var last = i === t.length - 1;
        toolbar.crumbs.appendChild(last
          ? h("strong", { title: i === 0 && opts.file ? opts.file : c.title }, [c.title])
          : h("button", { title: "Back to “" + c.title + "”", onclick: function () { openCanvas(c.path); } }, [c.title]));
      });
    }
    /* Open the canvas at `path` (ids from the top board; [] for the top). */
    function openCanvas(path) {
      var res = JSON.parse(st.session.open_canvas(JSON.stringify(path || [])));
      if (!res.ok) { flash(res.error, true); return false; }
      select([]);
      refreshDoc();
      render();
      fit();
      emit("canvas", st.canvasPath.slice());
      return true;
    }
    function openLinked(id) {
      var n = node(id);
      if (n && n.canvas) openCanvas(st.canvasPath.concat([n.canvas]));
    }
    /* A new canvas inside this one, with a button on this one that opens it. */
    function newCanvas(at) {
      askAt(st.menuAt, "", "Name of the new canvas", function (title) {
        if (!title) return;
        var id = freshId(slug(title) + "-link"), cid = canvasIdFor(title);
        var res = applyOps([
          { op: "add_canvas", id: cid, title: title },
          { op: "add_node", id: id, label: title, shape: "pill", canvas: cid, x: Math.round(at.x - 70), y: Math.round(at.y - 22) }
        ]);
        if (res.ok) { select([id]); flash("Double-click “" + title + "” (or its badge) to open it"); }
      });
    }
    /* The id add_canvas will give a canvas with this title. */
    function canvasIdFor(title) {
      var base = slug(title) || "canvas", id = base, k = 2;
      var taken = function (x) { return idTaken(x) || (st.doc.canvases || []).some(function (c) { return c.id === x; }); };
      while (taken(id)) id = base + "-" + k++;
      return id;
    }
    function canvasOf(id) { var n = node(id); return n && n.canvas && (st.doc.canvases || []).find(function (c) { return c.id === n.canvas; }); }
    function countIn(doc) {
      return (doc.nodes || []).length + (doc.edges || []).length + (doc.groups || []).length + (doc.canvases || []).reduce(function (k, c) { return k + 1 + countIn(c); }, 0);
    }
    /* Removing a box that opens a canvas removes the canvas: ask first. */
    function confirmCanvases(ids, go) {
      var cs = ids.map(canvasOf).filter(Boolean);
      if (!cs.length) return go();
      var names = cs.map(function (c) { return "“" + (c.title || c.id) + "” (" + countIn(c) + " inside)"; }).join(", ");
      showModal(cs.length > 1 ? "Delete these canvases?" : "Delete this canvas?", h("p", {}, [
        "Deleting the link also deletes the canvas " + names + ". " + MOD + "Z brings it back."
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary danger", onclick: function () { closeModal(); go(); } }, ["Delete"])
      ]);
    }
    function linkStyleMenu(id) {
      var cur = node(id).shape || "box";
      setTimeout(function () {
        openMenu(st.menuAt, [["pill", "Button"], ["box", "Card"], ["text", "Text"], ["sticky", "Sticky note"], ["circle", "Circle"]].map(function (k) {
          return [(k[0] === cur ? "✓ " : "\u2003") + k[1], function () { applyOps([{ op: "update_node", id: id, shape: k[0] }]); }];
        }));
      }, 0);
    }
    function node(id) { return st.doc && st.doc.nodes.find(function (n) { return n.id === id; }); }
    function edge(id) { return st.doc && (st.doc.edges || []).find(function (e) { return e.id === id; }); }
    function group(id) { return st.doc && (st.doc.groups || []).find(function (g) { return g.id === id; }); }
    function comment(id) { return st.doc && (st.doc.comments || []).find(function (c) { return c.id === id; }); }
    function kindOf(id) { return node(id) ? "node" : edge(id) ? "edge" : group(id) ? "group" : null; }
    function idTaken(id) { return !!(node(id) || edge(id) || group(id)); }
    /* Unused ids for things about to be made together (distinct from each other too). */
    function freshIds(bases) {
      var out = [];
      bases.forEach(function (b) {
        var x = b, i = 2;
        while (idTaken(x) || out.indexOf(x) >= 0) x = b + "-" + i++;
        out.push(x);
      });
      return out;
    }
    function freshId(base) {
      if (!idTaken(base)) return base;
      for (var i = 2; ; i++) if (!idTaken(base + "-" + i)) return base + "-" + i;
    }
    function labelOf(id) {
      var n = node(id) || group(id);
      if (n) return n.label || n.id;
      var e = edge(id);
      if (e) return (e.label ? "“" + e.label + "” " : "") + e.from + " → " + e.to;
      return id;
    }

    function load(src, record) {
      if (record && st.text !== src) pushUndo();
      var res = JSON.parse(st.session.set_source(src));
      st.errors = res.errors || [];
      st.text = res.format === "dsl" ? src : st.session.source("dsl");
      refreshDoc();
      return res;
    }

    function pushUndo() {
      st.undo.push(st.text);
      if (st.undo.length > 200) st.undo.shift();
      st.redo = [];
    }
    function undo() {
      if (st.preview) return discardPreview();
      if (!st.undo.length) return;
      st.redo.push(st.text);
      load(st.undo.pop(), false);
      afterChange(true);
    }
    function redo() {
      if (!st.redo.length) return;
      st.undo.push(st.text);
      load(st.redo.pop(), false);
      afterChange(true);
    }

    function afterChange(syncText) {
      if (syncText) syncCode(true);
      showErrors();
      render();
      if (st.answer) return; // a temporary answer is not the board
      emit("change", st.text);
      if (opts.onChange) opts.onChange(st.text, api);
      if (since) { clearTimeout(branchTimer); branchTimer = setTimeout(paintBranch, 600); }
    }

    /* Apply ops through the core. `live` skips undo/code sync (used while dragging). */
    /* `top`: ops for the whole board (its type, its perspectives), applied to
       the top board also while a canvas is open. */
    function applyOps(ops, live, top) {
      // without a name (none from git), say who you are first
      if (opts.requireName && !st.author && !view) { askName(); return { ok: false, error: "your name first" }; }
      // editing while looking at the AI's changes keeps them
      if (st.preview && !live) acceptPreview();
      // something new, made while looking through a perspective, belongs to it
      var tag = st.perspective && st.perspective.tags && st.perspective.tags[0];
      if (tag) ops = ops.map(function (o) { return o.op === "add_node" && !(o.tags && o.tags.length) ? Object.assign({}, o, { tags: [tag] }) : o; });
      var before = st.text;
      var res = JSON.parse(top ? st.session.apply_top(JSON.stringify({ ops: ops })) : st.session.apply(JSON.stringify({ ops: ops })));
      if (!res.ok) {
        flash(res.error || "could not apply the change", true);
        return res;
      }
      refreshDoc();
      st.errors = [];
      if (live) {
        render();
        return res;
      }
      st.undo.push(before);
      if (st.undo.length > 200) st.undo.shift();
      st.redo = [];
      st.text = st.session.source("dsl");
      afterChange(true);
      return res;
    }

    // ------------------------------------------------------------ code pane
    function syncCode(force) {
      if (!showCode && !force) return;
      var text = st.codeFormat === "dsl" ? st.text : st.session.source(st.codeFormat);
      if (textarea.value !== text) {
        var top = textarea.scrollTop;
        textarea.value = text;
        textarea.scrollTop = top;
      }
      textarea.readOnly = st.codeFormat === "mermaid";
      updateGutter();
    }
    function updateGutter() {
      var lines = textarea.value.split("\n").length;
      var bad = {};
      if (st.codeFormat === "dsl") st.errors.forEach(function (e) { bad[e.line] = e.message; });
      var html = "";
      for (var i = 1; i <= lines; i++) html += '<div class="' + (bad[i] ? "err" : "") + '"' + (bad[i] ? ' title="' + bad[i].replace(/"/g, "&quot;") + '"' : "") + ">" + i + "</div>";
      gutter.innerHTML = html;
      gutter.scrollTop = textarea.scrollTop;
    }
    function showErrors() {
      errorsEl.innerHTML = "";
      st.errors.forEach(function (e) {
        errorsEl.appendChild(h("button", { class: "dge-error", onclick: function () { gotoLine(e.line); } }, [(e.line ? "line " + e.line + ": " : "") + e.message]));
      });
      errorsEl.hidden = !st.errors.length;
      updateGutter();
    }
    function gotoLine(line) {
      if (!line) return;
      var lines = textarea.value.split("\n");
      var start = 0;
      for (var i = 0; i < line - 1 && i < lines.length; i++) start += lines[i].length + 1;
      textarea.focus();
      textarea.setSelectionRange(start, start + (lines[line - 1] || "").length);
      textarea.scrollTop = Math.max(0, (line - 5) * 20);
    }
    var typing = null;
    textarea.addEventListener("input", function () {
      updateGutter();
      clearTimeout(typing);
      typing = setTimeout(function () {
        if (st.codeFormat === "mermaid") return;
        var before = st.text;
        var res = JSON.parse(st.session.set_source(textarea.value));
        st.errors = res.errors || [];
        if (st.text !== textarea.value) {
          st.undo.push(before);
          st.redo = [];
        }
        st.text = st.codeFormat === "dsl" ? textarea.value : st.session.source("dsl");
        refreshDoc();
        afterChange(false);
      }, 180);
    });
    textarea.addEventListener("scroll", function () { gutter.scrollTop = textarea.scrollTop; });
    textarea.addEventListener("keydown", function (e) {
      if (e.key === "Tab") {
        e.preventDefault();
        document.execCommand("insertText", false, "  ");
      }
    });

    // --------------------------------------------------------------- render
    function render() {
      var r = JSON.parse(st.session.render(JSON.stringify({ comments: st.showResolved ? "all" : "open", theme: st.dark ? "dark" : "light", today: isoDay(today()), perspective: st.perspective || null, open: st.perspective ? st.perspOpen : null })));
      if (r.error) {
        flash(r.error, true);
        return;
      }
      var old = st.r;
      st.r = r;
      stage.innerHTML = r.svg;
      // the canvas around the board takes the board's theme
      if (r.canvas) {
        canvas.style.setProperty("--canvas", r.canvas.background);
        canvas.style.setProperty("--dot", r.canvas.dots);
        canvas.classList.toggle("dge-dark-canvas", !!r.canvas.dark);
      }
      overlay.setAttribute("viewBox", [r.view.x, r.view.y, r.view.w, r.view.h].join(" "));
      overlay.setAttribute("width", r.view.w);
      overlay.setAttribute("height", r.view.h);
      stage.appendChild(overlay);
      if (deck) {
        // presentation mode: the slides' order on the board
        var layer = document.createElementNS("http://www.w3.org/2000/svg", "svg");
        layer.setAttribute("class", "dge-deck-layer");
        layer.setAttribute("viewBox", [r.view.x, r.view.y, r.view.w, r.view.h].join(" "));
        layer.setAttribute("width", r.view.w);
        layer.setAttribute("height", r.view.h);
        layer.innerHTML = '<defs><marker id="dge-deck-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" class="dge-deck-head-mark"/></marker></defs>';
        drawDeckOrder(layer);
        stage.appendChild(layer);
        if (!deck.contains(document.activeElement)) renderDeck();
      }
      if (old && st.fitted) {
        st.panX += (r.view.x - old.view.x) * st.zoom;
        st.panY += (r.view.y - old.view.y) * st.zoom;
      }
      if (!st.fitted && canvas.clientWidth) {
        fit();
        st.fitted = true;
      }
      drawMinimap();
      transform();
      markSelection();
      drawOverlay();
      renderPanel();
      if (toolbar.kind && st.doc) toolbar.kind.value = (st.answer ? st.answer.main.session : st.session).board_type();
      fillPerspectives();
      if (toolbar.dirSel && st.doc) {
        var s = st.doc.settings || {};
        toolbar.dirSel.value = s.direction || "right";
        toolbar.routing.value = s.routing || "orthogonal";
        fillThemes();
      }
      var nodes = st.doc ? st.doc.nodes.length : 0;
      hint.textContent = nodes ? "" : view ? "Empty diagram" : "Press T and click to start a mindmap · double-click to add a box · or paste a list";
      hint.hidden = !!nodes;
      emptyView(nodes > 0 && st.perspective && !r.nodes.length && !(r.groups || []).length);
    }

    function transform() {
      stage.style.transform = "translate(" + st.panX + "px," + st.panY + "px) scale(" + st.zoom + ")";
      zoomLabel.textContent = Math.round(st.zoom * 100) + "%";
      placeTitles();
      placeMinimapView();
    }
    /* Zoomed out, section titles get too small to read: they are shown on
       top at a readable size instead. */
    var titles = null;
    function placeTitles() {
      if (!titles) {
        titles = h("div", { class: "dge-titles", "aria-hidden": "true" });
        canvas.appendChild(titles);
      }
      var show = st.r && st.zoom < 0.6 && !st.presenting;
      titles.hidden = !show;
      if (!show) return;
      // top-level sections only: nested titles would sit on top of their parents'
      var want = (st.r.groups || []).filter(function (g) { var gr = group(g.id); return gr && !gr.parent; });
      if (titles._n !== want.length || titles._r !== st.r) {
        titles.innerHTML = "";
        want.forEach(function (g) {
          var gr = group(g.id);
          titles.appendChild(h("div", { class: "dge-title", "data-id": g.id }, [gr.label || gr.id]));
        });
        titles._n = want.length;
        titles._r = st.r;
      }
      [].forEach.call(titles.children, function (t) {
        var g = geom(t.getAttribute("data-id"));
        if (!g) return;
        var p = toScreen(g.x, g.y);
        t.style.left = p.x + "px";
        t.style.top = p.y + "px";
        t.style.maxWidth = Math.max(40, g.w * st.zoom) + "px";
      });
    }
    function fit() {
      if (!st.r) return;
      var cw = canvas.clientWidth, ch = canvas.clientHeight;
      if (!cw || !ch) return;
      var v = st.r.view;
      // an answer's bar covers the top: fit below it
      var top = st.answer && st.answer.bar && st.answer.bar.isConnected ? st.answer.bar.offsetTop + st.answer.bar.offsetHeight : 0;
      st.zoom = Math.max(0.1, Math.min(1.5, Math.min(cw / v.w, (ch - top) / v.h) * 0.94));
      st.panX = (cw - v.w * st.zoom) / 2;
      st.panY = top + (ch - top - v.h * st.zoom) / 2;
      transform();
    }
    function fitSoon() { setTimeout(fit, 0); }
    function zoomTo(z, cx, cy) {
      z = Math.max(0.1, Math.min(4, z));
      if (cx == null) { cx = canvas.clientWidth / 2; cy = canvas.clientHeight / 2; }
      st.panX = cx - (cx - st.panX) * (z / st.zoom);
      st.panY = cy - (cy - st.panY) * (z / st.zoom);
      st.zoom = z;
      transform();
    }
    function toDiagram(clientX, clientY) {
      var rect = canvas.getBoundingClientRect();
      return { x: st.r.view.x + (clientX - rect.left - st.panX) / st.zoom, y: st.r.view.y + (clientY - rect.top - st.panY) / st.zoom };
    }
    function toScreen(x, y) {
      return { x: st.panX + (x - st.r.view.x) * st.zoom, y: st.panY + (y - st.r.view.y) * st.zoom };
    }
    function geom(id) {
      if (!st.r) return null;
      return st.r.nodes.find(function (n) { return n.id === id; }) || st.r.groups.find(function (g) { return g.id === id; });
    }

    // ------------------------------------------------------------ mindmaps
    function mapInfo(id) { var g = st.r && st.r.nodes.find(function (n) { return n.id === id; }); return g && g.map ? g.map : null; }
    function isBranch(id) { var m = mapInfo(id); return !!(m && m.parent); }
    function kidsOf(id) { return st.doc.nodes.filter(function (n) { return n.parent === id; }).map(function (n) { return n.id; }); }
    function subtreeOf(id) {
      var out = [id];
      for (var i = 0; i < out.length; i++) kidsOf(out[i]).forEach(function (k) { if (out.indexOf(k) < 0) out.push(k); });
      return out;
    }
    /* Topics of the map centred on `root` that were placed by hand. */
    function placedTopics(root) { return subtreeOf(root).filter(function (id) { var n = node(id); return id !== root && n && n.x != null; }); }
    function resetArrangement(root) {
      var ops = placedTopics(root).map(function (id) { return { op: "unpin", id: id }; });
      if (ops.length && applyOps(ops).ok) flash("Mindmap arrangement reset");
    }
    /* The side a topic's own branches grow towards. */
    function outerSide(id) {
      var m = mapInfo(id);
      if (!m) return "right";
      if (m.depth === 0) return m.layout === "down" ? "down" : m.layout === "left" ? "left" : "right";
      return m.side;
    }
    /* Where a topic's branches leave it (screen of the map's own geometry). */
    function outerPoint(id, out) {
      var g = geom(id), m = mapInfo(id), side = outerSide(id);
      var underline = m && m.depth >= 2 && m.layout !== "down" && (node(id).shape || "box") === "box";
      var y = underline ? g.y + g.h : g.y + g.h / 2;
      if (side === "down") return { x: g.x + g.w / 2, y: g.y + g.h + out, side: side };
      if (side === "left") return { x: g.x - out, y: y, side: side };
      return { x: g.x + g.w + out, y: y, side: side };
    }

    /* Pan just enough to bring a node into view. */
    function reveal(id) {
      var g = geom(id);
      if (!g) return;
      var a = toScreen(g.x, g.y), b = toScreen(g.x + g.w, g.y + g.h), m = 48;
      var dx = 0, dy = 0, cw = canvas.clientWidth, ch = canvas.clientHeight;
      if (b.x > cw - m) dx = cw - m - b.x;
      if (a.x + dx < m) dx = m - a.x;
      if (b.y > ch - m) dy = ch - m - b.y;
      if (a.y + dy < m) dy = m - a.y;
      if (dx || dy) { st.panX += dx; st.panY += dy; transform(); }
    }

    function addTopic(parent, index, flow) {
      var id = freshId("topic");
      var ops = [];
      if (node(parent) && node(parent).collapsed) ops.push({ op: "update_node", id: parent, collapsed: false });
      var op = { op: "add_node", id: id, label: "", parent: parent };
      if (index != null) op.index = index;
      ops.push(op);
      var res = applyOps(ops);
      if (res.ok) {
        select([id]);
        editLabel(id, true, flow !== false);
      }
      return res.ok ? id : null;
    }
    /* A new topic between `id` and its parent; above the centre it becomes the
       new centre, in the same place, and the old centre its branch. */
    function addParent(id) {
      var n = node(id);
      if (!n) return null;
      var nid, add;
      if (n.parent) {
        nid = freshId("topic");
        add = { op: "add_node", id: nid, label: "", parent: n.parent, index: kidsOf(n.parent).indexOf(id) };
      } else {
        nid = freshId("map");
        add = { op: "add_node", id: nid, label: "", mindmap: n.mindmap || "right" };
        var g = geom(id);
        if (n.x != null || g) { add.x = n.x != null ? n.x : Math.round(g.x); add.y = n.y != null ? n.y : Math.round(g.y); }
        if (n.group) add.group = n.group;
      }
      var res = applyOps([add, { op: "update_node", id: id, parent: nid }]);
      if (res.ok) {
        select([nid]);
        editLabel(nid, true, false, id);
      }
      return res.ok ? nid : null;
    }
    function addSibling(id, after) {
      var n = node(id);
      if (!n || !n.parent) return addTopic(id);
      var i = kidsOf(n.parent).indexOf(id);
      return addTopic(n.parent, i + (after ? 1 : 0));
    }
    function outdent(id) {
      var n = node(id), p = n && n.parent && node(n.parent);
      if (n && !n.parent) return addParent(id); // ⇧Tab on the centre: a new centre above it
      if (!p || !p.parent) return;
      applyOps([{ op: "update_node", id: id, parent: p.parent, index: kidsOf(p.parent).indexOf(p.id) + 1 }]);
    }
    function moveSibling(id, delta) {
      var n = node(id);
      if (!n || !n.parent) return;
      var sibs = kidsOf(n.parent), j = sibs.indexOf(id) + delta;
      if (j < 0 || j >= sibs.length) return;
      applyOps([{ op: "update_node", id: id, index: j }]);
    }
    function toggleFold(id) {
      var n = node(id);
      if (!n || !kidsOf(id).length) return;
      applyOps([{ op: "update_node", id: id, collapsed: !n.collapsed }]);
    }
    /* Arrow keys walk the map the way it is drawn. */
    function navigate(key) {
      var id = st.sel[0], m = mapInfo(id), n = node(id);
      if (!m) return false;
      var side = outerSide(id), down = side === "down";
      var vis = function (pid, sd) {
        return st.r.nodes.filter(function (b) { return b.map && b.map.parent === pid && (!sd || b.map.side === sd); })
          .sort(function (a, b) { return down ? a.x - b.x : a.y - b.y; });
      };
      if (m.depth === 0 && !down && (key === "ArrowLeft" || key === "ArrowRight")) {
        var k0 = vis(id, key === "ArrowLeft" ? "left" : "right");
        if (k0.length) select([k0[Math.floor((k0.length - 1) / 2)].id]);
        return true;
      }
      var toParent = down ? "ArrowUp" : side === "right" ? "ArrowLeft" : "ArrowRight";
      var toChild = down ? "ArrowDown" : side === "right" ? "ArrowRight" : "ArrowLeft";
      if (key === toParent) { if (n.parent) select([n.parent]); return true; }
      if (key === toChild) {
        var kids = vis(id);
        if (kids.length) select([kids[0].id]);
        else if (n.collapsed) toggleFold(id);
        return true;
      }
      if (!n.parent) return true;
      var sibs = vis(n.parent, m.layout === "both" && m.depth === 1 ? m.side : null);
      var i = sibs.findIndex(function (b) { return b.id === id; });
      var prev = down ? "ArrowLeft" : "ArrowUp";
      var j = key === prev ? i - 1 : i + 1;
      if (sibs[j]) select([sibs[j].id]);
      return true;
    }

    /* Your AI: how to connect one (none is), which one is used and whether
       it works, or, after a call failed, what went wrong and what to try. */
    function setupAI(failure) {
      var setup = opts.aiSetup;
      var body = h("div", { class: "dge-ai-setup" });
      var code = function (t) { return h("code", {}, [t]); };
      // `quoted` parts of a hint as code
      var said = function (t) { return h("span", {}, String(t || "").split("`").map(function (part, i) { return i % 2 ? code(part) : part; })); };
      var status = h("p", { class: "dge-muted" });
      var check = function () {
        status.textContent = "Asking it to say OK…";
        setup.check().then(function (r) {
          status.innerHTML = "";
          if (r.ok) status.appendChild(h("span", { class: "dge-ok" }, ["✓ It works: it answered “" + String(r.text).slice(0, 60) + "”."]));
          else status.appendChild(h("span", {}, [h("b", {}, ["It did not work: "]), r.error, h("br"), h("b", {}, ["What to try: "]), said(r.hint)]));
        }, function (e) { status.textContent = "Could not check: " + e.message; });
      };
      var render = function (info) {
        body.innerHTML = "";
        if (failure) {
          body.appendChild(h("p", {}, ["Nothing on the board changed. What it said:"]));
          body.appendChild(h("pre", { class: "dge-ai-error" }, [String(failure.message || failure)]));
          if (failure.hint) body.appendChild(h("p", {}, [h("b", {}, ["What to try: "]), said(failure.hint)]));
        }
        if (opts.ai) {
          if (info && info.command) body.appendChild(h("p", {}, ["Connected through ", code(info.command), ". To use another one, start the editor with ", code("shape FILE --ai codex"), " (or ", code("claude"), ", ", code("gemini"), ", ", code("claude:opus"), ", any command that reads a prompt and prints the answer), or set ", code("SHAPE_AI"), "."]));
          else body.appendChild(h("p", {}, ["The AI is the one this app was set up with."]));
          if (setup) body.appendChild(status);
          return;
        }
        var installed = (info && info.installed) || [];
        body.appendChild(h("p", {}, ["diagram uses the AI subscription you already pay for, through its command-line tool, signed in as you. Install one and sign in once:"]));
        [
          ["claude", "Claude Code", "Claude Pro or Max", "curl -fsSL https://claude.ai/install.sh | bash", "claude", "https://docs.anthropic.com/en/docs/claude-code/setup"],
          ["codex", "Codex CLI", "ChatGPT Plus or Pro", "npm install -g @openai/codex", "codex login", "https://github.com/openai/codex"],
          ["gemini", "Gemini CLI", "a Google account", "npm install -g @google/gemini-cli", "gemini", "https://github.com/google-gemini/gemini-cli"]
        ].forEach(function (t) {
          var has = installed.indexOf(t[0]) >= 0;
          body.appendChild(h("div", { class: "dge-ai-tool" }, [
            h("div", {}, [h("b", {}, [t[1]]), h("span", { class: "dge-muted" }, [" · with " + t[2]]), has ? h("span", { class: "dge-ok" }, [" · installed"]) : null, " ", h("a", { href: t[5], target: "_blank", rel: "noopener" }, ["docs ↗"])]),
            has ? null : h("div", {}, ["Install: ", code(t[3])]),
            h("div", {}, ["Sign in: run ", code(t[4]), " in a terminal once"])
          ]));
        });
        body.appendChild(h("p", {}, [installed.length
          ? "One is installed but this editor started without it: close it and open the board again (" : "Then open the board again (",
          code("shape FILE"), "); it uses the first one it finds. Pick one with ", code("--ai codex"), ", or use any command that reads a prompt and prints the answer: ", code("--ai \"my-tool --flag\""), "."]));
        body.appendChild(h("p", { class: "dge-muted" }, ["Until then, the ✦ actions show the prompt to paste into any chat, and take the reply back."]));
      };
      showModal(failure ? "The AI did not answer" : opts.ai ? "Your AI" : "Connect your AI", body, [
        opts.ai && setup ? h("button", { onclick: check }, ["Check it works"]) : null,
        h("button", { class: "primary", onclick: closeModal }, ["Close"])
      ].filter(Boolean));
      render(null);
      if (setup) setup.status().then(render, function () {});
    }
    /* Ask the model. With no `ai` option the prompt is shown to copy into any
       chat or harness, and the reply is pasted back. */
    function ask(prompt, title) {
      // the date, so the model does not guess the year
      prompt = "Today is " + isoDay(today()) + ".\n\n" + prompt;
      // when the AI fails, say why and what to try
      if (opts.ai) return Promise.resolve().then(function () { return opts.ai(prompt); }).catch(function (e) {
        if (String(e && e.message) !== "cancelled") { setupAI(e); if (e && typeof e === "object") e.shown = true; }
        throw e;
      });
      return new Promise(function (resolve, reject) {
        var pre = h("textarea", { class: "dge-preview", readonly: true, rows: 10 });
        pre.value = prompt;
        var reply = h("textarea", { class: "dge-preview", rows: 8, placeholder: "Paste the model's reply here" });
        var settled = false;
        showModal(title || "✦ Ask a model", h("div", { class: "dge-prompt" }, [
          h("p", { class: "dge-muted" }, ["No model is connected here. Copy the prompt into any chat or agent, then paste its reply. ", h("a", { href: "#", onclick: function (e) { e.preventDefault(); settled = true; reject(new Error("cancelled")); setupAI(); } }, ["Connect your AI…"])]),
          pre, reply
        ]), [
          h("button", { onclick: function () { copyText(prompt).then(function () { flash("Prompt copied"); }); } }, ["Copy prompt"]),
          h("button", { class: "dge-primary", onclick: function () { settled = true; var t = reply.value; closeModal(); resolve(t); } }, ["Add the reply"])
        ]);
        var obs = new MutationObserver(function () { if (modal.hidden && !settled) { obs.disconnect(); reject(new Error("cancelled")); } });
        obs.observe(modal, { attributes: true, attributeFilter: ["hidden"] });
      });
    }
    // ------------------------------------------------------------ merge conflicts
    /* Settle a merge that clashed: yours, the result and theirs side by side,
       a side picked per conflict (or described in words for the AI to pick),
       then `done(source)` with the merged board. */
    function resolveConflict(m) {
      var base = m.base || "", ours = m.ours || "", theirs = m.theirs || "";
      var info = JSON.parse(wasm.merge3(base, ours, theirs));
      if (!info.ok) { flash("Cannot read the versions: " + info.error, true); return false; }
      var picks = {}, ops = [];
      var pane = function (title, cls) {
        var body = h("div", { class: "dge-merge-svg" });
        return { el: h("section", { class: "dge-merge-pane " + cls }, [h("h3", {}, [title]), body]), body: body };
      };
      var mine = pane("Yours", "mine"), result = pane("Result", "result"), other = pane("Theirs", "theirs");
      var look = JSON.stringify({ theme: st.dark ? "dark" : "light", comments: "none" });
      var clash = info.conflicts.filter(function (c) { return !c.canvas.length; }).map(function (c) { return c.id; });
      var show = function (p, r) {
        p.body.innerHTML = r.svg || "<p class='dge-muted'>" + (r.error || "") + "</p>";
        // what clashed, outlined (its own colours stay visible)
        clash.forEach(function (id) {
          p.body.querySelectorAll('[data-id="' + id.replace(/"/g, '\\"') + '"]').forEach(function (x) { x.classList.add("dge-clash"); });
        });
      };
      var draw = function (p, side) {
        if (side !== "merged") return show(p, JSON.parse(wasm.merge_svg(base, ours, theirs, side, "{}", look)));
        // the result as it will be saved: picks and the AI's edits
        var r = JSON.parse(wasm.merge_resolve(base, ours, theirs, JSON.stringify(picks), JSON.stringify(ops)));
        if (!r.ok) return show(p, { error: r.error });
        var d = new wasm.Diagram();
        d.set_source(r.source);
        show(p, JSON.parse(d.render(look)));
        d.free();
      };
      var note = h("p", { class: "dge-muted dge-merge-note" });
      var list = h("div", { class: "dge-merge-list" });
      var row = function (c) {
        var choose = function (side) { picks[c.key] = side; ops = []; refresh(); };
        var btn = function (side, label) {
          return h("button", { class: (picks[c.key] || "ours") === side ? "on" : "", onclick: function () { choose(side); } }, [label]);
        };
        return h("div", { class: "dge-merge-row" }, [
          h("span", {}, [c.text]),
          h("span", { class: "dge-merge-pick" }, [btn("ours", c.ours ? "Yours" : "Removed (yours)"), btn("theirs", c.theirs ? "Theirs" : "Removed (theirs)")])
        ]);
      };
      var refresh = function () {
        list.innerHTML = "";
        info.conflicts.forEach(function (c) { list.appendChild(row(c)); });
        note.textContent = info.conflicts.length
          ? info.conflicts.length + (info.conflicts.length === 1 ? " thing was" : " things were") + " changed both ways; everything else is merged." + (ops.length ? " The AI also made " + ops.length + " edit" + (ops.length === 1 ? "" : "s") + "." : "")
          : "Nothing clashed: everything is merged.";
        draw(result, "merged");
      };
      var wish = h("textarea", { class: "dge-chart-data", rows: "2", placeholder: "Or say what to keep, e.g. “their colours, but my labels; keep both new boxes”" });
      var askBtn = h("button", { onclick: function () {
        var w = wish.value.trim();
        if (!w) return wish.focus();
        askBtn.disabled = true;
        ask(wasm.merge_prompt(base, ours, theirs, w), "✦ Settle the merge").then(function (reply) {
          var a = JSON.parse(wasm.merge_answer(reply));
          if (!a.ok) throw new Error(a.error);
          picks = a.picks || {};
          ops = a.ops || [];
          refresh();
          flash("The AI picked sides" + (ops.length ? " and made " + ops.length + " edit" + (ops.length === 1 ? "" : "s") : "") + ": check the result");
        }).catch(function (e) { if (e.message !== "cancelled") flash("The AI's answer did not work: " + e.message, true); })
          .then(function () { askBtn.disabled = false; });
      } }, ["✦ Ask AI"]);
      var close = function () { panel.remove(); };
      var save = function () {
        var r = JSON.parse(wasm.merge_resolve(base, ours, theirs, JSON.stringify(picks), JSON.stringify(ops)));
        if (!r.ok) return flash(r.error, true);
        close();
        if (m.done) m.done(r.source);
        else api.setSource(r.source);
      };
      var panel = h("div", { class: "dge-merge", role: "dialog", "aria-label": "Settle the merge" }, [
        h("header", {}, [
          h("h2", {}, ["Settle the merge"]),
          note,
          h("span", { class: "dge-merge-actions" }, [
            h("button", { onclick: close, title: "Leave the file as it is" }, ["Later"]),
            h("button", { class: "primary", onclick: save }, [m.git ? "Save and mark resolved" : "Save"])
          ])
        ]),
        h("div", { class: "dge-merge-panes" }, [mine.el, result.el, other.el]),
        h("footer", {}, [list, h("div", { class: "dge-merge-ask" }, [wish, askBtn])])
      ]);
      el.appendChild(panel);
      draw(mine, "ours");
      draw(other, "theirs");
      refresh();
      return true;
    }

    // ------------------------------------------------------------ ask AI
    /* Add what you know: information to place, or a change to make, about the
       selection (or the board) goes to the model
       (the host's `ai`, e.g. the subscription CLI behind the local server, or
       copy/paste); its ops are tried on a copy, a reply that does not apply
       goes back with its error (twice at most), and the changes are shown
       before they are kept. */
    function askAI() {
      if (st.preview) return flash("Keep or discard the AI's changes first", true);
      var ids = st.sel.filter(kindOf);
      closeAiMenu();
      var input = h("textarea", { class: "dge-chart-data", rows: "5", placeholder: ids.length ? "e.g. Ana says this runs nightly and takes about 40 minutes; it must finish before 6am" : "e.g. In the interviews, 5 of 8 teams left over price; two asked for SSO; legal needs an audit log by March" });
      var go = function () {
        var req = input.value.trim();
        if (!req) return;
        closeModal();
        var prompt = st.session.ask_prompt(JSON.stringify(ids), req);
        askRound(prompt, 0, req, null, st.canvasPath.slice());
      };
      input.addEventListener("keydown", function (e) { if ((e.metaKey || e.ctrlKey) && e.key === "Enter") go(); });
      showModal("✦ Add what you know", h("div", {}, [
        h("p", { class: "dge-muted" }, ["Write down details or information you have; the AI puts each where it belongs" + (ids.length ? ", around the " + ids.length + " selected item" + (ids.length > 1 ? "s" : "") + "." : " on this canvas (select something to narrow it).") + " You can also ask for a change."]),
        input,
        h("p", { class: "dge-muted" }, [opts.ai ? "Your AI subscription's CLI answers; you see the changes before keeping them." : "No AI is connected here: you'll get the prompt to paste into any chat, then paste its reply back."])
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: go }, ["Add (" + MOD + "↵)"])
      ]);
      setTimeout(function () { input.focus(); }, 0);
    }
    /* A box's details: the context its title leaves out. */
    function editDetails(id) {
      var n = node(id);
      if (!n) return;
      var input = h("textarea", { class: "dge-chart-data", rows: "6", placeholder: "What is this about? Who is on it, what is known, links to threads…" });
      input.value = n.detail || "";
      var save = function () {
        closeModal();
        if (input.value.trim() !== (n.detail || "")) applyOps([{ op: "update_node", id: id, detail: input.value.trim() }]);
        canvas.focus({ preventScroll: true });
      };
      input.addEventListener("keydown", function (e) { if ((e.metaKey || e.ctrlKey) && e.key === "Enter") save(); });
      showModal("Details: " + (n.label || id), h("div", {}, [input]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: save }, ["Save (" + MOD + "↵)"])
      ]);
      setTimeout(function () { input.focus(); }, 0);
    }
    /* Ask: the conversation lasts while the panel is open; nothing is saved. */
    function openAsk() {
      closeAiMenu();
      askPanel.hidden = false;
      fitPopup(askPanel, true);
      setTimeout(function () { askInput.focus(); }, 0);
    }
    function closeAsk(keep) {
      askPanel.hidden = true;
      if (!keep) {
        st.askHistory = [];
        askThread.innerHTML = "";
      }
      canvas.focus({ preventScroll: true });
    }
    function sendQuestion() {
      var q = askInput.value.trim();
      if (!q || st.asking) return;
      askInput.value = "";
      var a = h("div", { class: "dge-ask-a dge-muted" }, [opts.ai ? "✦ Drawing an answer…" : "✦ Waiting for the reply you paste…"]);
      askThread.appendChild(h("div", { class: "dge-ask-q" }, [q]));
      askThread.appendChild(a);
      askThread.scrollTop = askThread.scrollHeight;
      var history = (st.askHistory || []).slice(-4);
      // the question is about the board, also when an answer is open
      var board = st.answer ? st.answer.main.session : st.session;
      var prompt = board.answer_board_prompt(JSON.stringify(history), q, isoDay(today()));
      st.asking = true;
      var round = function (p, n) {
        return ask(p, "✦ Ask: " + q).then(function (text) {
          var r = JSON.parse(wasm.answer_parts(String(text || "")));
          if (!r.ok && opts.ai && n < 2) return round(wasm.ask_again(prompt, String(text || ""), r.error), n + 1);
          if (!r.ok) throw new Error(r.error);
          return r;
        });
      };
      round(prompt, 0).then(function (r) {
        st.askHistory = history.concat([[q, r.summary]]);
        a.className = "dge-ask-a";
        a.textContent = r.summary || "Answered on its own canvas.";
        closeAsk(true);
        openAnswer(q, r.summary, r.source);
      }, function (e) {
        a.className = "dge-ask-a dge-ask-err";
        a.textContent = e && e.message === "cancelled" ? "Cancelled." : "No answer: " + (e && e.message || e);
      }).then(function () {
        st.asking = false;
        askThread.scrollTop = askThread.scrollHeight;
        if (!askPanel.hidden) askInput.focus();
      });
    }

    /* An answer: a temporary canvas the AI drew for the question. It has a
       session of its own and is never saved with the board (autosave, ⌘S
       and undo leave the file alone) until "Save as canvas". */
    function openAnswer(question, summary, source) {
      var main = st.answer ? st.answer.main : { session: st.session, text: st.text, undo: st.undo, redo: st.redo, path: st.canvasPath.slice() };
      if (st.answer) { st.answer.session.free(); st.answer.bar.remove(); }
      if (st.preview) discardPreview();
      var session = new wasm.Diagram();
      session.set_source(source);
      st.answer = { question: question, summary: summary, session: session, main: main };
      st.session = session;
      st.text = source;
      st.undo = [];
      st.redo = [];
      select([]);
      refreshDoc();
      render();
      var said = h("span", { class: "dge-answer-summary" });
      (summary || "").split(/(\*\*[^*]+\*\*)/).forEach(function (part) {
        var b = /^\*\*([^*]+)\*\*$/.exec(part);
        said.appendChild(b ? h("b", {}, [b[1]]) : document.createTextNode(part));
      });
      var bar = h("div", { class: "dge-preview-bar dge-answer-bar", role: "status" }, [
        h("span", { title: (summary || "").replace(/\*\*/g, "") }, [h("b", {}, ["✦ " + question]), summary ? said : null].filter(Boolean)),
        h("span", { class: "dge-muted dge-answer-temp" }, ["Temporary, not saved"]),
        h("button", { title: "Ask a follow-up question; the answer is drawn again", onclick: function () { openAsk(); } }, ["Follow-up…"]),
        h("button", { title: "Rethink the board with this answer and your comments on it as the instructions", onclick: function () { rethinkFromAnswer(); } }, ["Rethink the board from this"]),
        h("button", { class: "primary", title: "Keep this answer as a canvas of the board, with a link to it", onclick: function () { saveAnswer(); } }, ["Save as canvas"]),
        h("button", { "aria-label": "Close the answer", title: "Back to the board (Esc)", onclick: function () { closeAnswer(); } }, ["×"])
      ]);
      st.answer.bar = bar;
      el.appendChild(bar);
      fit();
    }
    function closeAnswer() {
      if (!st.answer) return;
      var a = st.answer, m = a.main;
      a.bar.remove();
      a.session.free();
      st.answer = null;
      st.session = m.session;
      st.text = m.text;
      st.undo = m.undo;
      st.redo = m.redo;
      select([]);
      refreshDoc();
      render();
      fit();
      return a;
    }
    function rethinkFromAnswer() {
      var a = st.answer;
      if (!a) return;
      var drawn = st.session.source("dsl");
      var talk = (st.doc.comments || []).length ? " Act on the comments on it too: they say what the person thinks of the answer." : "";
      var instructions = "The person asked: “" + a.question + "”. The answer was drawn as this small board:\n\n```shape\n" + drawn + "```\n\nBring the board up to date with what this answer shows." + talk + " Keep every piece of information: reorganise, merge and move it, but never drop it.";
      closeAnswer();
      runRethink(instructions);
    }
    function saveAnswer() {
      var a = st.answer;
      if (!a) return;
      var drawn = st.session.source("dsl");
      var title = (st.doc.title || a.question).slice(0, 60);
      closeAnswer();
      var cid = canvasIdFor(title), at = freeSpot();
      var res = applyOps([
        { op: "add_canvas", id: cid, title: title },
        { op: "add_node", id: freshId(slug(title) + "-link"), label: title, shape: "pill", canvas: cid, x: Math.round(at.x), y: Math.round(at.y) },
        { op: "in", canvas: cid, ops: [{ op: "merge", dsl: drawn }] }
      ]);
      if (res.ok) flash("Saved as the canvas “" + title + "” (double-click its link to open it)");
    }
    /* An answer: paragraphs and bullets, **bold**, and [[canvas/id]] as links to that place. */
    function renderAnswer(into, text) {
      var refs = {};
      JSON.parse(st.session.answer_refs(text)).forEach(function (r) { refs[r.ref] = r; });
      var inline = function (line, parent) {
        line.split(/(\[\[[^\]]+\]\]|\*\*[^*]+\*\*)/).forEach(function (part) {
          var m = /^\[\[([^\]]+)\]\]$/.exec(part);
          if (m) {
            var r = refs[m[1].trim().replace(/^`|`$/g, "")];
            if (!r || !r.label) return parent.appendChild(h("span", { class: "dge-ref missing", title: "Not on the board: " + m[1] }, [m[1]]));
            var where = r.path.length ? (r.id ? "in " + r.path.join(" › ") : "a canvas") : "on the board";
            return parent.appendChild(h("button", { class: "dge-ref", title: "Go to it (" + where + ")", onclick: function () { goToRef(r); } }, [r.label]));
          }
          var b = /^\*\*([^*]+)\*\*$/.exec(part);
          if (b) return parent.appendChild(h("b", {}, [b[1]]));
          if (part) parent.appendChild(document.createTextNode(part));
        });
      };
      var list = null;
      text.split(/\n/).forEach(function (line) {
        var li = /^\s*(?:[-*•]|\d+[.)])\s+(.*)$/.exec(line);
        if (li) {
          if (!list) { list = h("ul"); into.appendChild(list); }
          var item = h("li");
          inline(li[1], item);
          list.appendChild(item);
          return;
        }
        list = null;
        if (!line.trim()) return;
        var para = h("p");
        inline(line, para);
        into.appendChild(para);
      });
    }
    function goToRef(r) {
      var here = JSON.stringify(st.canvasPath) === JSON.stringify(r.path);
      if (!here && !openCanvas(r.path)) return;
      if (r.id) followLink("#" + r.id);
    }
    /* Rethink: the model organises what was collected on the open canvas.
       Where the host knows an earlier version (`since()`: in git, where the
       branch left main, or the last commit), what changed since is the new
       information to revise the rest around. */
    /* Rethink: runs at once with its default instructions (the core's). */
    function rethink() {
      if (st.preview) return flash("Keep or discard the AI's changes first", true);
      closeAiMenu();
      runRethink();
    }
    function runRethink(instructions) {
      var since = opts.since ? Promise.resolve(opts.since()).catch(function () { return null; }) : Promise.resolve(null);
      since.then(function (v) {
        var own = instructions && instructions !== wasm.rethink_default() ? instructions : undefined;
        var r = JSON.parse(st.session.rethink_prompt(v ? v.source : undefined, v ? v.label : "", own));
        if (r.error) return flash(r.error, true);
        askRound(r.prompt, 0, "Rethink the board",
          v ? "✦ Rethinking the board, with what is new since " + v.label + "…" : "✦ Rethinking the board…", []);
      });
    }
    /* `where`: the canvas the reply's ops are about ([]: the top board,
       where `in` ops reach into canvases). */
    function askRound(prompt, round, req, busy, where) {
      if (opts.ai) flash(round ? "✦ The reply did not fit; asking again…" : busy || "✦ Thinking…", false, 180000);
      ask(prompt, "✦ " + req).then(function (text) {
        text = String(text || "");
        var test = new wasm.Diagram();
        test.set_source(st.text);
        if (where.length) test.open_canvas(JSON.stringify(where));
        var res = JSON.parse(test.apply(text));
        if (!res.ok) {
          test.free();
          if (opts.ai && round < 2) return askRound(wasm.ask_again(prompt, text, res.error || "invalid reply"), round + 1, req, busy, where);
          return flash("The AI's reply did not apply: " + (res.error || "no ops in it"), true);
        }
        var next = test.source("dsl");
        test.free();
        var note = wasm.reply_note(text);
        if (!res.count) return showNote(note || "The AI changed nothing.");
        startPreview(next, res.count, note);
      }, function (e) {
        if (e && e.shown) flash(""); // the dialog says it
        else if (String(e && e.message) !== "cancelled") flash("AI: " + (e && e.message || e), true);
      });
    }
    /* What the AI said when it changed nothing: stays until read. */
    function showNote(text) {
      flash("");
      var bar = h("div", { class: "dge-preview-bar", role: "status" }, [
        h("span", {}, ["✦ " + text]),
        h("button", { class: "primary", onclick: function () { bar.remove(); } }, ["OK"])
      ]);
      el.appendChild(bar);
    }
    /* The AI's changes on the board, highlighted, until kept or discarded. */
    function startPreview(next, count, note) {
      st.preview = { before: st.text, base: st.base, label: st.baseLabel };
      st.session.set_source(next);
      refreshDoc();
      setBase(st.preview.before, "before the AI");
      var bar = h("div", { class: "dge-preview-bar", role: "status" }, [
        h("span", { title: note || "" }, ["✦ " + (note ? note + " " : "The AI's changes are highlighted ") + "(" + count + " op" + (count === 1 ? "" : "s") + ")"]),
        h("button", { title: "What changed, as Markdown", onclick: function () { copyText(st.session.diff_markdown()).then(function () { flash("Change summary copied"); }); } }, ["Copy summary"]),
        h("button", { onclick: discardPreview }, ["Discard"]),
        h("button", { class: "primary", onclick: acceptPreview }, ["Keep"])
      ]);
      st.preview.bar = bar;
      el.appendChild(bar);
      toolbar.baseChip.hidden = true; // the bar says it
      flash("");
    }
    function endPreview() {
      var pv = st.preview;
      st.preview = null;
      if (pv.bar) pv.bar.remove();
      setBase(pv.base == null ? null : pv.base, pv.label);
      return pv;
    }
    function acceptPreview() {
      if (!st.preview) return;
      var pv = endPreview();
      st.undo.push(pv.before);
      st.redo = [];
      st.text = st.session.source("dsl");
      afterChange(true);
      flash("Kept the AI's changes (" + MOD + "Z undoes them)");
    }
    function discardPreview() {
      if (!st.preview) return;
      var pv = endPreview();
      st.session.set_source(pv.before);
      refreshDoc();
      render();
      flash("Discarded the AI's changes");
    }

    function thinking(id, on) {
      if (on) st.thinking[id] = true; else delete st.thinking[id];
      markSelection();
    }
    function expandTopic(id, instructions) {
      var r = JSON.parse(st.session.expand_prompt(id, JSON.stringify({ instructions: instructions || null })));
      if (r.error) return flash(r.error, true);
      thinking(id, true);
      if (opts.ai) flash("✦ Thinking…", false, 60000);
      ask(r.prompt, "✦ Grow “" + labelOf(id) + "”").then(function (text) {
        var outline = wasm.reply_outline(String(text || ""));
        if (!outline.trim()) throw new Error("The reply had no topics");
        var ops = [];
        if (node(id) && node(id).collapsed) ops.push({ op: "update_node", id: id, collapsed: false });
        ops.push({ op: "add_outline", parent: id, outline: outline });
        var res = applyOps(ops);
        if (res.ok) flash("✦ Added " + res.created.length + " topics");
      }).catch(function (e) { if (e && e.message !== "cancelled" && !e.shown) flash(e.message || String(e), true); else flash(""); })
        .then(function () { thinking(id, false); });
    }
    /* Free space for a new map: below everything that is there. */
    function freeSpot() {
      if (!st.r || !st.r.nodes.length) return toDiagram(canvas.getBoundingClientRect().left + canvas.clientWidth / 2, canvas.getBoundingClientRect().top + canvas.clientHeight / 2);
      var x = Infinity, y = -Infinity;
      st.r.nodes.forEach(function (n) { x = Math.min(x, n.x); y = Math.max(y, n.y + n.h); });
      (st.r.groups || []).forEach(function (g) { x = Math.min(x, g.x); y = Math.max(y, g.y + g.h); });
      return { x: x + 80, y: y + 96 };
    }
    function newMap(topic, at) {
      topic = (topic || "").trim();
      if (!topic) return;
      if (!at) at = freeSpot();
      if (opts.ai) flash("✦ Thinking…", false, 60000);
      ask(wasm.mindmap_prompt(topic, "{}"), "✦ Mindmap about “" + topic + "”").then(function (text) {
        var outline = wasm.reply_outline(String(text || ""));
        if (!outline.trim()) throw new Error("The reply had no topics");
        var res = applyOps([{ op: "add_outline", outline: outline, x: Math.round(at.x - 80), y: Math.round(at.y - 24) }]);
        if (res.ok) { select([res.created[0]]); reveal(res.created[0]); flash("✦ Added a mindmap with " + res.created.length + " topics"); }
      }).catch(function (e) { if (e && e.message !== "cancelled" && !e.shown) flash(e.message || String(e), true); else flash(""); });
    }
    function pasteOutline(text) {
      var lines = text.split(/\r?\n/).filter(function (l) { return l.trim(); });
      if (!lines.length) return false;
      var target = st.sel.length === 1 && mapInfo(st.sel[0]) ? st.sel[0] : null;
      if (!target && lines.length < 2) return false;
      var outline = wasm.reply_outline(text);
      var op = { op: "add_outline", outline: outline };
      if (target) op.parent = target;
      else {
        var c = freeSpot();
        op.x = Math.round(c.x - 80); op.y = Math.round(c.y - 24);
      }
      var res = applyOps([op]);
      if (res.ok) {
        if (!target) { select([res.created[0]]); reveal(res.created[0]); }
        flash("Added " + res.created.length + " topics");
      }
      return res.ok;
    }

    function markSelection() {
      updateScreenBar();
      stage.querySelectorAll(".sel").forEach(function (e) { e.classList.remove("sel"); });
      st.sel.forEach(function (id) {
        stage.querySelectorAll('[data-id="' + cssEsc(id) + '"]').forEach(function (e) { e.classList.add("sel"); });
      });
      if (st.comment) stage.querySelectorAll('[data-comment="' + cssEsc(st.comment) + '"]').forEach(function (e) { e.classList.add("sel"); });
      Object.keys(st.thinking).forEach(function (id) {
        stage.querySelectorAll('[data-id="' + cssEsc(id) + '"]').forEach(function (e) { e.classList.add("thinking"); });
      });
      if (st.comment) {
        var c = comment(st.comment);
        if (c && c.on) stage.querySelectorAll('[data-id="' + cssEsc(c.on) + '"]').forEach(function (e) { e.classList.add("cmt"); });
      }
    }
    function cssEsc(s) { return String(s).replace(/["\\]/g, "\\$&"); }

    var SVGNS = "http://www.w3.org/2000/svg";
    function svgEl(tag, attrs) {
      var e = document.createElementNS(SVGNS, tag);
      Object.keys(attrs).forEach(function (k) { e.setAttribute(k, attrs[k]); });
      return e;
    }
    function control(cls, p, r, label, id) {
      var gEl = svgEl("g", { class: "dge-ctl " + cls, "data-node": id });
      gEl.appendChild(svgEl("circle", { cx: p.x, cy: p.y, r: r }));
      var t = svgEl("text", { x: p.x, y: p.y + r * 0.42, "font-size": r * 1.4, "text-anchor": "middle" });
      t.textContent = label;
      gEl.appendChild(t);
      overlay.appendChild(gEl);
    }
    function drawOverlay() {
      while (overlay.firstChild) overlay.removeChild(overlay.firstChild);
      whens.forEach(function (w) { w.remove(); });
      whens = [];
      if (view || !st.r) return;
      var targets = st.drag && st.drag.type === "connect" ? [] : st.sel.filter(function (id) { return node(id); });
      if (st.hover && node(st.hover) && targets.indexOf(st.hover) < 0) targets.push(st.hover);
      if (st.tool !== "select" && st.tool !== "connect") targets = [];
      targets.forEach(function (id) {
        var g = geom(id);
        if (!g) return;
        if (mapInfo(id) && st.tool !== "connect") {
          // mindmap topics: + adds a branch, − / + folds
          if (d && d.type === "branch") return;
          var rr = 9 / Math.max(st.zoom, 0.5);
          var hasKids = kidsOf(id).length > 0, folded = node(id).collapsed;
          var pp = outerPoint(id, 14 / Math.max(st.zoom, 0.5) + 2);
          if (hasKids && !folded) {
            control("dge-foldbtn", pp, rr, "−", id);
            pp = pp.side === "down" ? { x: pp.x + 2.4 * rr, y: pp.y } : { x: pp.x, y: pp.y - 2.4 * rr };
          }
          if (!folded) control("dge-plus", pp, rr, "+", id);
          return;
        }
        if (matrixOf(id) || plotOf(id)) return; // risks' dots and charts' rows have no handles
        var r = 5 / Math.max(st.zoom, 0.4);
        // a task: one dot past the end of its bar (draw to the task that waits for it); the end itself resizes
        var dots = chartOf(id) ? [[g.x + g.w + 3 * r, g.y + g.h / 2]]
          : [[g.x + g.w / 2, g.y], [g.x + g.w, g.y + g.h / 2], [g.x + g.w / 2, g.y + g.h], [g.x, g.y + g.h / 2]];
        dots.forEach(function (p) {
          var c = svgEl("circle", { cx: p[0], cy: p[1], r: r, class: "dge-handle", "data-node": id });
          overlay.appendChild(c);
        });
        // the one selected box (not a task, a screen's part or a list's item): its corner resizes it
        // (the grip sits just outside it, clear of a link's or a canvas's badge)
        if (st.sel.length === 1 && st.sel[0] === id && !chartOf(id) && !screenOf(id) && !todoOf(id) && !(st.drag && st.drag.type === "size")) {
          var gs = 9 / Math.max(st.zoom, 0.5);
          overlay.appendChild(svgEl("rect", { class: "dge-grip", "data-node": id, x: g.x + g.w + gs * 0.15, y: g.y + g.h + gs * 0.15, width: gs, height: gs, rx: gs * 0.25 }));
        }
      });
      var d = st.drag;
      if (d && d.type === "connect" && d.to) {
        var g0 = geom(d.from), s0 = d.start || { x: g0.x + g0.w / 2, y: g0.y + g0.h / 2 };
        overlay.appendChild(svgEl("path", { class: "dge-rubber", d: "M" + s0.x + "," + s0.y + " L" + d.to.x + "," + d.to.y }));
        if (d.target) {
          var gt = geom(d.target);
          overlay.appendChild(svgEl("rect", { class: "dge-target", x: gt.x - 4, y: gt.y - 4, width: gt.w + 8, height: gt.h + 8, rx: 8 }));
        }
      }
      if (d && d.type === "branch" && d.target) {
        var tg = geom(d.target.id), mode = d.target.mode;
        if (mode === "child") overlay.appendChild(svgEl("rect", { class: "dge-target", x: tg.x - 5, y: tg.y - 5, width: tg.w + 10, height: tg.h + 10, rx: 8 }));
        else if (outerSide(d.target.id) === "down" || (mapInfo(d.target.id) || {}).layout === "down") {
          var lx = mode === "before" ? tg.x - 8 : tg.x + tg.w + 8;
          overlay.appendChild(svgEl("path", { class: "dge-drop", d: "M" + lx + "," + (tg.y - 4) + " L" + lx + "," + (tg.y + tg.h + 4) }));
        } else {
          var ly = mode === "before" ? tg.y - 5 : tg.y + tg.h + 5;
          overlay.appendChild(svgEl("path", { class: "dge-drop", d: "M" + (tg.x - 4) + "," + ly + " L" + (tg.x + tg.w + 4) + "," + ly }));
        }
      }
      if (d && d.type === "size") {
        overlay.appendChild(svgEl("rect", { class: "dge-size", x: d.x0, y: d.y0, width: d.w, height: d.h, rx: 6 }));
      }
      if (d && (d.type === "marquee" || d.type === "zoombox") && d.rect) {
        overlay.appendChild(svgEl("rect", { class: "dge-marquee", x: d.rect.x, y: d.rect.y, width: d.rect.w, height: d.rect.h }));
      }
      // dragging a task: its dates above the bar
      var moving = d && d.moved && (d.type === "resize" ? [d.id] : d.type === "move" && d.tasks ? d.items.filter(function (m) { return m.direct; }).map(function (m) { return m.id; }) : []);
      (moving || []).forEach(function (id) {
        var ch = chartBox(id), g = geom(id);
        if (!ch || !g || ch.origin == null) return;
        var ms = node(id).milestone;
        var from = ch.origin + Math.round((g.x + (ms ? g.w / 2 : 0) - ch.left) / ch.day), n = node(id).days || 1;
        var tip = h("div", { class: "dge-when" }, [ms ? shortDay(from) + " · milestone" : shortDay(from) + (n > 1 ? " – " + shortDay(from + n - 1) : "") + " · " + n + (n === 1 ? " day" : " days")]);
        var at = toScreen(g.x + g.w / 2, g.y);
        tip.style.left = at.x + "px";
        tip.style.top = at.y + "px";
        whens.push(tip);
        canvas.appendChild(tip);
      });
    }
    var whens = [];
    function shortDay(days) { return new Date(days * 86400000).toLocaleDateString("en-GB", { weekday: "short", day: "numeric", month: "short", timeZone: "UTC" }); }

    // ---------------------------------------------------------- interaction
    function setTool(t) {
      if (view && t !== "zoom") t = "select";
      st.tool = t;
      st.drag = null;
      applyChrome();
      drawOverlay();
      var tips = { mindmap: "Click to start a mindmap", node: "Click to place a box", decision: "Click to place a decision", sticky: "Click to place a sticky note", connect: "Drag from one node to another (or to empty space)", comment: "Click a block to comment on it, or anywhere on the canvas", zoom: "Click to zoom in · ⌥-click to zoom out · drag a box to zoom to it · Z or Esc to leave" };
      if (tips[t]) flash(tips[t]);
    }

    function select(ids, additive) {
      if (additive) {
        ids.forEach(function (id) {
          var i = st.sel.indexOf(id);
          if (i >= 0) st.sel.splice(i, 1);
          else st.sel.push(id);
        });
      } else st.sel = ids.slice();
      st.comment = null;
      markSelection();
      drawOverlay();
      renderPanel();
      emit("select", st.sel.slice());
    }

    function hit(target) {
      if (!target || !target.closest) return null;
      var gr0 = target.closest(".dge-grip");
      if (gr0) return { type: "grip", id: gr0.getAttribute("data-node") };
      var h1 = target.closest(".dge-handle");
      if (h1) return { type: "handle", id: h1.getAttribute("data-node") };
      var ctl = target.closest(".dge-ctl");
      if (ctl) return { type: ctl.classList.contains("dge-plus") ? "plus" : "fold", id: ctl.getAttribute("data-node") };
      var fold = target.closest(".dg-fold");
      if (fold) return { type: "fold", id: fold.getAttribute("data-fold") };
      var op = target.closest(".dg-open");
      if (op) return { type: "open", id: op.getAttribute("data-node"), canvas: op.getAttribute("data-canvas") };
      var lk = target.closest(".dg-link");
      if (lk) return { type: "link", id: lk.getAttribute("data-node"), link: lk.getAttribute("data-link") };
      // a risk's row in its matrix's list stands for the risk
      var rr = target.closest(".dg-risk-row");
      if (rr) return { type: "node", id: rr.getAttribute("data-node") };
      var dep = target.closest(".dg-dep");
      if (dep) return { type: "dep", id: dep.getAttribute("data-to"), from: dep.getAttribute("data-from") };
      var pin = target.closest(".dg-pin");
      if (pin) return { type: "pin", id: pin.getAttribute("data-comment") };
      var n = target.closest(".dg-node");
      if (n) return { type: "node", id: n.getAttribute("data-id") };
      var e = target.closest(".dg-edge");
      if (e) return { type: "edge", id: e.getAttribute("data-id") };
      var g = target.closest(".dg-group");
      if (g) return { type: "group", id: g.getAttribute("data-id") };
      // the inside of a plotted block (a screen, a chart…) stands for it
      var pl = target.closest(".dg-chart[data-group]");
      if (pl) return { type: "group", id: pl.getAttribute("data-group") };
      return null;
    }

    function membersOf(gid) {
      var out = [];
      var groups = [gid];
      var changed = true;
      while (changed) {
        changed = false;
        (st.doc.groups || []).forEach(function (g) {
          if (g.parent && groups.indexOf(g.parent) >= 0 && groups.indexOf(g.id) < 0) { groups.push(g.id); changed = true; }
        });
      }
      st.doc.nodes.forEach(function (n) { if (n.group && groups.indexOf(n.group) >= 0) out.push(n.id); });
      return out;
    }

    function addNodeAt(p, shape, groupId, thenEdit) {
      var sticky = shape === "sticky";
      var w = sticky ? 150 : 120, hgt = sticky ? 110 : 44;
      var id = freshId(sticky ? "note" : "node");
      var op = { op: "add_node", id: id, label: sticky ? "Note" : shape === "diamond" ? "Decision?" : "New box", x: Math.round(p.x - w / 2), y: Math.round(p.y - hgt / 2) };
      if (shape && shape !== "box") op.shape = shape;
      if (groupId) op.group = groupId;
      var res = applyOps([op]);
      if (res.ok) {
        select([id]);
        if (thenEdit) editLabel(id, true);
      }
      return res.ok ? id : null;
    }

    var raf = 0;
    /* Dragging boxes: what moves follows the pointer at once (its drawing is
       shifted), and the board catches up (arrows, sections around it) when
       the pointer rests, and on the drop. */
    function glide(d) {
      if (!d.els) {
        var ids = {};
        d.items.forEach(function (m) {
          ids[m.id] = true;
          if (group(m.id)) {
            membersOf(m.id).forEach(function (x) { ids[x] = true; });
            (st.doc.groups || []).forEach(function (g) { var c = g; while (c && c.parent) { if (c.parent === m.id) { ids[g.id] = true; break; } c = group(c.parent); } });
          }
        });
        d.moving = ids;
        d.els = [];
        Object.keys(ids).forEach(function (id) {
          stage.querySelectorAll('[data-id="' + cssEsc(id) + '"], [data-group="' + cssEsc(id) + '"], [data-fold="' + cssEsc(id) + '"]').forEach(function (el2) { d.els.push(el2); });
        });
        // the arrows to what moves are redrawn when it rests: faded till then
        d.edges = [];
        (st.doc.edges || []).forEach(function (ed) {
          if (ids[ed.from] || ids[ed.to]) stage.querySelectorAll('[data-id="' + cssEsc(ed.id) + '"]').forEach(function (el2) { d.edges.push(el2); });
        });
      }
      var ox = d.dx - (d.cx || 0), oy = d.dy - (d.cy || 0);
      d.els.forEach(function (el2) { el2.setAttribute("transform", "translate(" + ox + "," + oy + ")"); });
      d.edges.forEach(function (el2) { el2.style.opacity = "0.25"; });
      overlay.style.opacity = "0";
      clearTimeout(d.rest);
      d.rest = setTimeout(function () {
        if (st.drag !== d) return;
        applyOps(d.items.map(function (m) { return { op: "move", id: m.id, x: m.x + d.dx, y: m.y + d.dy }; }), true);
        d.cx = d.dx; d.cy = d.dy;
        d.els = null; // drawn anew where it rests
        overlay.style.opacity = "";
      }, 160);
    }
    function liveMove() {
      if (raf) return;
      raf = requestAnimationFrame(function () {
        raf = 0;
        var d = st.drag;
        if (!d || d.type !== "move") return;
        if (!d.tasks) return glide(d);
        // a task: show where it lands (snapped to its chart, the tasks after it following) and when
        st.session.set_source(d.before);
        refreshDoc();
        var ops = dropOps(d, true);
        if (!JSON.parse(st.session.apply(JSON.stringify({ ops: ops }))).ok) st.session.set_source(d.before);
        refreshDoc();
        render();
      });
    }

    /* Right-click actions, by kind of block. */
    function contextItems(t) {
      var items = itemsFor(t);
      var many = st.sel.length > 1 && st.sel.indexOf(t.id) >= 0;
      if (many) return [
        ["Copy as prompt", function () { copyPrompt(); }, "The selected blocks, the arrows between them and their open comments, as Markdown (" + MOD + "⇧C)"]
      ].concat(view ? [] : [
        ["Put in a section", function () { groupSelection(); }, MOD + "G"],
        "-",
        ["Delete", function () { deleteSelection(); }]
      ]);
      if (t.type === "edge" || t.type === "dep" || mapInfo(t.id)) return items;
      var copy = ["Copy as prompt", function () { copyPrompt([t.id]); }, "As Markdown to paste into a prompt (" + MOD + "⇧C)"];
      return view ? [copy] : [copy, "-"].concat(items);
    }
    function itemsFor(t) {
      if (t.type === "dep") return view ? [] : [
        ["Remove dependency", function () { dropDependency(t.id, t.from); }, "“" + labelOf(t.id) + "” stops waiting for “" + labelOf(t.from) + "”"]
      ];
      var items = baseItems(t);
      if (t.type !== "node" || view) return items;
      var lk = node(t.id).link;
      var links = [["Link to…", function () { startLinking(t.id); }, "Then click the element this one should take you to"],
        ["Link to a board…", function () { linkToBoard(t.id, st.menuAt); }, "Another board file; add #id to go to one element on it"]];
      if (lk) links.push(["Remove link", function () { applyOps([{ op: "update_node", id: t.id, link: "" }]); }, lk]);
      var cut = items.lastIndexOf("-");
      return cut > 0 ? items.slice(0, cut).concat(links, items.slice(cut)) : items.concat(["-"], links);
    }
    function baseItems(t) {
      var gr = t.type === "group" && group(t.id);
      if (t.type === "node" && !view && chartOf(t.id)) {
        var tk = node(t.id);
        return [
          ["Rename", function () { editLabel(t.id); }],
          tk.milestone ? null : ["Duration…", function () { askDuration(t.id); }, "Or drag the end of the bar"],
          ["Starts after…", function () { afterMenu(t.id); }, "Or draw an arrow from the task it waits for (in any chart)"],
          tk.milestone ? null : ["Progress…", function () { askProgress(t.id); }, "How much is done, like 60%"],
          ["Owner…", function () { askOwner(t.id); }, "Who does it"],
          [tk.milestone ? "Make it a task" : "Make it a milestone", function () { applyOps([{ op: "update_node", id: t.id, milestone: !tk.milestone }]); }, tk.milestone ? "A bar again" : "A point in time, drawn as a diamond"],
          ["Colour…", function () { colourMenu(t.id, "update_node", tk.color); }, "The colour of its bar"],
          tk.start ? ["Follow the task before", function () { applyOps([{ op: "update_node", id: t.id, start: "" }]); }, "Drop its fixed date"] : null,
          ["Add task after", function () { addTask(tk.group, t.id); }],
          "-",
          ["Delete", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
        ].filter(Boolean);
      }
      if (t.type === "node" && !view && entityOf(t.id)) {
        var col = node(t.id), eg = entityOf(t.id);
        return [
          ["Edit column…", function () {
            askAt(st.menuAt, columnText(col), "email: text [unique]", function (v) {
              var c = parseColumn(v);
              if (!c) return flash("Write `name: type [pk, unique, null, fk: table.column, rel: …]`", true);
              var cols = st.doc.nodes.filter(function (n) { return n.group === eg.id; }).map(function (n) { return n.id === col.id ? c : parseColumn(columnText(n)); });
              applyOps(entityOps(eg.id, cols));
            });
          }, "Its name, type and keys"],
          ["Edit columns…", function () { editEntity(eg.id); }, "The whole table as text"],
          ["Colour…", function () { colourMenu(t.id, "update_node", col.color); }, "Tint the row"],
          "-",
          ["Delete column", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
        ];
      }
      var fromSchema = gr && gr.entity && gr.parent && schemaOf(gr.parent);
      if (gr && !view && gr.entity) return [
        fromSchema && fromSchema.file && opts.sources ? ["Update from " + fromSchema.file, function () { updateFromFile(fromSchema.id); }, "Change the file, not the table: it is read again"] : null,
        ["Rename", function () { editLabel(t.id); }],
        ["Edit columns…", function () { editEntity(t.id); }, "name: type [pk, unique, null, fk: table.column]"],
        ["Add column", function () {
          askAt(st.menuAt, "", "name: type [null]", function (v) {
            var c = parseColumn(v);
            if (!c) return flash("Write `name: type [pk, unique, null, fk: table.column, rel: …]`", true);
            applyOps([{ op: "add_node", id: t.id + "." + c.name, label: c.name, group: t.id, column: c.column }]);
          });
        }],
        "-",
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The table and its columns"]
      ].filter(Boolean);
      if (t.type === "node" && !view && recordOf(t.id)) {
        var rf = node(t.id), rg = recordOf(t.id);
        return [
          ["Edit text…", function () {
            askAt(st.menuAt, (rf.detail || "").replace(/\n/g, "\\n"), "What it says", function (v) {
              applyOps([{ op: "update_node", id: t.id, detail: v.replace(/\\n/g, "\n") }]);
            });
          }, rf.label || rf.id],
          ["Rename field", function () {
            askAt(st.menuAt, rf.label || rf.id, "Field name", function (v) {
              var ops = [];
              ops.push({ op: "update_node", id: relabel(rf, rg.id, v, ops), label: v });
              applyOps(ops);
            });
          }],
          ["Colour…", function () { colourMenu(t.id, "update_node", rf.color); }, "Tint the field"],
          ["Edit as text…", function () { editRecord(rg.id); }, "The whole record"],
          "-",
          ["Delete field", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
        ];
      }
      if (gr && !view && gr.adr) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Status…", function () { statusMenu(t.id); }, gr.adr],
        ["Edit as text…", function () { editRecord(t.id); }, "One field per line: `context: …`"],
        ["Add field", function () {
          askAt(st.menuAt, "", "Field name, e.g. risks", function (v) {
            var id = freshId(t.id + "." + slug(v));
            if (applyOps([{ op: "add_node", id: id, label: v, group: t.id }]).ok) select([id]);
          });
        }],
        ["Colour…", function () { colourMenu(t.id, "update_group", gr.color); }],
        "-",
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }]
      ];
      if (t.type === "node" && !view && tableOf(t.id)) {
        var tr = node(t.id), tg = tableOf(t.id);
        return [
          ["Edit row…", function () {
            askAt(st.menuAt, [tr.label || tr.id].concat(tr.cells || []).join(" | "), (tg.columns || []).join(" | "), function (v) {
              var c = splitCells(v), ops = [];
              ops.push({ op: "update_node", id: relabel(tr, tg.id, c[0] || "—", ops), label: c[0] || "—", cells: c.slice(1) });
              applyOps(ops);
            });
          }, "Its cells, separated by |"],
          ["Add row below", function () {
            var all = st.doc.nodes.filter(function (n) { return n.group === tg.id; });
            applyOps([{ op: "add_node", id: freshId(tg.id + ".new-row"), label: "New row", group: tg.id, index: all.indexOf(tr) + 1 }]);
          }],
          ["Colour…", function () { colourMenu(t.id, "update_node", tr.color); }, "Tint the row"],
          ["Edit as text…", function () { editTable(tg.id); }, "The whole table"],
          "-",
          ["Delete row", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
        ];
      }
      if (t.type === "node" && !view && timelineOf(t.id)) {
        var ev = node(t.id), tlg = timelineOf(t.id);
        return [
          ["Rename", function () { editLabel(t.id); }],
          ["Change date…", function () {
            askAt(st.menuAt, ev.date || "", "e.g. 28 Oct 2026, Oct 2026, 7–14 May 2026", function (v) { if (v) applyOps([{ op: "update_node", id: t.id, date: v }]); });
          }, ev.date || ""],
          [ev.important ? "Not important" : "Mark important", function () { applyOps([{ op: "update_node", id: t.id, important: !ev.important }]); }, "Important events are starred and sit closest to the line"],
          [ev.detail ? "Edit details…" : "Add details…", function () { editDetails(t.id); }, "Shown on hover"],
          ["Colour…", function () { colourMenu(t.id, "update_node", ev.color); }],
          ["Edit as text…", function () { editTimeline(tlg.id); }, "The whole timeline"],
          "-",
          ["Delete event", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
        ];
      }
      if (t.type === "node" && !view && node(t.id) && node(t.id).group && group(node(t.id).group) && group(node(t.id).group).cycle) {
        var cg = group(node(t.id).group);
        return [
          ["Rename", function () { editLabel(t.id); }],
          ["Add step after…", function () {
            var all = st.doc.nodes.filter(function (n) { return n.group === cg.id; }), at = all.indexOf(node(t.id)) + 1;
            askAt(st.menuAt, "", "The next step", function (label) {
              if (!label) return;
              var base = cg.id + "." + slug(label), id = base, k = 2;
              while (idTaken(id)) id = base + "-" + k++;
              if (applyOps([{ op: "add_node", id: id, label: label, group: cg.id, index: at }]).ok) select([id]);
            });
          }],
          [node(t.id).detail ? "Edit details…" : "Add details…", function () { editDetails(t.id); }, "Shown on hover"],
          ["Colour…", function () { colourMenu(t.id, "update_node", node(t.id).color); }],
          ["Edit as text…", function () { editCycle(cg.id); }, "The whole cycle"],
          "-",
          ["Delete step", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
        ];
      }
      if (t.type === "node" && !view && node(t.id) && node(t.id).group && group(node(t.id).group) && group(node(t.id).group).screen) {
        var sg = group(node(t.id).group);
        return [
          ["Edit text", function () { editLabel(t.id); }],
          ["Add component after…", function () { select([t.id]); openWidgets(sg.id); }, "Fields, buttons, lists, tabs…"],
          ["Move up", function () { moveComponent(t.id, -1); }, "⌥↑"],
          ["Move down", function () { moveComponent(t.id, 1); }, "⌥↓"],
          [node(t.id).detail ? "Edit details…" : "Add details…", function () { editDetails(t.id); }, "A card's text, or a note"],
          ["Colour…", function () { colourMenu(t.id, "update_node", node(t.id).color); }, "Fills a button"],
          ["Edit screen as text…", function () { editScreen(sg.id); }, "One component per line"],
          ["Lead to another screen", function () { select([t.id]); setTool("connect"); flash("Drag from “" + (node(t.id).label || t.id) + "” to the screen it leads to"); }],
          "-",
          ["Delete component", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
        ];
      }
      if (t.type === "node" && !view && todoOf(t.id)) {
        var tl = todoOf(t.id), it = node(t.id);
        return [
          [doneOf(it) ? "Untick" : "Tick", function () { toggleTodo(t.id); }, "Or click its box"],
          ["Edit text", function () { editLabel(t.id); }],
          ["Add item after", function () { addTodoItem(tl.id, t.id); }, "Enter"],
          [it.level ? "Out a level" : "Make it a sub-item", function () { applyOps([{ op: "update_node", id: t.id, level: it.level ? it.level - 1 : 1 }]); }, it.level ? "⇧Tab" : "Tab"],
          ["Owner…", function () { askAt(st.menuAt, it.owner || "", "Who does it", function (v) { if (v !== null) applyOps([{ op: "update_node", id: t.id, owner: v.trim() }]); }); }],
          ["Due date…", function () { askAt(st.menuAt, it.date || "", "When, e.g. 2026-10-28 or 28 Oct 2026", function (v) { if (v !== null) applyOps([{ op: "update_node", id: t.id, date: v.trim() }]); }); }],
          [it.detail ? "Edit details…" : "Add details…", function () { editDetails(t.id); }, "Shown on hover"],
          ["Edit list as text…", function () { editTodo(tl.id); }, "One item per line"],
          "-",
          ["Delete item", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
        ];
      }
      if (gr && !view && gr.todo) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Add item", function () { addTodoItem(t.id, null); }],
        ["Edit as text…", function () { editTodo(t.id); }, "One item per line: [x] done, [ ] to do"],
        ["Colour…", function () { colourMenu(t.id, "update_group", gr.color); }],
        "-",
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The list and its items"]
      ];
      if (gr && !view && gr.schema) return [
        ["Rename", function () { editLabel(t.id); }],
        gr.file && opts.sources ? ["Update from " + gr.file, function () { updateFromFile(t.id); }, "Read the file again"] : null,
        ["Paste a new version…", function () { pasteSchema(t.id); }, "SQL or a Rails schema.rb"],
        ["Tables to show…", function () {
          askAt(st.menuAt, gr.only || "", "All, or e.g. users orders* billing.*", function (v) {
            if (v === null || !applyOps([{ op: "update_group", id: t.id, only: v.trim() }]).ok) return;
            if (gr.file && opts.sources) updateFromFile(t.id); else pasteSchema(t.id);
          });
        }, gr.only ? "Now: " + gr.only : "All of them now"],
        ["Colour…", function () { colourMenu(t.id, "update_group", gr.color); }],
        "-",
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The schema and its tables"]
      ].filter(Boolean);
      if (gr && !view && gr.screen) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Edit as text…", function () { editScreen(t.id); }, "One component per line"],
        ["Add component…", function () { select([t.id]); openWidgets(t.id); }, "Fields, buttons, lists, tabs…"],
        ["Device…", function () {
          setTimeout(function () {
            openMenu(st.menuAt, [["mobile", "Phone"], ["tablet", "Tablet"], ["web", "Web page"]].map(function (d) {
              return [(gr.screen === d[0] ? "✓ " : "\u2003") + d[1], function () { applyOps([{ op: "update_group", id: t.id, screen: d[0] }]); }];
            }));
          }, 0);
        }],
        "-",
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The screen and its components"]
      ];
      if (gr && !view && gr.cycle) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Edit as text…", function () { editCycle(t.id); }, "One step per line"],
        ["Colour…", function () { colourMenu(t.id, "update_group", gr.color); }],
        "-",
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The cycle and its steps"]
      ];
      if (gr && !view && gr.timeline) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Add event…", function () {
          askAt(st.menuAt, "", "DATE: what happened, e.g. 28 Oct 2026: Open beta", function (v) {
            var e = v && eventLine(v);
            if (!e) return v && flash("Write it as DATE: what happened", true);
            var base = t.id + "." + slug(e.label || e.date), id = base, k = 2;
            while (idTaken(id)) id = base + "-" + k++;
            applyOps([{ op: "add_node", id: id, label: e.label, group: t.id, date: e.date, important: e.important }]);
          });
        }],
        ["Edit as text…", function () { editTimeline(t.id); }, "One event per line: DATE: what happened"],
        ["Colour…", function () { colourMenu(t.id, "update_group", gr.color); }],
        "-",
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The timeline and its events"]
      ];
      if (gr && !view && gr.table) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Edit as text…", function () { editTable(t.id); }, "Rows and columns, separated by |"],
        ["Show as timeline", function () { tableToTimeline(t.id); }, "When the first column is dates: each row becomes an event"],
        ["Add row", function () {
          var rows = [gr.columns || []].concat(st.doc.nodes.filter(function (n) { return n.group === t.id; }).map(function (n) { return [n.label || n.id].concat(n.cells || []); }));
          rows.push(["New row"]);
          applyOps(tableOps(t.id, rows));
        }],
        ["Add column", function () {
          askAt(st.menuAt, "", "Column name", function (v) { applyOps([{ op: "update_group", id: t.id, columns: (gr.columns || []).concat([v]) }]); });
        }],
        "-",
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The table and its rows"]
      ];
      if (t.type === "node" && !view && plotOf(t.id)) {
        var row = node(t.id), pg = plotOf(t.id);
        return [
          ["Rename", function () { askAt(st.menuAt, row.label || row.id, "Label", function (v) { applyOps([{ op: "update_node", id: t.id, label: v }]); }); }],
          ["Values…", function () {
            askAt(st.menuAt, (row.values || []).map(function (v) { return v == null ? "-" : v; }).join(", "), (pg.series || []).join(", ") || "120", function (v) {
              var vals = v.split(/[,;|]/).map(cellValue);
              if (vals.some(function (x) { return x === undefined; })) { flash("Write numbers separated by commas (- for none)", true); return; }
              applyOps([{ op: "update_node", id: t.id, values: vals }]);
            });
          }, (pg.series || []).length ? "One per series: " + pg.series.join(", ") : "Its value"],
          ["Colour…", function () { colourMenu(t.id, "update_node", row.color); }, "Its own colour (single-series charts)"],
          ["Edit data…", function () { editChartData(pg.id); }, "The whole table"],
          "-",
          ["Delete", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
        ];
      }
      if (gr && !view && gr.chart) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Edit data…", function () { editChartData(t.id); }, "Rows and series as a table"],
        ["Type…", function () { chartTypeMenu(t.id); }, "Bars, lines, areas, pie, donut or scatter"],
        gr.chart === "pie" || gr.chart === "donut" || gr.chart === "scatter" ? null : [gr.stacked ? "Side by side" : "Stacked", function () { applyOps([{ op: "update_group", id: t.id, stacked: !gr.stacked }]); }],
        gr.chart === "bar" ? [gr.horizontal ? "Vertical bars" : "Horizontal bars", function () { applyOps([{ op: "update_group", id: t.id, horizontal: !gr.horizontal }]); }] : null,
        ["Unit…", function () { askAt(st.menuAt, gr.unit || "", "%, k€, ms, $", function (v) { applyOps([{ op: "update_group", id: t.id, unit: v }]); }); }, "What the values are in"],
        "-",
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The chart and its data"]
      ].filter(Boolean);
      if (t.type === "node" && !view && matrixOf(t.id)) {
        var rk = node(t.id);
        return [
          ["Rename", function () { askAt(st.menuAt, rk.label || rk.id, "What could go wrong?", function (v) { applyOps([{ op: "update_node", id: t.id, label: v }]); }); }],
          ["Probability…", function () { levelMenu(t.id, "probability"); }, "How likely (or drag the dot up or down)"],
          ["Impact…", function () { levelMenu(t.id, "impact"); }, "How bad (or drag the dot left or right)"],
          ["Description…", function () { askAt(st.menuAt, rk.detail || "", "What it is, what we do about it", function (v) { applyOps([{ op: "update_node", id: t.id, detail: v }]); }); }, "Shown when you hover the dot"],
          ["Owner…", function () { askOwner(t.id); }, "Who watches it"],
          "-",
          ["Delete", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
        ];
      }
      if (gr && !view && gr.risks) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Add risk", function () { addRisk(t.id); }],
        ["Size…", function () { levelsMenu(t.id); }, "3 × 3, 4 × 4 or 5 × 5"],
        "-",
        ["Remove matrix", function () { applyOps([{ op: "remove", id: t.id, keep_members: true }]); select([]); }, "Its risks stay, as boxes"],
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The matrix and its risks"]
      ];
      if (gr && !view && gr.gantt) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Add task", function () { addTask(t.id); }],
        ["Scale…", function () { scaleMenu(t.id); }, "Days, weeks, months or quarters"],
        "-",
        ["Remove chart", function () { applyOps([{ op: "remove", id: t.id, keep_members: true }]); select([]); }, "Its tasks stay, as boxes"],
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The chart and its tasks"]
      ];
      if (t.type === "node" && !mapInfo(t.id) && !view && canvasOf(t.id)) {
        var cv = canvasOf(t.id);
        return [
          ["Open canvas", function () { openLinked(t.id); }, "Or double-click it"],
          ["Rename", function () { editLabel(t.id); }, "The link's text"],
          ["Rename canvas…", function () {
            askAt(st.menuAt, cv.title || cv.id, "Canvas name", function (v) { if (v) applyOps([{ op: "update_canvas", id: cv.id, title: v }]); });
          }, cv.title || cv.id],
          ["Link style…", function () { linkStyleMenu(t.id); }, "Button, card, text…"],
          ["Colour…", function () { colourMenu(t.id, "update_node", node(t.id).color); }],
          "-",
          ["Delete", function () { confirmCanvases([t.id], function () { applyOps([{ op: "remove", id: t.id }]); select([]); }); }, "The link and its canvas"]
        ];
      }
      if (t.type === "node" && !mapInfo(t.id) && !view) return [
        ["Rename", function () { editLabel(t.id); }],
        [node(t.id).detail ? "Edit details…" : "Add details…", function () { editDetails(t.id); }, "More context: shown under the title, or on hover for a kanban card"],
        ["Shape…", function () { shapeMenu(t.id); }],
        ["Colour…", function () { colourMenu(t.id, "update_node", node(t.id).color); }],
        "-",
        ["Delete", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
      ];
      if (t.type === "edge" && !view) {
        var ed = edge(t.id), msgs = messagesOf(ed), k = msgs.indexOf(ed);
        return (k >= 0 ? [
          k > 0 ? ["Move up", function () { applyOps([{ op: "update_edge", id: t.id, index: k - 1 }]); }, "Earlier in the sequence"] : null,
          k < msgs.length - 1 ? ["Move down", function () { applyOps([{ op: "update_edge", id: t.id, index: k + 1 }]); }, "Later in the sequence"] : null,
          "-"
        ].filter(Boolean) : []).concat([
          [ed.label ? "Edit label" : "Add label", function () { editLabel(t.id); }],
          [ed.line === "dashed" ? "Solid line" : "Dashed line", function () { applyOps([{ op: "update_edge", id: t.id, line: ed.line === "dashed" ? "solid" : "dashed" }]); }],
          ["Reverse direction", function () { applyOps([{ op: "update_edge", id: t.id, from: ed.to, to: ed.from }]); select([]); }],
          ["Colour…", function () { colourMenu(t.id, "update_edge", ed.color); }],
          "-",
          ["Delete", function () { applyOps([{ op: "remove", id: t.id }]); select([]); }]
        ]);
      }
      if (gr && !view && gr.sequence) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Add participant", function () {
          var id = freshId("participant");
          if (!applyOps([{ op: "add_node", id: id, label: "Participant", group: t.id }]).ok) return;
          select([id]);
          editLabel(id, true);
        }, "Then connect two participants to add a message"],
        ["Colour…", function () { colourMenu(t.id, "update_group", gr.color); }],
        "-",
        ["Remove sequence diagram", function () { applyOps([{ op: "remove", id: t.id, keep_members: true }]); select([]); }, "The participants and arrows stay"],
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The diagram, its participants and messages"]
      ];
      if (gr && !view && gr.kanban) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Add column", function () { addLane(t.id); }],
        "-",
        ["Remove kanban", function () { applyOps(lanesOf(t.id).map(function (l) { return { op: "remove", id: l.id, keep_members: true }; }).concat([{ op: "remove", id: t.id, keep_members: true }])); select([]); }, "The columns go; the cards stay as boxes"],
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The board and all its cards"]
      ];
      if (gr && !view && gr.parent && group(gr.parent) && group(gr.parent).kanban) return [
        ["Add card", function () { addCard(t.id); }],
        ["Rename", function () { editLabel(t.id); }],
        ["Add column after", function () { addLane(gr.parent, t.id); }],
        ["Colour…", function () { colourMenu(t.id, "update_group", gr.color); }],
        "-",
        ["Remove column", function () { applyOps([{ op: "remove", id: t.id, keep_members: true }]); select([]); }, "Its cards stay on the board"],
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The column and its cards"]
      ];
      if (gr && !view && gr.lanes) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Add lane", function () { addLane(t.id); }],
        "-",
        ["Remove swimlanes", function () { applyOps(lanesOf(t.id).map(function (l) { return { op: "remove", id: l.id, keep_members: true }; }).concat([{ op: "remove", id: t.id, keep_members: true }])); select([]); }, "The lanes go; the blocks stay"],
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The swimlanes and everything in them"]
      ];
      if (gr && !view && gr.parent && group(gr.parent) && group(gr.parent).lanes) return [
        ["Rename", function () { editLabel(t.id); }],
        ["Add lane after", function () { addLane(gr.parent, t.id); }],
        ["Colour…", function () { colourMenu(t.id, "update_group", gr.color); }],
        "-",
        ["Remove lane", function () { applyOps([{ op: "remove", id: t.id, keep_members: true }]); select([]); }, "Its blocks stay in the swimlanes"],
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The lane and its blocks"]
      ];
      if (t.type === "group") return view ? [] : [
        ["Rename", function () { editLabel(t.id); }],
        ["Colour…", function () { colourMenu(t.id, "update_group", gr.color); }],
        ["Remove section", function () { applyOps([{ op: "remove", id: t.id, keep_members: true }]); select([]); }, "The blocks stay where they are"],
        ["Delete", function () { applyOps([{ op: "remove", id: t.id, keep_members: false }]); select([]); }, "The section and everything in it"]
      ];
      if (t.type === "node" && mapInfo(t.id)) return [
        ["Add parent", function () { addParent(t.id); }, mapInfo(t.id).parent ? "A new topic between this one and its parent" : "A new centre above this one (⇧Tab)"],
        [mapInfo(t.id).parent ? "Branch colour…" : "Colour…", function () { colourMenu(t.id, "update_node", node(t.id).color); }, mapInfo(t.id).parent ? "This topic and everything under it" : null],
        "-",
        ["Copy as prompt", function () { copyPrompt([t.id]); }, "This topic and everything under it as Markdown (" + MOD + "⇧C)"]
      ].filter(function (it) { return !view || it[0] === "Copy as prompt"; });
      return [];
    }
    /* Pick a shape for a box, at the spot of the menu that led here. */
    function shapeMenu(id) {
      var cur = node(id).shape || "box";
      setTimeout(function () {
        openMenu(st.menuAt, SHAPES.map(function (sh) {
          return [(sh === cur ? "✓ " : "\u2003") + sh.charAt(0).toUpperCase() + sh.slice(1), function () { applyOps([{ op: "update_node", id: id, shape: sh }]); }];
        }));
      });
    }
    /* Pick a colour (or the default) for a block, an arrow or a section. */
    function colourMenu(id, op, cur) {
      setTimeout(function () {
        openMenu(st.menuAt, [""].concat(COLORS).map(function (c) {
          var label = h("span", { class: "dge-colour" }, [h("i", { class: c ? "" : "none", style: c ? "background:" + SWATCH[c] : null }), (c || "default").charAt(0).toUpperCase() + (c || "default").slice(1)]);
          if (c === (cur || "")) label.appendChild(document.createTextNode(" ✓"));
          return [label, function () { var o = { op: op, id: id, color: c }; applyOps([o]); }];
        }));
      });
    }
    /* The whole board, every canvas in it, as a prompt to build what it
       describes: paste it into any LLM or coding agent. */
    function copyBoardPrompt() {
      var board = st.answer ? st.answer.main.session : st.session;
      copyText(board.build_prompt(isoDay(today()))).then(function () {
        flash("The whole board is copied as a prompt: paste it into Claude, ChatGPT or your coding agent to build it");
      });
    }
    /* The selection (or `ids`) as Markdown for a prompt. */
    function copyPrompt(ids) {
      ids = (ids || st.sel).filter(kindOf);
      if (!ids.length) return;
      var n = ids.length === 1 && (node(ids[0]) || group(ids[0])), label = n && (n.label || n.id);
      copyText(st.session.selection_markdown_at(JSON.stringify(ids), isoDay(today()))).then(function () {
        flash(n ? "Prompt copied — “" + label + "”" + (mapInfo(ids[0]) || group(ids[0]) ? " and everything in it" : "") : "Prompt copied — " + ids.length + " items");
      });
    }
    canvas.addEventListener("contextmenu", function (e) {
      var t = st.r && hit(e.target);
      var at = toDiagram(e.clientX, e.clientY);
      var items = t ? contextItems(t) : view ? [] : [
        ["✦ Rethink the board", function () { rethink(); }, "The AI acts on the open comments and brings the whole board up to date; you see the changes first"],
        ["Copy the board as a prompt", function () { copyBoardPrompt(); }, "Everything on it, canvases too, as a spec to build from (" + MOD + "⇧C)"],
        ["New canvas here…", function () { newCanvas(at); }, "A board inside this one, opened from a button here"],
        ["Insert recipe…", function () { recipeMenu(); }, "Incidents, OKRs, a burndown, a retro…: ready-made arrangements of blocks"],
        ["Insert image…", function () { pickImages(at); }, "Or paste an image, or drop image files on the board"]
      ];
      if (!items.length) return;
      e.preventDefault();
      if (t && (t.type === "node" || t.type === "edge" || t.type === "group") && st.sel.indexOf(t.id) < 0) select([t.id]);
      st.menuAt = { x: e.clientX, y: e.clientY };
      openMenu(st.menuAt, items);
    });

    // a box with a web link is an <a> in the drawing: the editor follows links
    // from their badge (and never while placing something), not the browser
    canvas.addEventListener("click", function (e) {
      if (e.target.closest && e.target.closest(".dge-stage a")) e.preventDefault();
    }, true);
    canvas.addEventListener("pointerdown", function (e) {
      if (!st.r) return;
      canvas.focus({ preventScroll: true });
      closeMenu();
      var p = toDiagram(e.clientX, e.clientY);
      var t = hit(e.target);
      // with a tool that puts something on the board (a comment, a box…), a
      // link or canvas badge is part of its box: nothing is followed
      if (t && (t.type === "link" || t.type === "open") && st.tool !== "select" && st.tool !== "hand" && !view) t = { type: "node", id: t.id };
      if (e.button === 0 && t && t.type === "link" && !st.linking) { e.preventDefault(); followLink(t.link); return; }
      if (e.button === 0 && t && t.type === "open") { e.preventDefault(); openCanvas(st.canvasPath.concat([t.canvas])); return; }
      // a to-do item's checkbox: a click ticks it
      // (the connector handle on its left sits on the box: a click there ticks too)
      if (t && (t.type === "node" || t.type === "handle") && !view && e.button === 0 && todoOf(t.id) && st.tool === "select") {
        var tg = geom(t.id);
        if (tg && p.x - tg.x < 26) { e.preventDefault(); toggleTodo(t.id); return; }
      }
      // a folded block in a perspective: it opens in place
      if (t && t.type === "node" && /^@fold\//.test(t.id)) { e.preventDefault(); if (e.button === 0) unfold(t.id.slice(6)); return; }
      // the magnifier (Z): a click zooms in, ⌥-click out, a box zooms to it
      if (st.tool === "zoom" && e.button === 0 && !st.space) {
        e.preventDefault();
        st.drag = { type: "zoombox", sx: p.x, sy: p.y, out: e.altKey };
        canvas.setPointerCapture(e.pointerId);
        return;
      }
      if (st.linking && e.button === 0) {
        // Link to…: the next click picks the target
        var from = st.linking, to = t && (t.type === "node" || t.type === "group") ? t.id : null;
        endLinking();
        e.preventDefault();
        if (to && to !== from) { applyOps([{ op: "update_node", id: from, link: "#" + to }]); flash("Linked to “" + labelOf(to) + "”"); }
        return;
      }
      if (e.button === 1 || st.space || (e.button === 0 && (st.tool === "hand" || view) && !(t && t.type === "pin"))) {
        st.drag = { type: "pan", sx: e.clientX, sy: e.clientY, px: st.panX, py: st.panY };
        canvas.setPointerCapture(e.pointerId);
        canvas.classList.add("panning");
        return;
      }
      if (e.button !== 0) return;
      closeBubble();
      // the popovers take the focus: keep the browser from handing it back to the canvas
      if (t && t.type === "pin") { e.preventDefault(); openThread(t.id, e.clientX, e.clientY); return; }
      if (st.tool === "comment") {
        var on = t && (t.type === "node" || t.type === "edge" || t.type === "group") ? t.id : null;
        e.preventDefault();
        if (on) select([on]);
        openComposer(on ? { on: on } : { x: Math.round(p.x), y: Math.round(p.y) }, e.clientX, e.clientY);
        return;
      }
      if (t && t.type === "plus") { e.preventDefault(); addTopic(t.id); return; }
      if (t && t.type === "fold") { toggleFold(t.id); select([t.id]); return; }
      if (st.tool === "mindmap") {
        setTool("select");
        var mid = freshId("map");
        e.preventDefault(); // keep the focus in the label editor
        var mres = applyOps([{ op: "add_node", id: mid, label: "", mindmap: "right", x: Math.round(p.x - 60), y: Math.round(p.y - 24) }]);
        if (mres.ok) { select([mid]); editLabel(mid, true, true); }
        return;
      }
      if (st.tool === "node" || st.tool === "sticky" || st.tool === "decision") {
        e.preventDefault();
        var gid = t && t.type === "group" ? t.id : t && t.type === "node" ? (node(t.id) || {}).group : null;
        addNodeAt(p, { sticky: "sticky", decision: "diamond" }[st.tool] || "box", gid, true);
        setTool("select");
        return;
      }
      if (t && t.type === "grip" && !view) {
        // the corner of a box: drag to resize it
        var sg = geom(t.id);
        st.drag = { type: "size", id: t.id, sx: p.x, sy: p.y, x0: sg.x, y0: sg.y, w0: sg.w, h0: sg.h, w: sg.w, h: sg.h };
        canvas.setPointerCapture(e.pointerId);
        drawOverlay();
        return;
      }
      if (t && (t.type === "handle" || (st.tool === "connect" && t.type === "node"))) {
        st.drag = { type: "connect", from: t.id, to: p };
        // from a handle: the line starts there, and a click (no drag) puts a
        // new box on that side
        if (t.type === "handle") {
          var hg = geom(t.id);
          var sides = [["top", hg.x + hg.w / 2, hg.y], ["right", hg.x + hg.w, hg.y + hg.h / 2], ["bottom", hg.x + hg.w / 2, hg.y + hg.h], ["left", hg.x, hg.y + hg.h / 2]];
          var near = sides.reduce(function (a, b) { return Math.hypot(b[1] - p.x, b[2] - p.y) < Math.hypot(a[1] - p.x, a[2] - p.y) ? b : a; });
          st.drag.start = { x: near[1], y: near[2] };
          st.drag.side = near[0];
          st.drag.sx = p.x; st.drag.sy = p.y;
        }
        canvas.setPointerCapture(e.pointerId);
        drawOverlay();
        return;
      }
      if (t && t.type === "node" && !view && barEnd(t.id, e.clientX)) {
        // the end of a bar: drag to change how long the task takes
        select([t.id]);
        st.drag = { type: "resize", id: t.id, sx: p.x, days: node(t.id).days || 1, before: st.text, moved: false };
        canvas.setPointerCapture(e.pointerId);
        return;
      }
      if (t && (t.type === "node" || t.type === "group" || t.type === "edge")) {
        if (e.shiftKey || e.metaKey) select([t.id], true);
        else if (st.sel.indexOf(t.id) < 0) select([t.id]);
        if (t.type === "edge") return;
        if (isBranch(t.id) && !e.shiftKey && !e.metaKey) {
          // dragging a branch gives it a new place in the map
          st.drag = { type: "branch", id: t.id, sx: p.x, sy: p.y, items: subtreeOf(t.id), moved: false };
          canvas.setPointerCapture(e.pointerId);
          return;
        }
        // everything selected moves; groups carry their members
        var ids = [], direct = [];
        st.sel.forEach(function (id) {
          if (isBranch(id)) return;
          if (plotOf(id)) ids.push(plotOf(id).id); // a chart's row: the chart moves
          else if (node(id)) { ids.push(id); direct.push(id); }
          else if (blockOf(id)) ids.push(blockOf(id));
          else if (group(id)) membersOf(id).forEach(function (m) { ids.push(blockOf(node(m).group) || m); });
        });
        var items = [];
        ids.forEach(function (id) {
          var g = geom(id);
          if (g && !items.some(function (i) { return i.id === id; })) items.push({ id: id, x: g.x, y: g.y, w: g.w, h: g.h, direct: direct.indexOf(id) >= 0 });
        });
        // sections as they are now: while dragging, they stretch around what moves
        var sections = (st.r.groups || []).map(function (g) { return { id: g.id, x: g.x, y: g.y, w: g.w, h: g.h, lane: !!g.lane || !!g.sequence, seq: !!g.sequence, gantt: g.gantt || null, matrix: g.matrix || null }; });
        st.drag = { type: "move", sx: p.x, sy: p.y, dx: 0, dy: 0, items: items, sections: sections, before: st.text, moved: false,
          tasks: items.some(function (m) { return m.direct && (chartOf(m.id) || matrixOf(m.id)); }) };
        canvas.setPointerCapture(e.pointerId);
        return;
      }
      if (!e.shiftKey) select([]);
      st.drag = { type: "marquee", sx: p.x, sy: p.y, additive: e.shiftKey };
      canvas.setPointerCapture(e.pointerId);
    });

    canvas.addEventListener("pointermove", function (e) {
      if (!st.r) return;
      var d = st.drag;
      if (!d) {
        var t = hit(e.target);
        canvas.classList.toggle("resizing", !!(t && t.type === "node" && !view && barEnd(t.id, e.clientX)));
        var hov = t && (t.type === "node" || t.type === "handle") ? t.id : null;
        if (hov !== st.hover) {
          st.hover = hov;
          drawOverlay();
        }
        return;
      }
      if (d.type === "pan") {
        st.panX = d.px + e.clientX - d.sx;
        st.panY = d.py + e.clientY - d.sy;
        transform();
        return;
      }
      var p = toDiagram(e.clientX, e.clientY);
      if (d.type === "resize") {
        var ch = chartBox(d.id), step = snapDays(ch.scale);
        var days = Math.max(step, Math.round((d.days * ch.day + p.x - d.sx) / ch.day / step) * step);
        if (!d.moved && Math.abs(p.x - d.sx) < 3) return;
        d.moved = true;
        if (days !== d.now) { d.now = days; applyOps([{ op: "update_node", id: d.id, days: days }], true); }
        return;
      }
      if (d.type === "size") {
        d.w = Math.max(40, Math.round(d.w0 + p.x - d.sx));
        d.h = Math.max(24, Math.round(d.h0 + p.y - d.sy));
        drawOverlay();
        return;
      }
      if (d.type === "move") {
        d.dx = Math.round(p.x - d.sx);
        d.dy = Math.round(p.y - d.sy);
        if (!d.moved && Math.abs(d.dx) + Math.abs(d.dy) < 3) return;
        d.moved = true;
        liveMove();
      } else if (d.type === "branch") {
        var bx = p.x - d.sx, by = p.y - d.sy;
        if (!d.moved && Math.abs(bx) + Math.abs(by) < 4) return;
        d.bx = bx; d.by = by;
        d.moved = true;
        d.items.forEach(function (id) {
          stage.querySelectorAll('[data-id="' + cssEsc(id) + '"],[data-fold="' + cssEsc(id) + '"]').forEach(function (el2) {
            el2.setAttribute("transform", "translate(" + bx + "," + by + ")");
            el2.classList.add("dragging");
          });
        });
        var hits = document.elementsFromPoint(e.clientX, e.clientY);
        d.target = null;
        for (var hi = 0; hi < hits.length; hi++) {
          var th = hit(hits[hi]);
          if (!th || th.type !== "node" || d.items.indexOf(th.id) >= 0 || !mapInfo(th.id)) continue;
          var tg = geom(th.id), tm = mapInfo(th.id), mode = "child";
          if (tm.parent) {
            var rel = tm.layout === "down" ? (p.x - tg.x) / tg.w : (p.y - tg.y) / tg.h;
            mode = rel < 0.28 ? "before" : rel > 0.72 ? "after" : "child";
          }
          d.target = { id: th.id, mode: mode };
          break;
        }
        drawOverlay();
      } else if (d.type === "connect") {
        d.to = p;
        var under = document.elementFromPoint(e.clientX, e.clientY);
        var t2 = hit(under);
        d.target = t2 && (t2.type === "node" || t2.type === "group" || t2.type === "handle") && t2.id !== d.from ? t2.id : null;
        drawOverlay();
      } else if (d.type === "marquee" || d.type === "zoombox") {
        d.rect = { x: Math.min(d.sx, p.x), y: Math.min(d.sy, p.y), w: Math.abs(p.x - d.sx), h: Math.abs(p.y - d.sy) };
        drawOverlay();
      }
    });

    function endDrag(e) {
      var d = st.drag;
      st.drag = null;
      canvas.classList.remove("panning");
      if (!d) return;
      if (d.type === "branch") {
        if (d.moved && d.target) {
          var tid = d.target.id, op;
          if (d.target.mode === "child") op = { op: "update_node", id: d.id, parent: tid };
          else {
            var par = node(tid).parent;
            var sibs = kidsOf(par).filter(function (x) { return x !== d.id; });
            op = { op: "update_node", id: d.id, parent: par, index: sibs.indexOf(tid) + (d.target.mode === "after" ? 1 : 0) };
          }
          var ops2 = [];
          if (d.target.mode === "child" && node(tid).collapsed) ops2.push({ op: "update_node", id: tid, collapsed: false });
          ops2.push(op);
          if (!applyOps(ops2).ok) render();
        } else if (d.moved) {
          // dropped on empty canvas: the topic stays there (relative to its centre)
          var tg = geom(d.id), cg = geom(mapInfo(d.id).root);
          if (!applyOps([{ op: "move", id: d.id, x: Math.round(tg.x + d.bx - cg.x), y: Math.round(tg.y + d.by - cg.y) }]).ok) render();
        }
        drawOverlay();
        return;
      }
      if (d.type === "resize") {
        if (d.moved && d.now) {
          st.session.set_source(d.before);
          st.text = d.before;
          applyOps([{ op: "update_node", id: d.id, days: d.now }]);
        }
        return;
      }
      if (d.type === "size") {
        if (d.w !== d.w0 || d.h !== d.h0) applyOps([{ op: "update_node", id: d.id, w: d.w, h: d.h }]);
        select([d.id]);
        return;
      }
      if (d.type === "move" && d.moved) {
        if (raf) { cancelAnimationFrame(raf); raf = 0; }
        clearTimeout(d.rest);
        overlay.style.opacity = "";
        st.session.set_source(d.before);
        st.text = d.before;
        refreshDoc();
        var ops = dropOps(d);
        if (!applyOps(ops).ok) { refreshDoc(); render(); }
      } else if (d.type === "connect" && e) {
        var p = toDiagram(e.clientX, e.clientY);
        var clicked = d.side && Math.hypot(p.x - d.sx, p.y - d.sy) < 6;
        if (clicked) {
          // a click on a handle: a new box on that side, joined to this one
          var fg = geom(d.from), gap = 90, bw = 140, bh = 44;
          var at = d.side === "right" ? { x: fg.x + fg.w + gap, y: fg.y + fg.h / 2 - bh / 2 }
            : d.side === "left" ? { x: fg.x - gap - bw, y: fg.y + fg.h / 2 - bh / 2 }
            : d.side === "bottom" ? { x: fg.x + fg.w / 2 - bw / 2, y: fg.y + fg.h + gap * 0.7 }
            : { x: fg.x + fg.w / 2 - bw / 2, y: fg.y - gap * 0.7 - bh };
          var src0 = node(d.from), nid = freshId("node");
          var ops0 = [{ op: "add_node", id: nid, label: "New box", x: Math.round(at.x), y: Math.round(at.y), group: src0 && src0.group && !plotOf(d.from) && !screenOf(d.from) && !todoOf(d.from) ? src0.group : undefined }];
          ops0.push({ op: "add_edge", from: d.from, to: nid });
          if (applyOps(ops0).ok) { select([nid]); editLabel(nid, true); }
          drawOverlay();
          return;
        }
        if (d.target && chartOf(d.from) && chartOf(d.target)) dependOn(d.target, d.from);
        else if (d.target) applyOps([{ op: "add_edge", from: d.from, to: d.target }]);
        else if (Math.hypot(p.x - (geom(d.from).x + geom(d.from).w / 2), p.y - (geom(d.from).y + geom(d.from).h / 2)) > 60) {
          // dropped on empty canvas: create a connected box there (from a
          // screen's component: the next screen, of the same device)
          var before = st.text;
          var src = node(d.from);
          var scr = src && src.group && group(src.group) && group(src.group).screen;
          if (scr) {
            var sid = freshId("screen");
            // beside the screen it comes from, not on top of it
            var from = st.r.groups.find(function (x) { return x.id === src.group; });
            var at = from ? { x: from.x + from.w + 100, y: from.y } : { x: p.x, y: p.y - 40 };
            var taken = function (q) { return st.r.groups.some(function (x) { return !group(x.id).parent && q.x < x.x + x.w && q.x + 400 > x.x && q.y < x.y + x.h && q.y + 300 > x.y; }); };
            for (var k = 0; k < 12 && taken(at); k++) at = { x: at.x, y: at.y + (from ? from.h + 60 : 300) };
            if (applyOps([
              { op: "merge", dsl: "screen " + sid + ": Next screen [" + scr + "] {\n" + SCREEN_STARTERS[scr === "web" ? "web" : "mobile"] + "\n}" },
              { op: "move", id: sid, x: Math.round(at.x), y: Math.round(at.y) },
              { op: "add_edge", from: d.from, to: sid }
            ]).ok) { select([sid]); editLabel(sid, true); }
            drawOverlay();
            return;
          }
          var id = freshId("node");
          var res = JSON.parse(st.session.apply(JSON.stringify({ ops: [
            { op: "add_node", id: id, label: "New box", x: Math.round(p.x - 60), y: Math.round(p.y - 22), group: src && src.group ? src.group : undefined },
            { op: "add_edge", from: d.from, to: id }
          ] })));
          if (res.ok) {
            st.undo.push(before);
            st.redo = [];
            refreshDoc();
            st.text = st.session.source("dsl");
            afterChange(true);
            select([id]);
            editLabel(id, true);
          } else flash(res.error, true);
        }
        drawOverlay();
      } else if (d.type === "zoombox") {
        var zr = d.rect;
        if (zr && zr.w * st.zoom > 8 && zr.h * st.zoom > 8) focusOn(zr, 4, 4, 0.95);
        else {
          var zs = toScreen(d.sx, d.sy);
          zoomTo(st.zoom * (d.out || (e && e.altKey) ? 1 / 1.6 : 1.6), zs.x, zs.y);
        }
        drawOverlay();
      } else if (d.type === "marquee" && d.rect && (d.rect.w > 3 || d.rect.h > 3)) {
        var r = d.rect;
        var ids = st.r.nodes.filter(function (n) { return n.x < r.x + r.w && n.x + n.w > r.x && n.y < r.y + r.h && n.y + n.h > r.y; }).map(function (n) { return n.id; });
        select(ids, d.additive);
      } else drawOverlay();
    }
    canvas.addEventListener("pointerup", endDrag);
    canvas.addEventListener("pointercancel", function () { endDrag(null); });
    canvas.addEventListener("pointerleave", function () { if (st.hover && !st.drag) { st.hover = null; drawOverlay(); } });

    canvas.addEventListener("dblclick", function (e) {
      if (view || !st.r || st.tool === "hand") return;
      // the first click may have re-rendered what is under the pointer
      var under = document.elementFromPoint(e.clientX, e.clientY) || e.target;
      var t = hit(under);
      if (t && t.type === "node" && /^@fold\//.test(t.id)) return;
      if (t && t.type === "node" && canvasOf(t.id)) { openLinked(t.id); return; }
      if (t && (t.type === "node" || t.type === "edge" || t.type === "group")) {
        editLabel(t.id);
        return;
      }
      if (!t) addNodeAt(toDiagram(e.clientX, e.clientY), "box", null, true);
    });

    // a pasted list becomes topics: under the selected topic, or a new map
    document.addEventListener("paste", onPaste);
    function onPaste(e) {
      if (view || !modal.hidden) return;
      var a = document.activeElement, tag = (a && a.tagName) || "";
      if (tag === "INPUT" || tag === "TEXTAREA" || !(a === canvas || canvas.contains(a))) return;
      // an image (a screenshot, a copied picture) becomes an image box
      var files = imageFiles(e.clipboardData);
      if (files.length) {
        e.preventDefault();
        var r = canvas.getBoundingClientRect();
        importImages(files, toDiagram(r.left + canvas.clientWidth / 2, r.top + canvas.clientHeight / 2));
        return;
      }
      var text = e.clipboardData && e.clipboardData.getData("text/plain");
      if (text && pasteOutline(text)) e.preventDefault();
    }

    /* Image files in a clipboard or a drop. */
    function imageFiles(dt) {
      if (!dt) return [];
      var out = [];
      Array.prototype.forEach.call(dt.files || [], function (f) { if (/^image\//.test(f.type)) out.push(f); });
      if (!out.length) Array.prototype.forEach.call(dt.items || [], function (it) {
        if (it.kind === "file" && /^image\//.test(it.type)) { var f = it.getAsFile(); if (f) out.push(f); }
      });
      return out;
    }
    /* Images as boxes around `at` (diagram coordinates), one beside the other. */
    function importImages(files, at) {
      var x = at.x;
      files.reduce(function (p, f) {
        return p.then(function () {
          return imageData(f).then(function (img) {
            var id = addImageBox(img, { x: x, y: at.y });
            if (id) x += img.w + 32;
          });
        });
      }, Promise.resolve()).catch(function (e) { flash("Could not read the image: " + (e.message || e), true); });
    }
    /* A file as a data URI and its size; big ones are scaled down (at most
       1600px) and photos past ~400 KB stored as JPEG, so the board stays small. */
    function imageData(file) {
      var read = function (blob) {
        return new Promise(function (ok, fail) {
          var fr = new FileReader();
          fr.onload = function () { ok(fr.result); };
          fr.onerror = function () { fail(fr.error); };
          fr.readAsDataURL(blob);
        });
      };
      return read(file).then(function (uri) {
        return new Promise(function (ok, fail) {
          var img = new Image();
          img.onload = function () {
            var nw = img.naturalWidth || 320, nh = img.naturalHeight || 200;
            var sc = Math.min(1, 1600 / Math.max(nw, nh));
            var w = Math.round(nw * sc), h = Math.round(nh * sc);
            var keep = sc === 1 && (file.size < 400000 || file.type === "image/svg+xml") && /^image\/(png|jpeg|gif|webp|svg\+xml)$/.test(file.type);
            if (keep) return ok({ data: uri, w: w, h: h });
            var c = document.createElement("canvas");
            c.width = w; c.height = h;
            c.getContext("2d").drawImage(img, 0, 0, w, h);
            var png = c.toDataURL("image/png");
            ok({ data: png.length < 540000 ? png : c.toDataURL("image/jpeg", 0.85), w: w, h: h });
          };
          img.onerror = function () { fail(new Error("not an image the browser can read")); };
          img.src = uri;
        });
      });
    }
    /* An image box (shown at most 480px wide) with its asset; returns its id. */
    function addImageBox(img, at) {
      var ids = (st.doc.assets || []).map(function (a) { return a.id; }), k = 1;
      while (ids.indexOf("img" + k) >= 0) k++;
      var aid = "img" + k, id = freshId("image");
      var bw = Math.min(480, img.w), bh = Math.round(img.h * bw / img.w);
      var res = applyOps([
        { op: "add_asset", id: aid, data: img.data },
        { op: "add_node", id: id, label: "", image: aid, w: bw + 8, h: bh + 8, x: Math.round(at.x - (bw + 8) / 2), y: Math.round(at.y - (bh + 8) / 2) }
      ]);
      if (!res.ok) return null;
      select([id]);
      return id;
    }
    function pickImages(at) {
      var input = h("input", { type: "file", accept: "image/*", multiple: "multiple" });
      input.onchange = function () { if (input.files.length) importImages(Array.prototype.slice.call(input.files), at); };
      input.click();
    }
    canvas.addEventListener("dragover", function (e) {
      if (!view && e.dataTransfer && Array.prototype.indexOf.call(e.dataTransfer.types || [], "Files") >= 0) e.preventDefault();
    });
    canvas.addEventListener("drop", function (e) {
      if (view) return;
      var files = imageFiles(e.dataTransfer);
      if (!files.length) return;
      e.preventDefault();
      importImages(files, toDiagram(e.clientX, e.clientY));
    });

    canvas.addEventListener("wheel", function (e) {
      e.preventDefault();
      var rect = canvas.getBoundingClientRect();
      if (e.ctrlKey || e.metaKey) {
        zoomTo(st.zoom * Math.exp(-e.deltaY * 0.01), e.clientX - rect.left, e.clientY - rect.top);
      } else {
        st.panX -= e.deltaX;
        st.panY -= e.deltaY;
        transform();
      }
    }, { passive: false });

    /* Inline label editor over a node, edge label or group title. `wrapped`:
       the topic was just inserted above that one; left empty, the insert is undone. */
    function editLabel(id, isNew, flow, wrapped) {
      var kind = kindOf(id);
      var topic = kind === "node" && !!mapInfo(id);
      if (!kind || !st.r) return;
      var box, current;
      if (kind === "edge") {
        var ep = st.r.edges.find(function (x) { return x.id === id; });
        box = { x: ep.mid[0] - 70, y: ep.mid[1] - 14, w: 140, h: 28 };
        current = edge(id).label || "";
      } else if (kind === "group") {
        var gg = geom(id);
        box = { x: gg.x + 8, y: gg.y + 4, w: Math.max(160, gg.w * 0.6), h: 26 };
        current = group(id).label || id;
      } else {
        box = geom(id);
        current = node(id).label || id;
      }
      if (kind === "node") reveal(id);
      var tl = toScreen(box.x, box.y);
      var ta = h("textarea", { class: "dge-inline" + (topic ? " dge-inline-topic" : ""), spellcheck: "false" });
      if (topic && isNew) current = "";
      ta.value = current;
      ta.style.left = tl.x + "px";
      ta.style.top = tl.y + "px";
      ta.style.width = Math.max(80, box.w * st.zoom) + "px";
      ta.style.height = Math.max(26, box.h * st.zoom) + "px";
      ta.style.fontSize = Math.max(11, 14 * st.zoom) + "px";
      canvas.appendChild(ta);
      ta.focus();
      ta.select();
      var done = false;
      /* Returns the node's id afterwards (it may be renamed), or null if it went away. */
      function finish(save) {
        if (done) return id;
        done = true;
        var v = ta.value.trim();
        ta.remove();
        canvas.focus({ preventScroll: true });
        if (topic && isNew && wrapped && (!save || !v)) {
          undo();
          st.redo.pop();
          select([wrapped]);
          return null;
        }
        if (topic && isNew && (!save || !v)) {
          // an empty new topic is dropped; its parent stays selected
          var par = node(id) && node(id).parent;
          applyOps([{ op: "remove", id: id }]);
          select(par ? [par] : []);
          return null;
        }
        if (!save || v === current) return id;
        var ops;
        if (kind === "node") {
          ops = [{ op: "update_node", id: id, label: v }];
          // a screen's component or a list's item keeps its block's prefix
          var scr = screenOf(id) || todoOf(id);
          var nice = (scr ? scr.id + "." : "") + slug(v);
          // a box that was just added gets an id from its label
          if (isNew && v && nice !== id && !idTaken(nice)) ops.push({ op: "rename", id: id, to: nice });
          else if (isNew && v && nice !== id && !/^(node|note|topic|map)(-\d+)?$/.test(nice)) ops.push({ op: "rename", id: id, to: freshId(nice) });
        } else if (kind === "edge") ops = [{ op: "update_edge", id: id, label: v }];
        else {
          ops = [{ op: "update_group", id: id, label: v }];
          // a section that was just made gets an id from its title
          var gnice = slug(v);
          if (isNew && v && gnice && gnice !== id) ops.push({ op: "rename", id: id, to: idTaken(gnice) ? freshId(gnice) : gnice });
        }
        var res = applyOps(ops);
        if (res.ok && ops.length > 1) { select([ops[1].to]); return ops[1].to; }
        return id;
      }
      ta.addEventListener("keydown", function (e) {
        e.stopPropagation();
        if (topic && e.key === "Tab") {
          // keep typing the outline: Tab goes a level deeper
          e.preventDefault();
          var nid = finish(true);
          if (nid) addTopic(nid);
          return;
        }
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          var was = finish(true);
          // Enter on a topic just typed starts the next sibling
          if (topic && isNew && flow && was && node(was) && node(was).parent) addSibling(was, true);
          return;
        }
        if (e.key === "Escape") { e.preventDefault(); finish(isNew && topic && ta.value.trim() !== ""); }
      });
      ta.addEventListener("blur", function () { finish(true); });
    }

    function deleteSelection() {
      if (!st.sel.length) return;
      var first = node(st.sel[0]);
      var up = first && first.parent && st.sel.indexOf(first.parent) < 0 ? first.parent : null;
      // a topic inside a selected branch goes with it
      var ids = st.sel.filter(kindOf).filter(function (id) {
        var n = node(id);
        while (n && n.parent) { if (st.sel.indexOf(n.parent) >= 0) return false; n = node(n.parent); }
        return true;
      });
      // a section, swimlanes or chart goes with everything in it ("Remove section" keeps it)
      ids = ids.filter(function (id) {
        var n = node(id), g = n ? n.group : group(id) && group(id).parent;
        for (; g; g = group(g) && group(g).parent) if (ids.indexOf(g) >= 0) return false;
        return true;
      });
      confirmCanvases(ids, function () {
        applyOps(ids.map(function (id) { return group(id) ? { op: "remove", id: id, keep_members: false } : { op: "remove", id: id }; }));
        select(up && node(up) ? [up] : []);
      });
    }

    /* A titled section (a group) around the selected blocks. */
    function groupSelection() {
      var members = st.sel.filter(function (id) { return (node(id) && !node(id).parent) || group(id); });
      if (!members.length) return flash("Select the blocks to put in a section (drag a box around them), then press Section");
      var id = freshId("section");
      var res = applyOps([{ op: "add_group", id: id, label: "Section", members: members }]);
      if (res.ok) {
        select([id]);
        editLabel(id, true);
      }
    }

    /* Gantt charts: days since 1970 ↔ YYYY-MM-DD (UTC, like the core). */
    function isoDay(days) { return new Date(days * 86400000).toISOString().slice(0, 10); }
    function today() { var d = new Date(); return Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) / 86400000; }
    /* Swimlanes and gantt charts move as one block: the one a group is (or is a lane of). */
    function blockOf(gid) {
      var g = gid && group(gid);
      if (!g) return null;
      if (g.lanes || g.gantt || g.risks || g.chart || g.table || g.entity || g.adr || g.sequence) return g.id;
      var up = g.parent && group(g.parent);
      if (up && up.lanes) return up.id;
      // a section moves as a whole (and keeps its place), unless boxes in it
      // were placed by hand one by one
      var placed = st.doc.nodes.some(function (n) { return n.group === g.id && n.x != null && n.y != null; });
      return placed ? null : g.id;
    }
    function chartOf(id) { var n = node(id), g = n && n.group && group(n.group); return g && g.gantt ? g : null; }
    function timelineOf(id) { var n = node(id), g = n && n.group && group(n.group); return g && g.timeline ? g : null; }
    function matrixOf(id) { var n = node(id), g = n && n.group && group(n.group); return g && g.risks ? g : null; }
    /* Themes: the board's choice ("" = default), and the picker's options. */
    function currentTheme() { return (st.doc && st.doc.settings && st.doc.settings.palette) || ""; }
    function fillThemes() {
      var sel = toolbar.theme;
      if (!sel || !st.doc) return;
      var t = JSON.parse(st.session.themes()), cur = currentTheme();
      var opts2 = [["", "Default theme"]].concat(t.presets.map(function (n) { return [n, n.charAt(0).toUpperCase() + n.slice(1)]; }));
      t.board.forEach(function (n) { if (t.presets.indexOf(n) < 0) opts2.push([n, n + " (this board)"]); });
      if (cur && !opts2.some(function (o) { return o[0] === cur; })) opts2.push([cur, cur]);
      opts2.push(["__edit", "Edit theme…"]);
      var key = JSON.stringify(opts2) + cur;
      if (sel._key === key) return;
      sel._key = key;
      sel.innerHTML = "";
      opts2.forEach(function (o) { var op = h("option", { value: o[0] }, [o[1]]); if (o[0] === cur) op.selected = true; sel.appendChild(op); });
    }
    /* Edit theme…: the board's theme as text, with its light and dark mode (a
       preset or the default is copied into a new theme). */
    function editTheme() {
      var cur = currentTheme(), own = (st.doc.themes || []).some(function (t) { return t.id === cur; });
      var text = cur ? st.session.theme_text(cur) : "";
      if (!own) {
        var id = freshThemeId(cur ? cur + "-custom" : "custom");
        text = text ? text.replace(/^theme\s+\S+/, "theme " + id)
          : "theme " + id + " {\n  blue: #0d99ff\n  light {\n    background: #f4f5f7\n    text: #1c2026\n    title: #1c2026\n  }\n  dark {\n    background: #0f1114\n    text: #e3e7ec\n    title: #e3e7ec\n  }\n}\n";
      }
      var ta = h("textarea", { class: "dge-chart-data", spellcheck: "false", rows: "18" });
      ta.value = text.trim();
      var save = function () {
        var m = /theme\s+([\w.-]+)/.exec(ta.value);
        if (!m) { flash("Start with `theme id {`", true); return; }
        if (applyOps([{ op: "theme", source: ta.value }, { op: "set", key: "theme", value: m[1] }]).ok) closeModal();
      };
      showModal("Theme", h("div", {}, [
        h("p", { class: "dge-muted" }, ["Lines at the top apply in both modes; `light { }` and `dark { }` hold what differs (the board follows the viewer's mode). Colours as #hex. Keys: background, dots, text, muted, surface, border, line, section, header, title, title-text, pin, font, radius, and the named colours (gray, blue, green, yellow, orange, red, purple, pink, teal)."]),
        ta
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: save }, ["Save and use"])
      ]);
      setTimeout(function () { ta.focus(); }, 0);
    }
    function freshThemeId(base) {
      var ids = (st.doc.themes || []).map(function (t) { return t.id; }), id = base, k = 2;
      while (ids.indexOf(id) >= 0) id = base + "-" + k++;
      return id;
    }
    /* The chart a row belongs to. */
    function plotOf(id) { var n = node(id), g = n && n.group && group(n.group); return g && (g.chart || g.table || g.entity) ? g : null; }
    function entityOf(id) { var n = node(id), g = n && n.group && group(n.group); return g && g.entity ? g : null; }
    /* A database table with an id; its name is typed next. */
    function addEntity() {
      var gid = freshId("table");
      var res = applyOps([{ op: "merge", dsl: "entity " + gid + ": Table {\n  id: uuid [pk]\n  created_at: timestamptz [default: now()]\n}" }]);
      if (res.ok) {
        select([gid]);
        reveal(gid);
        editLabel(gid, true);
      }
    }
    /* A column as written: `email: text [unique, fk: teams.id]`. */
    function columnText(n) {
      var c = n.column || {}, a = [];
      if (c.pk) a.push("pk");
      if (c.unique) a.push("unique");
      if (c.null) a.push("null");
      if (c.fk) a.push("fk: " + c.fk);
      if (c.default) a.push("default: " + c.default);
      if (c.rel) a.push("rel: " + c.rel);
      return (n.label || n.id.split(".").pop()) + ": " + (c.type || "") + (a.length ? " [" + a.join(", ") + "]" : "");
    }
    function parseColumn(line) {
      var m = /^\s*([^:\[]+?)\s*(?::\s*(.*?))?\s*(?:\[([^\]]+)\])?\s*$/.exec(line);
      if (!m) return null;
      var c = { type: (m[2] || "").trim() };
      (m[3] || "").split(",").map(function (x) { return x.trim(); }).filter(Boolean).forEach(function (f) {
        var kv = f.split(":"), k = kv[0].trim().toLowerCase(), v = kv.slice(1).join(":").trim();
        if (k === "pk" || k === "primary") c.pk = true;
        else if (k === "unique" || k === "uq") c.unique = true;
        else if (k === "null" || k === "nullable") c.null = true;
        else if (k === "fk" || k === "references") c.fk = v;
        else if (k === "default") c.default = v;
        else if (k === "rel" || k === "relationship") c.rel = v;
      });
      return { name: m[1].trim(), column: c };
    }
    /* Ops that make table `gid` have `cols` ({name, column}), in that order;
       a column keeps its id (and its comments) while its name stays. */
    function entityOps(gid, cols) {
      var have = st.doc.nodes.filter(function (n) { return n.group === gid; }), ops = [], keep = [];
      cols.forEach(function (c, i) {
        var id = gid + "." + c.name;
        keep.push(id);
        if (node(id)) ops.push({ op: "update_node", id: id, column: c.column, index: i });
        else ops.push({ op: "add_node", id: id, label: c.name, group: gid, column: c.column, index: i });
      });
      have.forEach(function (n) { if (keep.indexOf(n.id) < 0) ops.unshift({ op: "remove", id: n.id }); });
      return ops;
    }
    function editEntity(gid) {
      var ta = h("textarea", { class: "dge-chart-data", spellcheck: "false", rows: "12" });
      ta.value = st.doc.nodes.filter(function (n) { return n.group === gid; }).map(columnText).join("\n");
      var save = function () {
        var cols = [], bad = null;
        ta.value.split("\n").filter(function (l) { return l.trim(); }).forEach(function (l) { var c = parseColumn(l); if (c) cols.push(c); else bad = bad || l; });
        if (bad) return flash("Could not read “" + bad + "”: write `name: type [pk, unique, null, fk: table.column]`", true);
        if (applyOps(entityOps(gid, cols)).ok) closeModal();
      };
      showModal("Columns: " + (group(gid).label || gid), h("div", {}, [
        h("p", { class: "dge-muted" }, ["One column per line: `name: type [pk, unique, null, fk: table.column, rel: member of, default: now()]`. The type can be left out for a conceptual model."]),
        ta
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: save }, ["Save"])
      ]);
      setTimeout(function () { ta.focus(); }, 0);
    }
    function recordOf(id) { var n = node(id), g = n && n.group && group(n.group); return g && g.adr ? g : null; }
    var STATUSES = ["proposed", "accepted", "rejected", "superseded", "deprecated"];
    function statusMenu(gid) {
      var cur = group(gid).adr;
      setTimeout(function () {
        openMenu(st.menuAt, STATUSES.map(function (k) {
          return [(k === cur ? "✓ " : "\u2003") + k.charAt(0).toUpperCase() + k.slice(1), function () { applyOps([{ op: "update_group", id: gid, adr: k }]); }];
        }));
      }, 0);
    }
    /* A decision record with the usual fields to fill in; its title is typed next. */
    function addRecord() {
      var gid = freshId("decision");
      var res = applyOps([{ op: "merge", dsl: "adr " + gid + ": Decision [proposed] {\n  context: What is going on, and why decide now?\n  options: What we considered\n  decision: What we chose\n  consequences: What gets easier or harder\n}" }]);
      if (res.ok) {
        select([gid]);
        reveal(gid);
        editLabel(gid, true);
      }
    }
    /* The record's fields as text, one per line (`name: text`); saving updates,
       adds, reorders and removes fields by name. */
    function editRecord(gid) {
      var fields = function () { return st.doc.nodes.filter(function (n) { return n.group === gid; }); };
      var ta = h("textarea", { class: "dge-chart-data", spellcheck: "false", rows: "12" });
      ta.value = fields().map(function (n) { return (n.label || n.id) + ": " + (n.detail || "").replace(/\n/g, "\\n"); }).join("\n");
      var save = function () {
        var have = fields(), ops = [], keep = [], bad = null;
        ta.value.split("\n").filter(function (l) { return l.trim(); }).forEach(function (l, i) {
          var k = l.indexOf(":");
          if (k <= 0) { bad = bad || l; return; }
          var name = l.slice(0, k).trim(), text = l.slice(k + 1).trim().replace(/\\n/g, "\n");
          var f = have.find(function (n) { return (n.label || n.id) === name && keep.indexOf(n.id) < 0; });
          if (f) { keep.push(f.id); ops.push({ op: "update_node", id: f.id, detail: text, index: i }); }
          else {
            var id = gid + "." + slug(name), j = 2;
            while (node(id) || keep.indexOf(id) >= 0) id = gid + "." + slug(name) + "-" + j++;
            keep.push(id);
            ops.push({ op: "add_node", id: id, label: name, group: gid, detail: text || undefined, index: i });
          }
        });
        if (bad) return flash("Could not read “" + bad + "”: write `name: text`", true);
        var drop = have.filter(function (n) { return keep.indexOf(n.id) < 0; }).map(function (n) { return { op: "remove", id: n.id }; });
        if (applyOps(drop.concat(ops)).ok) closeModal();
      };
      showModal("Decision: " + (group(gid).label || gid), h("div", {}, [
        h("p", { class: "dge-muted" }, ["One field per line: `context: what is going on`. `\\n` starts a new line within a field."]),
        ta
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: save }, ["Save"])
      ]);
      setTimeout(function () { ta.focus(); }, 0);
    }
    function tableOf(id) { var n = node(id), g = n && n.group && group(n.group); return g && g.table ? g : null; }
    /* A table with a header and two rows; its title is typed next. */
    function addTable() {
      var gid = freshId("table");
      var res = applyOps([{ op: "merge", dsl: "table " + gid + ": Table {\n  item | owner | notes\n  First | | \n  Second | | \n}" }]);
      if (res.ok) {
        select([gid]);
        reveal(gid);
        editLabel(gid, true);
      }
    }
    /* A row whose id came from its label (`table.first`) follows a new label: a
       rename op (comments, links and arrows go along). Returns the id to use. */
    function relabel(row, gid, label, ops, taken) {
      var auto = gid + "." + slug(row.label || row.id), want = gid + "." + slug(label);
      if (row.id !== auto || want === row.id || idTaken(want) || (taken && taken.indexOf(want) >= 0)) return row.id;
      ops.push({ op: "rename", id: row.id, to: want });
      if (taken) taken.push(want);
      return want;
    }
    function splitCells(line) {
      return line.split(/(?<!\\)\|/).map(function (c) { return c.replace(/\\\|/g, "|").trim(); });
    }
    function tableText(gid) {
      var g = group(gid), esc = function (c) { return String(c).replace(/\|/g, "\\|"); };
      var lines = [(g.columns || []).map(esc).join(" | ")];
      st.doc.nodes.filter(function (n) { return n.group === gid; }).forEach(function (n) {
        lines.push([n.label || n.id].concat(n.cells || []).map(esc).join(" | "));
      });
      return lines.join("\n");
    }
    /* Ops that make table `gid` hold `rows` (the first is the header); rows keep their ids by position. */
    function tableOps(gid, rows) {
      var ops = [{ op: "update_group", id: gid, columns: rows.length ? rows[0] : [] }];
      var have = st.doc.nodes.filter(function (n) { return n.group === gid; }), taken = [];
      rows.slice(1).forEach(function (c, i) {
        var label = c[0] || "—", cells = c.slice(1);
        while (cells.length && cells[cells.length - 1] === "") cells.pop();
        if (have[i]) ops.push({ op: "update_node", id: relabel(have[i], gid, label, ops, taken), label: label, cells: cells });
        else {
          var base = gid + "." + slug(label), id = base, k = 2;
          while (idTaken(id) || taken.indexOf(id) >= 0) id = base + "-" + k++;
          taken.push(id);
          ops.push({ op: "add_node", id: id, label: label, group: gid, cells: cells });
        }
      });
      have.slice(Math.max(rows.length - 1, 0)).forEach(function (n) { ops.push({ op: "remove", id: n.id }); });
      return ops;
    }
    /* Who you are: asked once when git does not know (it signs comments
       and the versions kept of the board), remembered in this browser. */
    function askName(then) {
      if (!modal.hidden) return;
      var input = h("input", { class: "dge-name-input", placeholder: "Your name", autocomplete: "name" });
      var go = function () {
        var v = input.value.trim();
        if (!v) return input.focus();
        st.author = v;
        lsSet("diagram.author", v);
        closeModal();
        flash("Hello, " + v + ": your comments and versions are signed with your name");
        if (then) then();
      };
      input.addEventListener("keydown", function (e) { e.stopPropagation(); if (e.key === "Enter") go(); });
      showModal("Who are you?", h("div", { class: "dge-persp-form" }, [
        h("p", { class: "dge-muted" }, ["Git does not know your name here, so the board does not either. It signs your comments and the versions kept of the board, so others know who changed what."]),
        input
      ]), [h("button", { class: "primary", onclick: go }, ["Continue"])]);
      setTimeout(function () { input.focus(); }, 0);
    }
    /* History: the board's versions (commits in git, snapshots otherwise).
       A version can be compared with now or brought back (undoable). */
    var historyMenu = null;
    function toggleHistory(anchor) {
      if (historyMenu && !historyMenu.hidden) { historyMenu.hidden = true; return; }
      if (!historyMenu) { historyMenu = h("div", { class: "dge-board-menu dge-history", role: "dialog", "aria-label": "History", hidden: true }); el.appendChild(historyMenu); }
      historyMenu.innerHTML = "";
      historyMenu.appendChild(h("div", { class: "dge-muted" }, ["Loading the history…"]));
      var r = anchor.getBoundingClientRect(), host = el.getBoundingClientRect();
      historyMenu.hidden = false;
      historyMenu.style.top = (r.bottom - host.top + 6) + "px";
      fitPopup(historyMenu);
      historyMenu.style.left = Math.max(8, Math.min(r.right - host.left - 340, host.width - 348)) + "px";
      opts.history.list().then(function (res) {
        historyMenu.innerHTML = "";
        var head = res.git ? "Commits that changed this board" : "Versions kept of this board (it is not in git)";
        historyMenu.appendChild(h("b", {}, [head]));
        if (!res.versions || !res.versions.length) {
          historyMenu.appendChild(h("p", { class: "dge-muted" }, [res.git ? "No commits yet." : "None yet: a version is kept every few minutes while you edit."]));
          return;
        }
        var list = h("div", { class: "dge-history-list" });
        res.versions.forEach(function (v) {
          var when = ago(v.time);
          list.appendChild(h("div", { class: "dge-history-row" }, [
            h("span", { class: "dge-num", "aria-hidden": "true" }, [(v.author || "?").split(/\s+/).map(function (w) { return w.charAt(0); }).join("").slice(0, 2).toUpperCase()]),
            // the exact time (and the whole message) on hover
            h("span", { class: "dge-history-text", title: new Date(v.time * 1000).toLocaleString(undefined, { dateStyle: "full", timeStyle: "short" }) + (v.message ? "\n" + v.message : "") }, [h("b", {}, [v.author || "someone"]), h("span", {}, [when + (v.message ? " · " + v.message : "")])]),
            h("button", { title: "Highlight what changed since this version", onclick: function () { opts.history.get(v.id).then(function (src) { setBase(src, v.message ? "“" + v.message + "”" : when); historyMenu.hidden = true; }, function (e) { flash(e.message, true); }); } }, ["Changes"]),
            h("button", { title: "Bring the board back to this version (" + MOD + "Z undoes it)", onclick: function () { opts.history.get(v.id).then(function (src) { restoreVersion(src, v.author, when); }, function (e) { flash(e.message, true); }); } }, ["Restore"])
          ]));
        });
        historyMenu.appendChild(list);
      }, function (e) { historyMenu.innerHTML = ""; historyMenu.appendChild(h("p", {}, ["No history: " + e.message])); });
    }
    function ago(t) {
      var s = Math.max(0, Date.now() / 1000 - t);
      if (s < 90) return "just now";
      if (s < 3600) return Math.round(s / 60) + " min ago";
      if (s < 86400) return Math.round(s / 3600) + " h ago";
      if (s < 86400 * 7) return Math.round(s / 86400) + " days ago";
      return new Date(t * 1000).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
    }
    function restoreVersion(src, who, when) {
      if (opts.requireName && !st.author) return askName();
      historyMenu.hidden = true;
      st.undo.push(st.text);
      st.redo = [];
      st.session.set_source(src);
      st.text = st.session.source("dsl");
      refreshDoc();
      afterChange(true);
      flash("Back to " + (who ? who + "'s" : "the") + " version from " + when + " (" + MOD + "Z undoes it)");
    }
    /* Recipes: ready-made arrangements of blocks (the AI knows them too). */
    function recipeMenu() {
      var list = JSON.parse((st.answer ? st.answer.main.session : st.session).recipes());
      openMenu(st.menuAt, list.map(function (r) {
        return [r.name, function () { insertRecipe(r); }, "For " + r.when];
      }));
    }
    function insertRecipe(r) {
      var probe = new wasm.Diagram();
      probe.set_source(r.source);
      var ids = JSON.parse(probe.document()).groups.map(function (g) { return g.id; });
      probe.free();
      var have = ids.filter(function (id) { return kindOf(id); });
      if (have.length) { select([have[0]]); reveal(have[0]); return flash("This board already has one: add to it (the AI does too)"); }
      if (applyOps([{ op: "merge", dsl: r.source }]).ok) { if (ids[0]) { select([ids[0]]); reveal(ids[0]); } flash("Inserted “" + r.name + "”: its rows are examples, write over them"); }
    }
    /* Database schemas: a section of tables that come from a schema file
       (structure.sql, a MySQL or SQLite dump, Rails' schema.rb), or from a
       pasted one. The tables are ordinary tables on the board; updating
       reads the file again and changes only what changed. */
    function schemaOf(id) { var g = group(id); return g && g.schema ? g : null; }
    function addSchema() {
      var files = h("select", { class: "dge-select" }, [h("option", { value: "" }, ["Looking for schema files…"])]);
      var only = h("input", { type: "text", placeholder: "All tables, or e.g. users orders* billing.*", spellcheck: "false" });
      var ta = h("textarea", { class: "dge-chart-data", spellcheck: "false", rows: "8", placeholder: "CREATE TABLE users (\n  id uuid PRIMARY KEY,\n  email text NOT NULL UNIQUE\n);" });
      if (opts.sources) Promise.resolve(opts.sources.list()).then(function (list) {
        files.innerHTML = "";
        files.appendChild(h("option", { value: "" }, [list.length ? "Choose a file…" : "No .sql or schema.rb files in this project"]));
        list.forEach(function (f, i) { files.appendChild(h("option", { value: f, selected: i === 0 }, [f])); });
      }, function () { files.innerHTML = ""; files.appendChild(h("option", { value: "" }, ["No schema files found"])); });
      var add = function () {
        var file = ta.value.trim() ? "" : files.value;
        if (!file && !ta.value.trim()) return flash(opts.sources ? "Choose a file, or paste a schema" : "Paste a schema", true);
        var make = function (src) {
          var r = JSON.parse(st.session.schema_add_ops("Database", file, only.value.trim(), src));
          if (!r.ok) return flash(r.error, true);
          if (!applyOps(r.ops).ok) return;
          closeModal();
          var gid = r.ops[0].id;
          select([gid]); reveal(gid);
          flash((r.added.length === 1 ? "1 table" : r.added.length + " tables") + (file ? " from " + file : "") + (r.warnings.length ? " · " + r.warnings.length + " lines not understood" : ""), false, 3000);
        };
        if (file) opts.sources.read(file).then(make, function (e) { flash(e.message, true); });
        else make(ta.value);
      };
      showModal("Database schema", h("div", {}, [
        h("p", { class: "dge-muted" }, ["Its tables, keys and relationships go on the board as tables you can point arrows and comments at. Postgres structure.sql, MySQL and SQLite dumps and Rails' schema.rb work."]),
        opts.sources ? h("label", { class: "dge-schema-field" }, [h("span", {}, ["From a file in this project (it stays the source: Update reads it again)"]), files]) : null,
        h("label", { class: "dge-schema-field" }, [h("span", {}, [opts.sources ? "Or paste one" : "Paste the schema"]), ta]),
        h("label", { class: "dge-schema-field" }, [h("span", {}, ["Only these tables (optional)"]), only])
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: add }, ["Add tables"])
      ]);
    }
    /* Bring schema `gid` up to date with `src`; `quiet` says nothing when it is. */
    function syncSchema(gid, src, quiet) {
      var r = JSON.parse(st.session.schema_sync_ops(gid, src));
      if (!r.ok) return flash(r.error, true);
      if (!r.ops.length) { if (!quiet) flash("The tables are up to date"); return; }
      if (applyOps(r.ops).ok) flash("Updated: " + r.summary, false, 3000);
    }
    function updateFromFile(gid) {
      var g = schemaOf(gid);
      if (!g || !g.file || !opts.sources) return;
      opts.sources.read(g.file).then(function (src) { syncSchema(gid, src); }, function (e) { flash(e.message, true); });
    }
    function pasteSchema(gid) {
      var ta = h("textarea", { class: "dge-chart-data", spellcheck: "false", rows: "12", placeholder: "CREATE TABLE …" });
      var go = function () { if (ta.value.trim()) { closeModal(); syncSchema(gid, ta.value); } };
      ta.addEventListener("keydown", function (e) { if ((e.metaKey || e.ctrlKey) && e.key === "Enter") go(); });
      showModal("A new version of “" + (group(gid).label || gid) + "”", h("div", {}, [
        h("p", { class: "dge-muted" }, ["Paste the schema (SQL or a Rails schema.rb): tables and columns that changed are updated, the rest stays as it is."]), ta
      ]), [h("button", { onclick: closeModal }, ["Cancel"]), h("button", { class: "primary", onclick: go }, ["Update"])]);
      setTimeout(function () { ta.focus(); }, 0);
    }
    /* On opening: a schema whose file changed since says so, with a button to update. */
    function checkSchemas() {
      if (view || !opts.sources || !st.doc) return;
      (st.doc.groups || []).filter(function (g) { return g.schema && g.file; }).forEach(function (g) {
        opts.sources.read(g.file).then(function (src) {
          var r = JSON.parse(st.session.schema_sync_ops(g.id, src));
          if (r.ok && r.ops.length) flashAction(g.file + " changed (" + r.summary + ")", "Update", function () { syncSchema(g.id, src); });
        }, function () {});
      });
    }

    /* To-do lists: items with checkboxes, owners and due dates. */
    function todoOf(id) { var n = node(id), g = n && n.group && group(n.group); return g && g.todo ? g : null; }
    function doneOf(n) { return !!(n && n.done && n.done >= 100); }
    function toggleTodo(id) { applyOps([{ op: "update_node", id: id, done: doneOf(node(id)) ? 0 : 100 }]); select([id]); }
    function addTodo() {
      var gid = freshId("todo");
      var res = applyOps([{ op: "merge", dsl: "todo " + gid + ": To-do {\n  [ ] First thing\n  [ ] Second thing\n}" }]);
      if (res.ok) { select([gid]); reveal(gid); editLabel(gid, true); }
    }
    /* A new item after `after` (or at the end), at its level; its text is typed next. */
    function addTodoItem(gid, after) {
      var list = st.doc.nodes.filter(function (n) { return n.group === gid; });
      var at = after ? list.indexOf(node(after)) + 1 : list.length;
      var id = freshId(gid + ".item");
      if (!applyOps([{ op: "add_node", id: id, label: "New item", group: gid, index: at, level: after ? node(after).level || 0 : 0 }]).ok) return;
      select([id]);
      editLabel(id, true);
    }
    function editTodo(gid) {
      var ta = h("textarea", { class: "dge-chart-data", spellcheck: "false", rows: "12" });
      ta.value = st.session.todo_text(gid);
      var save = function () {
        var r = JSON.parse(st.session.todo_edit_ops(gid, ta.value));
        if (!r.ok) return flash(r.error, true);
        if (applyOps(r.ops).ok) closeModal();
      };
      ta.addEventListener("keydown", function (e) { if ((e.metaKey || e.ctrlKey) && e.key === "Enter") save(); });
      showModal("To-do: " + (group(gid).label || gid), h("div", {}, [
        h("p", { class: "dge-muted" }, ["One item per line: `[x] Done thing`, `[ ] Thing to do`, two spaces more for a sub-item, `[owner: ana, due: 28 Oct]` after it."]),
        ta
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: save }, ["Save (" + MOD + "↵)"])
      ]);
      setTimeout(function () { ta.focus(); }, 0);
    }
    /* Wireframe screens: a phone or a browser window, one component per
       line; arrows from components to screens make the flow. */
    var SCREEN_STARTERS = {
      mobile: "  header: App\n  title: Screen title\n  text: What this screen is for.\n  input: Email\n  button primary: Continue [blue]",
      web: "  navbar: App | Product | Pricing | Sign in\n  title: Page title\n  text: What this page is for.\n  buttons: Learn more | Get started [blue]\n  footer: © App",
      tablet: "  header: App\n  title: Screen title\n  text: What this screen is for.\n  list: First | Second | Third\n  button primary: Continue [blue]"
    };
    /* A flow to start from: sign in, the home screen, a detail, linked. */
    function addFlow() {
      var ids = freshIds(["sign-in", "home", "detail"]);
      var src = "screen " + ids[0] + ": Sign in [mobile] {\n  header: App\n  title: Welcome back\n  input email: Email\n  password: Password\n  button go: Sign in [blue]\n  link forgot: Forgot password?\n}\n" +
        "screen " + ids[1] + ": Home [mobile] {\n  header: Home\n  search: Search\n  list items: First item | Second item | Third item\n  tabbar: Home | Search | Me\n}\n" +
        "screen " + ids[2] + ": Detail [mobile] {\n  header: Item\n  image: Photo\n  title: Item name\n  text: What it is, in a line or two.\n  button buy: Add to cart [blue]\n}\n" +
        ids[0] + ".go -> " + ids[1] + ": signs in\n" + ids[1] + ".items -> " + ids[2] + ": opens";
      if (applyOps([{ op: "merge", dsl: src }]).ok) { select([ids[0]]); reveal(ids[0]); flash("A flow of three screens: click a screen to add components, drag from a button to another screen to link them"); }
    }
    /* Components: what each kind starts with, and the groups the bar offers. */
    var WIDGET_DEFAULTS = {
      header: "Title", navbar: "App | Home | Pricing | Sign in", title: "Title", subtitle: "Subtitle", text: "Some text that explains this part.",
      button: "Button", buttons: "Cancel | Save", link: "Link", input: "Label", password: "Password", search: "Search", textarea: "Message",
      select: "Choose one", date: "Date", checkbox: "Option", radio: "One | Two | Three", toggle: "Setting", slider: "Amount", image: "Image",
      video: "Video", map: "Map", chart: "Chart", avatar: "Name | Role", card: "Card title", list: "First | Second | Third",
      table: "Name | Email | Role", tabs: "One | Two | Three", tabbar: "Home | Search | Me", sidebar: "Home | Inbox | Settings",
      breadcrumb: "Home | Section | Page", badges: "New | Popular", alert: "Something to know", progress: "Progress", stepper: "One | Two | Three",
      pagination: "", rating: "", divider: "", spacer: "", footer: "© Company", fab: ""
    };
    var WIDGET_GROUPS = [
      ["Basics", ["title", "subtitle", "text", "button", "buttons", "link", "image", "divider", "spacer"]],
      ["Inputs", ["input", "password", "search", "textarea", "select", "date", "checkbox", "radio", "toggle", "slider"]],
      ["Navigation", ["header", "navbar", "tabs", "tabbar", "sidebar", "breadcrumb", "stepper", "pagination"]],
      ["Content", ["list", "table", "card", "avatar", "badges", "video", "map", "chart", "rating"]],
      ["Feedback", ["alert", "progress", "fab", "footer"]]
    ];
    function screenOf(id) {
      var g = group(id);
      if (g && g.screen) return g;
      var n = node(id), p = n && n.group && group(n.group);
      return p && p.screen ? p : null;
    }
    function componentsOf(gid) { return st.doc.nodes.filter(function (n) { return n.group === gid; }); }
    /* Add a component: after the selected one, else at the end; its text is typed next. */
    function addWidget(gid, kind) {
      var list = componentsOf(gid), cur = node(st.sel[0]);
      var at = cur && cur.group === gid ? list.indexOf(cur) + 1 : list.length;
      var base = gid + "." + kind, id = base, k = 2;
      while (idTaken(id)) id = base + "-" + k++;
      var op = { op: "add_node", id: id, label: WIDGET_DEFAULTS[kind] || "", group: gid, widget: kind, index: at };
      if (kind === "card") op.detail = "A line or two about it.";
      if (kind === "button" && !list.some(function (n) { return n.widget === "button" && n.color; })) op.color = "blue";
      if (!applyOps([op]).ok) return;
      closeWidgets();
      select([id]);
      if (WIDGET_DEFAULTS[kind]) editLabel(id, true);
    }
    function moveComponent(id, d) {
      var n = node(id), list = n && componentsOf(n.group), i = list ? list.indexOf(n) : -1, j = i + d;
      if (i < 0 || j < 0 || j >= list.length) return;
      applyOps([{ op: "update_node", id: id, index: j }]);
      select([id]);
    }
    /* The bar above the tools while a screen (or a component) is selected. */
    var screenBar = h("div", { class: "dge-screenbar", hidden: true });
    if (!view) canvas.appendChild(screenBar);
    ["pointerdown", "dblclick", "contextmenu"].forEach(function (ev) { screenBar.addEventListener(ev, function (e) { e.stopPropagation(); }); });
    var widgetsMenu = null;
    function closeWidgets() { if (widgetsMenu) { widgetsMenu.remove(); widgetsMenu = null; } }
    function openWidgets(gid) {
      if (widgetsMenu) return closeWidgets();
      widgetsMenu = h("div", { class: "dge-blocks dge-widgets", role: "menu", "aria-label": "Add a component" });
      WIDGET_GROUPS.forEach(function (grp) {
        widgetsMenu.appendChild(h("div", { class: "dge-blocks-head" }, [grp[0]]));
        widgetsMenu.appendChild(h("div", { class: "dge-widgets-grid" }, grp[1].map(function (k) {
          return h("button", { role: "menuitem", "data-widget": k, onclick: function () { addWidget(gid, k); } }, [k.charAt(0).toUpperCase() + k.slice(1)]);
        })));
      });
      ["pointerdown", "dblclick", "contextmenu"].forEach(function (ev) { widgetsMenu.addEventListener(ev, function (e) { e.stopPropagation(); }); });
      canvas.appendChild(widgetsMenu);
    }
    function updateScreenBar() {
      if (!screenBar) return; // before the editor is built
      var g = !view && st.sel.length === 1 && screenOf(st.sel[0]);
      screenBar.hidden = !g;
      if (!g) { closeWidgets(); return; }
      if (screenBar._for === st.sel[0]) return;
      screenBar._for = st.sel[0];
      screenBar.innerHTML = "";
      var comp = node(st.sel[0]) ? st.sel[0] : null;
      screenBar.appendChild(h("span", { class: "dge-screenbar-label" }, [comp ? "Add after" : "Add"]));
      ["title", "text", "button", "input", "list", "image", "tabs", "card"].forEach(function (k) {
        screenBar.appendChild(h("button", { "data-widget": k, title: "Add a " + k, onclick: function () { addWidget(g.id, k); } }, [k.charAt(0).toUpperCase() + k.slice(1)]));
      });
      screenBar.appendChild(h("button", { class: "dge-screenbar-more", "data-act": "widgets", title: "Every component", onclick: function () { openWidgets(g.id); } }, ["All ▾"]));
      screenBar.appendChild(h("span", { class: "dge-tools-sep" }));
      if (comp) {
        screenBar.appendChild(h("button", { title: "Up (⌥↑)", onclick: function () { moveComponent(comp, -1); } }, ["↑"]));
        screenBar.appendChild(h("button", { title: "Down (⌥↓)", onclick: function () { moveComponent(comp, 1); } }, ["↓"]));
      }
      screenBar.appendChild(h("button", { title: "The whole screen as text, one component per line", onclick: function () { editScreen(g.id); } }, ["Edit as text"]));
    }
    function addScreen(device) {
      var gid = freshId(device === "web" ? "page" : "screen");
      var res = applyOps([{ op: "merge", dsl: "screen " + gid + ": " + (device === "web" ? "Page" : "Screen") + " [" + device + "] {\n" + SCREEN_STARTERS[device] + "\n}" }]);
      if (res.ok) {
        select([gid]);
        reveal(gid);
        editLabel(gid, true);
      }
    }
    var WIDGETS = "header navbar title subtitle text button buttons link input password search textarea select date checkbox radio toggle slider image video map chart avatar card list table tabs tabbar sidebar breadcrumb badges alert progress stepper pagination rating divider spacer footer fab";
    function screenTextSave(gid, text) {
      var r = JSON.parse(st.session.screen_edit_ops(gid, text));
      if (!r.ok) { flash(r.error + " (components: " + WIDGETS.split(" ").join(", ") + ")", true); return false; }
      return applyOps(r.ops).ok;
    }
    function editScreen(gid) {
      var ta = h("textarea", { class: "dge-chart-data", spellcheck: "false", rows: "16" });
      ta.value = st.session.screen_text(gid);
      var save = function () { if (screenTextSave(gid, ta.value)) closeModal(); };
      ta.addEventListener("keydown", function (e) { if ((e.metaKey || e.ctrlKey) && e.key === "Enter") save(); });
      showModal("Screen: " + (group(gid).label || gid), h("div", {}, [
        h("p", { class: "dge-muted" }, ["One component per line, top to bottom: `kind [id]: text [colour]`, e.g. `button submit: Sign in [blue]`. Items go in the text separated by | (`tabs: Home | Search | Me`). Kinds: " + WIDGETS.split(" ").join(", ") + "."]),
        ta
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: save }, ["Save (" + MOD + "↵)"])
      ]);
      setTimeout(function () { ta.focus(); }, 0);
    }
    /* Cycles: steps round a circle, each leading to the next. */
    function addCycle() {
      var gid = freshId("cycle");
      var res = applyOps([{ op: "merge", dsl: "cycle " + gid + ": Cycle {\n  Plan\n  Do\n  Check\n  Act\n}" }]);
      if (res.ok) {
        select([gid]);
        reveal(gid);
        editLabel(gid, true);
      }
    }
    function editCycle(gid) {
      var ta = h("textarea", { class: "dge-chart-data", spellcheck: "false", rows: "8" });
      var have = st.doc.nodes.filter(function (n) { return n.group === gid; });
      ta.value = have.map(function (n) { return n.label || n.id; }).join("\n");
      var save = function () {
        var lines = ta.value.split("\n").map(function (l) { return l.trim(); }).filter(Boolean);
        var ops = [], taken = [], used = [];
        lines.forEach(function (label, i) {
          var k = have.findIndex(function (n, j) { return used.indexOf(j) < 0 && (n.label || n.id) === label; });
          if (k < 0) k = have.findIndex(function (n, j) { return used.indexOf(j) < 0 && lines.indexOf(n.label || n.id) < 0; });
          if (k >= 0) {
            used.push(k);
            ops.push({ op: "update_node", id: relabel(have[k], gid, label, ops, taken), label: label, index: i });
          } else {
            var base = gid + "." + slug(label), id = base, n = 2;
            while (idTaken(id) || taken.indexOf(id) >= 0) id = base + "-" + n++;
            taken.push(id);
            ops.push({ op: "add_node", id: id, label: label, group: gid, index: i });
          }
        });
        have.forEach(function (n, j) { if (used.indexOf(j) < 0) ops.push({ op: "remove", id: n.id }); });
        if (applyOps(ops).ok) closeModal();
      };
      showModal("Cycle: " + (group(gid).label || gid), h("div", {}, [h("p", { class: "dge-muted" }, ["One step per line, in order; the last leads back to the first."]), ta]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: save }, ["Save"])
      ]);
      setTimeout(function () { ta.focus(); }, 0);
    }
    /* Timelines: a line per event, `DATE: what happened`, `[important]` to star it. */
    function addTimeline() {
      var gid = freshId("timeline"), t = today();
      var res = applyOps([{ op: "merge", dsl: "timeline " + gid + ": Timeline {\n  " + isoDay(t) + ": Today [important]\n  " + isoDay(t + 30) + ": Next step\n}" }]);
      if (res.ok) {
        select([gid]);
        reveal(gid);
        editLabel(gid, true);
      }
    }
    function timelineText(gid) {
      return st.doc.nodes.filter(function (n) { return n.group === gid; }).map(function (n) {
        return (n.date || "?") + ": " + (n.label || "") + (n.important ? " [important]" : "");
      }).join("\n");
    }
    /* `DATE: what happened [important]` → {date, label, important}. */
    function eventLine(line) {
      var important = /\s*\[important\]\s*$/i.test(line);
      line = line.replace(/\s*\[important\]\s*$/i, "");
      var k = line.indexOf(": ");
      if (k < 0 && /:\s*$/.test(line)) k = line.lastIndexOf(":");
      if (k < 0) return null;
      return { date: line.slice(0, k).trim(), label: line.slice(k + 1).trim(), important: important };
    }
    function timelineOps(gid, lines) {
      var ops = [], have = st.doc.nodes.filter(function (n) { return n.group === gid; }), taken = [];
      // a line keeps the event with its label, else the next one left (its comments and links go along)
      var used = [], match = lines.map(function (e) {
        var k = have.findIndex(function (n, j) { return used.indexOf(j) < 0 && (n.label || "") === e.label; });
        if (k >= 0) used.push(k);
        return k;
      });
      match = match.map(function (k) {
        if (k >= 0) return k;
        k = have.findIndex(function (n, j) { return used.indexOf(j) < 0; });
        if (k >= 0) used.push(k);
        return k;
      });
      lines.forEach(function (e, i) {
        var old = match[i] >= 0 ? have[match[i]] : null;
        if (old) {
          ops.push({ op: "update_node", id: relabel(old, gid, e.label || e.date, ops, taken), label: e.label, date: e.date, important: e.important, index: i });
        } else {
          var base = gid + "." + slug(e.label || e.date), id = base, k = 2;
          while (idTaken(id) || taken.indexOf(id) >= 0) id = base + "-" + k++;
          taken.push(id);
          ops.push({ op: "add_node", id: id, label: e.label, group: gid, date: e.date, important: e.important });
        }
      });
      have.forEach(function (n, j) { if (used.indexOf(j) < 0) ops.push({ op: "remove", id: n.id }); });
      return ops;
    }
    function editTimeline(gid) {
      var ta = h("textarea", { class: "dge-chart-data", spellcheck: "false", rows: "12" });
      ta.value = timelineText(gid);
      var save = function () {
        var lines = ta.value.split("\n").filter(function (l) { return l.trim(); });
        var events = lines.map(eventLine), bad = lines.filter(function (l, i) { return !events[i]; });
        if (bad.length) return flash("Write each event as DATE: what happened (not “" + bad[0] + "”)", true);
        if (applyOps(timelineOps(gid, events)).ok) closeModal();
      };
      showModal("Timeline: " + (group(gid).label || gid), h("div", {}, [
        h("p", { class: "dge-muted" }, ["One event per line: DATE: what happened. Dates as you write them: 2026-10-28, 28 Oct 2026, Oct 2026, mid Dec 2026, Q4 2026, 2026, or a span like 7–14 May 2026. Add [important] to star one."]),
        ta
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: save }, ["Save"])
      ]);
      setTimeout(function () { ta.focus(); }, 0);
    }
    /* A table whose first column is dates becomes a timeline: each row an
       event, the rest of its cells its label. Ids, comments and links stay. */
    function tableToTimeline(gid) {
      var ops = [{ op: "update_group", id: gid, table: false, columns: [], timeline: true }], taken = [];
      st.doc.nodes.filter(function (n) { return n.group === gid; }).forEach(function (r) {
        var label = (r.cells || []).filter(Boolean).join(" · ") || "";
        ops.push({ op: "update_node", id: relabel(r, gid, label || r.label, ops, taken), date: r.label || r.id, label: label, cells: [] });
      });
      var res = applyOps(ops);
      if (res.ok) flash("Now a timeline (" + MOD + "Z turns it back into a table)");
    }
    /* Edit as text…: the table as lines of cells separated by |. */
    function editTable(gid) {
      var ta = h("textarea", { class: "dge-chart-data", spellcheck: "false", rows: "12" });
      ta.value = tableText(gid);
      var save = function () {
        var rows = ta.value.split("\n").filter(function (l) { return l.trim(); }).map(splitCells);
        if (applyOps(tableOps(gid, rows)).ok) closeModal();
      };
      showModal("Table: " + (group(gid).label || gid), h("div", {}, [
        h("p", { class: "dge-muted" }, ["One row per line, cells separated by | (write \\| for a | in a cell). The first line is the header."]),
        ta
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: save }, ["Save"])
      ]);
      setTimeout(function () { ta.focus(); }, 0);
    }
    /* Which chart to add: bars, lines, areas, pie, donut, scatter. */
    function chartMenu(anchor) {
      openMenu(anchor, CHARTS.map(function (c) {
        return [c[1], function () { canvas.focus({ preventScroll: true }); addChart(c[0]); }];
      }));
    }
    /* A chart of `kind` with a little data; its title is typed next. */
    function addChart(kind) {
      kind = kind || "bar";
      var gid = freshId("chart");
      var body = kind === "pie" || kind === "donut" ? "  Plan 50\n  Build 30\n  Launch 20"
        : kind === "scatter" ? "  idea | effort | impact\n  First | 3 | 8\n  Second | 6 | 5\n  Third | 2 | 3"
        : "  quarter | Plan | Actual\n  Q1 | 10 | 8\n  Q2 | 14 | 15\n  Q3 | 18 | 16";
      var res = applyOps([{ op: "merge", dsl: "chart " + gid + ": Chart [" + kind + "] {\n" + body + "\n}" }]);
      if (res.ok) {
        select([gid]);
        reveal(gid);
        editLabel(gid, true);
      }
    }
    var CHARTS = [["bar", "Bars"], ["line", "Lines"], ["area", "Areas"], ["pie", "Pie"], ["donut", "Donut"], ["scatter", "Scatter (x, y)"]];
    function chartTypeMenu(gid) {
      var cur = group(gid).chart;
      setTimeout(function () {
        openMenu(st.menuAt, CHARTS.map(function (c) {
          return [(c[0] === cur ? "✓ " : "\u2003") + c[1], function () { applyOps([{ op: "update_group", id: gid, chart: c[0] }]); }];
        }));
      });
    }
    /* A chart's data as the table it is written as: header, then rows. */
    function chartText(gid) {
      var g = group(gid), rows = st.doc.nodes.filter(function (n) { return n.group === gid; });
      var lines = [];
      if ((g.series || []).length) lines.push([g.axis || ""].concat(g.series).join(" | "));
      rows.forEach(function (n) {
        lines.push([n.label || n.id].concat((n.values || []).map(function (v) { return v == null ? "-" : String(v); })).join(" | "));
      });
      return lines.join("\n");
    }
    function cellValue(c) {
      var t = String(c).trim().replace(/%$/, "").replace(/[,_]/g, "");
      if (t === "" || t === "-") return null;
      var v = Number(t);
      return isFinite(v) ? v : undefined;
    }
    /* Edit data…: the table in a text box; rows keep their ids (and comments) by position. */
    function editChartData(gid) {
      var ta = h("textarea", { class: "dge-chart-data", spellcheck: "false", rows: "10" });
      ta.value = chartText(gid);
      var save = function () {
        var lines = ta.value.split("\n").map(function (l) { return l.trim(); }).filter(Boolean);
        var cells = lines.map(function (l) {
          var c = l.split("|").map(function (x) { return x.trim(); });
          if (c.length === 1) { var m = /^(.*\S)\s+(\S+)$/.exec(c[0]); if (m && cellValue(m[2]) != null) c = [m[1], m[2]]; }
          return c;
        });
        var ops = [], series = [], axis = "";
        if (cells.length && cells[0].length > 1 && cells[0].slice(1).every(function (c) { return cellValue(c) === undefined; })) {
          axis = cells[0][0]; series = cells[0].slice(1); cells.shift();
        }
        for (var i = 0; i < cells.length; i++) {
          var bad = cells[i].slice(1).filter(function (c) { return cellValue(c) === undefined; })[0];
          if (bad !== undefined) { flash("“" + bad + "” is not a number (row " + (i + 1) + ")", true); return; }
        }
        ops.push({ op: "update_group", id: gid, series: series, axis: axis });
        var rows = st.doc.nodes.filter(function (n) { return n.group === gid; });
        var taken = [];
        cells.forEach(function (c, i) {
          var values = c.slice(1).map(cellValue);
          if (rows[i]) ops.push({ op: "update_node", id: relabel(rows[i], gid, c[0], ops, taken), label: c[0], values: values });
          else {
            var base = gid + "." + slug(c[0]), id = base, k = 2;
            while (idTaken(id) || taken.indexOf(id) >= 0) id = base + "-" + k++;
            taken.push(id);
            ops.push({ op: "add_node", id: id, label: c[0], group: gid, values: values });
          }
        });
        rows.slice(cells.length).forEach(function (n) { ops.push({ op: "remove", id: n.id }); });
        if (applyOps(ops).ok) closeModal();
      };
      showModal("Data: " + (group(gid).label || gid), h("div", {}, [
        h("p", { class: "dge-muted" }, ["One row per line: a label, then a value for each series, separated by |. A first row of names is the header (series). Use - for no value."]),
        ta
      ]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "primary", onclick: save }, ["Save"])
      ]);
      setTimeout(function () { ta.focus(); }, 0);
    }
    /* A 5×5 risk matrix with one risk in the middle; its title is typed next. */
    function addRisks() {
      var ids = freshIds(["risks", "risk-1"]);
      var res = applyOps([
        { op: "add_group", id: ids[0], label: "Risks", risks: true },
        { op: "add_node", id: ids[1], label: "New risk", group: ids[0], probability: 3, impact: 3 }
      ]);
      if (res.ok) {
        select([ids[0]]);
        reveal(ids[0]);
        editLabel(ids[0], true);
      }
    }
    function addRisk(gid) {
      var nid = freshId("risk");
      if (applyOps([{ op: "add_node", id: nid, label: "New risk", group: gid, probability: 3, impact: 3 }]).ok) {
        select([nid]);
        askAt(st.menuAt, "New risk", "What could go wrong?", function (v) { applyOps([{ op: "update_node", id: nid, label: v }]); });
      }
    }
    /* Pick a level (1 … the matrix's levels) for a risk's probability or impact. */
    function levelMenu(id, field) {
      var g = matrixOf(id), n = g.levels || 5, cur = node(id)[field] || 0;
      var names = n === 5 ? ["Very low", "Low", "Medium", "High", "Very high"] : n === 4 ? ["Low", "Medium", "High", "Very high"] : ["Low", "Medium", "High"];
      setTimeout(function () {
        openMenu(st.menuAt, names.map(function (name, k) {
          var v = k + 1, o = { op: "update_node", id: id };
          o[field] = v;
          return [(v === cur ? "✓ " : "\u2003") + v + " · " + name, function () { applyOps([o]); }];
        }));
      });
    }
    function levelsMenu(gid) {
      var cur = group(gid).levels || 5;
      setTimeout(function () {
        openMenu(st.menuAt, [3, 4, 5].map(function (n) {
          return [(n === cur ? "✓ " : "\u2003") + n + " × " + n, function () { applyOps([{ op: "update_group", id: gid, levels: n === 5 ? 0 : n }]); }];
        }));
      });
    }
    /* A chart starting today with two tasks; its title is typed next. */
    function addGantt() {
      var ids = freshIds(["plan", "task-1", "task-2"]);
      var res = applyOps([
        { op: "add_group", id: ids[0], label: "Plan", gantt: true, start: isoDay(today()) },
        { op: "add_node", id: ids[1], label: "Task 1", group: ids[0], days: 5 },
        { op: "add_node", id: ids[2], label: "Task 2", group: ids[0], days: 3, after: [ids[1]] }
      ]);
      if (res.ok) {
        select([ids[0]]);
        reveal(ids[0]);
        editLabel(ids[0], true);
      }
    }
    function addTask(chart, after) {
      var nid = freshId("task"), op = { op: "add_node", id: nid, label: "New task", group: chart, days: 3 };
      if (after) {
        op.after = [after];
        // right below the task it follows
        op.index = st.doc.nodes.filter(function (n) { return n.group === chart; }).map(function (n) { return n.id; }).indexOf(after) + 1;
      }
      if (applyOps([op]).ok) {
        select([nid]);
        editLabel(nid, true);
      }
    }
    function taskDays(id, delta) {
      var d = (node(id).days || 1) + delta;
      if (d >= 1) applyOps([{ op: "update_node", id: id, days: d }]);
    }
    /* The laid-out chart a task is on: day width, scale, … */
    function chartBox(id) {
      var c = chartOf(id), g = c && st.r.groups.find(function (x) { return x.id === c.id; });
      return g && g.gantt;
    }
    /* What dragging a bar snaps to, in days: a day, or a week / month on long scales. */
    function snapDays(scale) { return scale === "quarter" ? 30 : scale === "month" ? 7 : 1; }
    /* Is the pointer on the right end of a task's bar? */
    function barEnd(id, clientX) {
      var g = chartOf(id) && !node(id).milestone && geom(id);
      if (!g) return false;
      var right = toScreen(g.x + g.w, 0).x + canvas.getBoundingClientRect().left;
      return Math.abs(clientX - right) <= 6;
    }
    /* `id` starts when `on` ends. */
    function dependOn(id, on) {
      var t = node(id), after = (t.after || []).slice();
      if (id === on || after.indexOf(on) >= 0) return;
      // no loops: `on` may not already wait for `id`
      var seen = [], todo = [on];
      while (todo.length) {
        var x = todo.pop();
        if (x === id) { flash("“" + labelOf(on) + "” already waits for “" + labelOf(id) + "”", true); return; }
        if (seen.indexOf(x) < 0) { seen.push(x); todo = todo.concat((node(x) || {}).after || []); }
      }
      after.push(on);
      applyOps([{ op: "update_node", id: id, after: after, start: "" }]);
      flash("“" + labelOf(id) + "” starts after “" + labelOf(on) + "”");
    }
    function dropDependency(id, on) {
      applyOps([{ op: "update_node", id: id, after: (node(id).after || []).filter(function (a) { return a !== on; }) }]);
    }
    /* Pick which tasks this one starts after (✓ = it does). */
    function afterMenu(id) {
      var c = chartOf(id), mine = node(id).after || [];
      var others = st.doc.nodes.filter(function (n) { return n.group === c.id && n.id !== id; });
      setTimeout(function () {
        openMenu(st.menuAt, others.length ? others.map(function (n) {
          var on = mine.indexOf(n.id) >= 0;
          return [(on ? "✓ " : "\u2003") + (n.label || n.id), function () { if (on) dropDependency(id, n.id); else dependOn(id, n.id); }];
        }) : [["No other tasks", function () {}]]);
      });
    }
    function scaleMenu(gid) {
      var cur = group(gid).scale || "week";
      setTimeout(function () {
        openMenu(st.menuAt, [["day", "Days"], ["week", "Weeks"], ["month", "Months"], ["quarter", "Quarters"]].map(function (sc) {
          return [(sc[0] === cur ? "✓ " : "\u2003") + sc[1], function () { applyOps([{ op: "update_group", id: gid, scale: sc[0] === "week" ? "" : sc[0] }]); }];
        }));
      });
    }
    /* A small text field at the menu's spot; Enter calls back with the text. */
    function askAt(at, value, placeholder, done) {
      var rect = canvas.getBoundingClientRect();
      var inp = h("input", { class: "dge-inline dge-ask", value: value, placeholder: placeholder, spellcheck: "false" });
      inp.style.left = (at.x - rect.left) + "px";
      inp.style.top = (at.y - rect.top) + "px";
      canvas.appendChild(inp);
      inp.focus(); inp.select();
      var over = false;
      function end(save) {
        if (over) return;
        over = true;
        var v = inp.value.trim();
        inp.remove();
        canvas.focus({ preventScroll: true });
        if (save && (v || value)) done(v);
      }
      inp.addEventListener("keydown", function (e) {
        e.stopPropagation();
        if (e.key === "Enter") { e.preventDefault(); end(true); }
        if (e.key === "Escape") { e.preventDefault(); end(false); }
      });
      inp.addEventListener("blur", function () { end(false); });
    }
    /* Duration…: 5d, 2w, 1m, 1q or a number of days. */
    function askDuration(id) {
      var cur = node(id).days || 1;
      askAt(st.menuAt, cur % 7 === 0 ? cur / 7 + "w" : cur + "d", "5d, 2w, 1m", function (v) {
        var m = /^(\d+(?:\.\d+)?)\s*([dwmq]?)$/i.exec(v);
        if (!m || !(+m[1] > 0)) { flash("Write a duration like 5d, 2w, 1m or 1q", true); return; }
        var days = +m[1] * ({ d: 1, w: 7, m: 30, q: 91 }[(m[2] || "d").toLowerCase()]);
        applyOps([{ op: "update_node", id: id, days: days }]);
      });
    }
    function askProgress(id) {
      askAt(st.menuAt, (node(id).done || 0) + "%", "60%", function (v) {
        var m = /^(\d+(?:\.\d+)?)\s*%?$/.exec(v);
        if (!m || +m[1] > 100) { flash("Write a percentage from 0 to 100, like 60%", true); return; }
        applyOps([{ op: "update_node", id: id, done: +m[1] }]);
      });
    }
    function askOwner(id) {
      askAt(st.menuAt, node(id).owner || "", "Name", function (v) {
        applyOps([{ op: "update_node", id: id, owner: v }]);
      });
    }

    // -------------------------------------------------------------- links
    /* Link to…: the next click on an element makes it this one's target. */
    function startLinking(id) {
      st.linking = id;
      canvas.classList.add("linking");
      flash("Click the element to link to — Esc cancels", false, 6000);
    }
    function endLinking() {
      st.linking = null;
      canvas.classList.remove("linking");
      toast.className = "dge-toast";
    }
    /* `#id` goes to that element on the board; anything else opens in a new tab. */
    /* `delivery.shape` or `../plans/q3.shape#m1`: another board, relative to this one. */
    function boardLink(link) {
      var m = /^([^#]+\.(?:shape|diagram))(?:#(.*))?$/.exec(link || "");
      return m && !/^[a-z][a-z0-9+.-]*:/i.test(link) ? { path: m[1], id: m[2] || null } : null;
    }
    function linkToBoard(id, at) {
      var cur = node(id).link || "";
      function set(v) { applyOps([{ op: "update_node", id: id, link: v }]); }
      function typed() {
        askAt(at, boardLink(cur) ? cur : "", "delivery.shape#element", function (v) {
          if (v && !boardLink(v)) return flash("A board link ends in .shape (or .diagram), optionally with #id", true);
          set(v);
        });
      }
      if (!opts.boards) return typed();
      opts.boards().then(function (list) {
        if (!list.length) return typed();
        openMenu(at, list.map(function (b) { return [b, function () { set(b); flash("Linked to " + b + " — add #id to the link to go to one element"); }]; })
          .concat(["-", ["Type a link…", typed, "board.shape#element"]]));
      }).catch(typed);
    }
    function followLink(link) {
      if (!link) return;
      var b = boardLink(link);
      if (b) { if (opts.openBoard) opts.openBoard(b.path, b.id); else window.open(link, "_blank", "noopener"); return; }
      if (link.charAt(0) !== "#") { window.open(link, "_blank", "noopener"); return; }
      if (link.charAt(1) === "/") {
        // a place on the board: #/id on the top board, #/plan/id in a canvas
        var parts = link.slice(2).split("/").filter(Boolean), target = parts.pop();
        if (st.answer) closeAnswer();
        if (JSON.stringify(st.canvasPath) !== JSON.stringify(parts) && !openCanvas(parts)) return;
        return followLink("#" + target);
      }
      var id = link.slice(1), g = geom(id);
      if (!g) { flash(kindOf(id) ? "“" + labelOf(id) + "” is folded away" : "“" + id + "” is not on the board any more", true); return; }
      focusOn(g, Math.max(st.zoom, 0.6), 1.5);
      select([id]);
    }
    /* Centre a box on the screen, zoomed to fill most of it (between `min` and `max`). */
    function focusOn(g, min, max, fill) {
      var cw = canvas.clientWidth, ch = canvas.clientHeight;
      var z = Math.min(min, (cw * (fill || 0.8)) / g.w, (ch * (fill || 0.8)) / g.h, max);
      st.zoom = Math.max(0.1, z);
      st.panX = cw / 2 - (g.x + g.w / 2 - st.r.view.x) * st.zoom;
      st.panY = ch / 2 - (g.y + g.h / 2 - st.r.view.y) * st.zoom;
      transform();
    }

    // ------------------------------------------------------------ find
    /* ⌘F: find boxes, topics, rows and sections by their text; Enter goes to
       the next match, ⇧Enter to the one before. */
    var finder = null;
    function openFind() {
      if (finder) { finder.input.focus(); finder.input.select(); return; }
      var input = h("input", { class: "dge-find-input", placeholder: "Find on the board", spellcheck: "false" });
      var count = h("span", { class: "dge-muted dge-find-count" });
      var box = h("div", { class: "dge-find", role: "search" }, [input, count, h("button", { title: "Close (Esc)", onclick: closeFind }, ["×"])]);
      finder = { box: box, input: input, count: count, hits: [], at: -1 };
      el.appendChild(box);
      var update = function () {
        var q = input.value.trim().toLowerCase();
        finder.hits = q ? findHits(q) : [];
        finder.at = -1;
        count.textContent = q ? (finder.hits.length ? finder.hits.length + " found" : "nothing found") : "";
      };
      input.addEventListener("input", update);
      input.addEventListener("keydown", function (e) {
        e.stopPropagation();
        if (e.key === "Escape") return closeFind();
        if (e.key === "Enter" && finder.hits.length) {
          finder.at = (finder.at + (e.shiftKey ? -1 : 1) + finder.hits.length) % finder.hits.length;
          var id = finder.hits[finder.at], g = geom(id);
          count.textContent = (finder.at + 1) + " of " + finder.hits.length;
          if (g) { focusOn(g, Math.max(st.zoom, 0.8), 1.5, 0.5); select([id]); }
          else flash("“" + labelOf(id) + "” is folded away");
        }
      });
      input.focus();
    }
    function closeFind() {
      if (!finder) return;
      finder.box.remove();
      finder = null;
      canvas.focus({ preventScroll: true });
    }
    /* What matches: labels, details, cells, owners; sections by title. Board order. */
    function findHits(q) {
      var has = function (s) { return s != null && String(s).toLowerCase().indexOf(q) >= 0; };
      return st.doc.nodes.filter(function (n) {
        return has(n.label || n.id) || has(n.detail) || has(n.owner) || (n.cells || []).some(has);
      }).map(function (n) { return n.id; }).concat((st.doc.groups || []).filter(function (g) { return has(g.label || g.id); }).map(function (g) { return g.id; }));
    }

    // ------------------------------------------------------------ present
    /* Present: the board's blocks one at a time, in reading order (rows, then
       left to right); ← / → (or Space) to move, Esc to stop. */
    function slides() {
      var r = st.r, items = [];
      (r.groups || []).forEach(function (g) {
        var gr = group(g.id);
        if (gr && !gr.parent) items.push({ id: g.id, x: g.x, y: g.y, w: g.w, h: g.h, title: gr.label || gr.id });
      });
      r.nodes.forEach(function (n) {
        var nd = node(n.id);
        if (nd && !nd.group && !nd.parent && nd.mindmap) {
          var b = n.map ? null : n; // the centre: its whole map
          var map = r.nodes.filter(function (m) { return m.map && m.map.root === n.id; }).concat([n]);
          var x0 = Math.min.apply(null, map.map(function (m) { return m.x; })), y0 = Math.min.apply(null, map.map(function (m) { return m.y; }));
          var x1 = Math.max.apply(null, map.map(function (m) { return m.x + m.w; })), y1 = Math.max.apply(null, map.map(function (m) { return m.y + m.h; }));
          items.push({ id: n.id, x: x0, y: y0, w: x1 - x0, h: y1 - y0, title: nd.label || nd.id });
        }
      });
      var row = function (it) { return Math.round(it.y / 120); };
      items.sort(function (a, b) { return row(a) - row(b) || a.x - b.x; });
      return items;
    }
    /* Presentation mode: the board's slides in order (the presentation in
       the file), each a section, block, box or canvas; the order is drawn
       on the board. Present plays them with their transitions. */
    var TRANSITIONS = [["glide", "Glide"], ["fade", "Fade"], ["zoom", "Zoom out and in"], ["none", "Cut"]];
    var deck = null;
    function deckOf() { return st.doc && st.doc.presentation; }
    function slideRect(target) {
      var r = st.r;
      if (target === "*") return { x: r.view.x, y: r.view.y, w: r.view.w, h: r.view.h };
      var g = (r.groups || []).find(function (x) { return x.id === target; });
      if (g) return g;
      var n = r.nodes.find(function (x) { return x.id === target; });
      if (!n) return null;
      var nd = node(target);
      if (nd && nd.mindmap) {
        var map = r.nodes.filter(function (m) { return m.map && m.map.root === target; }).concat([n]);
        var x0 = Math.min.apply(null, map.map(function (m) { return m.x; })), y0 = Math.min.apply(null, map.map(function (m) { return m.y; }));
        var x1 = Math.max.apply(null, map.map(function (m) { return m.x + m.w; })), y1 = Math.max.apply(null, map.map(function (m) { return m.y + m.h; }));
        return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
      }
      return n;
    }
    function targetName(t) {
      if (t === "*") return "The whole board";
      var g = group(t), n = node(t);
      return (g && (g.label || g.id)) || (n && (n.label || n.id)) || t;
    }
    function targetKind(t) {
      if (t === "*") return "overview";
      var g = group(t), n = node(t);
      if (n && n.canvas) return "canvas";
      if (n && n.mindmap) return "mindmap";
      if (g) return g.todo ? "to-do list" : g.screen ? (g.screen === "web" ? "web page" : "screen") : g.timeline ? "timeline" : g.cycle ? "cycle" : g.kanban ? "kanban" : g.gantt ? "gantt" : g.risks ? "risks" : g.table ? "table" : g.chart ? "chart" : g.adr ? "decision" : g.sequence ? "sequence" : g.lanes ? "swimlanes" : g.entity ? "table" : g.schema ? "schema" : "section";
      return "box";
    }
    // what can be a slide: the top-level sections and blocks, mindmaps and
    // boxes that open canvases, in reading order
    function slideCandidates() {
      return slides().map(function (it) { return it.id; }).concat(
        (st.doc.nodes || []).filter(function (n) { return n.canvas && !n.group; }).map(function (n) { return n.id; }));
    }
    function setDeck(change) {
      var ops = [Object.assign({ op: "presentation" }, change)];
      return applyOps(ops, false, true);
    }
    function slidesNow() { return ((deckOf() || {}).slides || []).map(function (s) { return Object.assign({}, s); }); }
    function openDeck() {
      if (st.preview) return flash("Keep or discard the AI's changes first", true);
      if (st.canvasPath.length && !openCanvas([])) return;
      if (deck) return;
      deck = h("aside", { class: "dge-deck", "aria-label": "Presentation" });
      el.appendChild(deck);
      el.classList.add("dge-decking");
      renderDeck();
      render();
    }
    function closeDeck() {
      if (!deck) return;
      deck.remove();
      deck = null;
      el.classList.remove("dge-decking");
      render();
    }
    function renderDeck() {
      if (!deck) return;
      var p = deckOf();
      deck.innerHTML = "";
      var head = h("div", { class: "dge-deck-head" }, [
        h("b", {}, ["Presentation"]),
        h("button", { class: "primary", "data-act": "play", title: "Present full screen (→ next, ← back, N notes, Esc stop)", onclick: function () { present(); } }, ["▶ Present"]),
        h("button", { class: "dge-x", title: "Close presentation mode", onclick: closeDeck }, ["×"])
      ]);
      deck.appendChild(head);
      if (!p || !p.slides || !p.slides.length) {
        var auto = slides();
        deck.appendChild(h("div", { class: "dge-deck-empty" }, [
          h("p", {}, ["No slides yet. A slide shows one part of the board: a section, a block, a mindmap, a canvas, or the whole board."]),
          auto.length ? h("button", { class: "primary", onclick: function () {
            setDeck({ title: (deckOf() || {}).title || st.doc.title || "", slides: [{ target: "*", title: st.doc.title || "Overview" }].concat(auto.map(function (it) { return { target: it.id }; })) });
            renderDeck();
          } }, ["Start with the board's sections (" + (auto.length + 1) + " slides)"]) : null,
          h("button", { onclick: draftSlides }, ["✦ Draft slides with the AI"])
        ].filter(Boolean)));
      }
      var title = h("input", { class: "dge-input", value: (p && p.title) || st.doc.title || "", placeholder: "Title of the presentation" });
      title.addEventListener("change", function () { setDeck({ title: title.value }); });
      var trans = h("select", { class: "dge-select", title: "How slides come in" });
      TRANSITIONS.forEach(function (t) { trans.appendChild(h("option", { value: t[0] }, [t[1]])); });
      trans.value = (p && p.transition) || "glide";
      trans.addEventListener("change", function () { setDeck({ transition: trans.value }); });
      var ins = h("textarea", { class: "dge-input", rows: "3", placeholder: "Who it is for, how long, what they should leave with. The AI follows this when it drafts the slides." });
      ins.value = (p && p.instructions) || "";
      ins.addEventListener("change", function () { setDeck({ instructions: ins.value }); });
      deck.appendChild(h("div", { class: "dge-deck-settings" }, [
        h("label", {}, [h("span", {}, ["Title"]), title]),
        h("label", {}, [h("span", {}, ["Transition"]), trans]),
        h("label", {}, [h("span", {}, ["Instructions"]), ins]),
        p && p.slides && p.slides.length ? h("button", { class: "dge-deck-draft", onclick: draftSlides, title: "The AI rewrites the slides (order, titles, notes) from the board and the instructions; you see the changes before keeping them" }, ["✦ Redraft the slides with the AI"]) : null
      ].filter(Boolean)));
      var list = h("ol", { class: "dge-deck-list" });
      ((p && p.slides) || []).forEach(function (sl, i, all) {
        var move = function (d) {
          var s2 = slidesNow(), j = i + d;
          if (j < 0 || j >= s2.length) return;
          var x = s2[i]; s2[i] = s2[j]; s2[j] = x;
          setDeck({ slides: s2 });
          renderDeck();
        };
        var drop = function () { var s2 = slidesNow(); s2.splice(i, 1); setDeck({ slides: s2 }); renderDeck(); };
        var patchSlide = function (k, v) { var s2 = slidesNow(); s2[i][k] = v || undefined; setDeck({ slides: s2 }); };
        var t2 = h("select", { class: "dge-select", title: "How this slide comes in" });
        t2.appendChild(h("option", { value: "" }, ["Transition: as set above"]));
        TRANSITIONS.forEach(function (t) { t2.appendChild(h("option", { value: t[0] }, [t[1]])); });
        t2.value = sl.transition || "";
        t2.addEventListener("change", function () { patchSlide("transition", t2.value); });
        var stitle = h("input", { class: "dge-input dge-slide-title", value: sl.title || "", placeholder: targetName(sl.target) });
        stitle.addEventListener("change", function () { patchSlide("title", stitle.value.trim()); });
        var notes = h("textarea", { class: "dge-input", rows: "2", placeholder: "Speaker notes: what to say" });
        notes.value = sl.notes || "";
        notes.addEventListener("change", function () { patchSlide("notes", notes.value.trim()); });
        var card = h("li", { class: "dge-slide", "data-slide": String(i) }, [
          h("div", { class: "dge-slide-top" }, [
            h("span", { class: "dge-slide-num" }, [String(i + 1)]),
            h("button", { class: "dge-slide-go", title: "Show it on the board", onclick: function () { var r = slideRect(sl.target); if (r) focusOn(r, 1.5, 1.5, 0.8); } }, [h("b", {}, [sl.title || targetName(sl.target)]), h("span", {}, [targetKind(sl.target)])]),
            h("button", { title: "Earlier", disabled: i === 0, onclick: function () { move(-1); } }, ["↑"]),
            h("button", { title: "Later", disabled: i === all.length - 1, onclick: function () { move(1); } }, ["↓"]),
            sl.target !== "*" ? h("button", { title: "Comment on " + targetName(sl.target), onclick: function (e) { var b = e.currentTarget.getBoundingClientRect(); select([sl.target]); openComposer({ on: sl.target }, b.left - 280, b.bottom + 4); } }, [h("span", { html: icon("comment") })]) : null,
            h("button", { title: "Remove the slide (the board keeps it)", onclick: drop }, ["×"])
          ].filter(Boolean)),
          stitle, notes, t2
        ]);
        list.appendChild(card);
        if (i < all.length - 1) list.appendChild(h("li", { class: "dge-slide-then", "aria-hidden": "true" }, ["↓ then"]));
      });
      deck.appendChild(list);
      var have = ((p && p.slides) || []).map(function (s) { return s.target; });
      var add = h("select", { class: "dge-select dge-deck-add" });
      add.appendChild(h("option", { value: "" }, ["+ Add a slide…"]));
      if (st.sel.length === 1 && have.indexOf(st.sel[0]) < 0 && (group(st.sel[0]) || node(st.sel[0]))) add.appendChild(h("option", { value: st.sel[0] }, ["The selection: " + targetName(st.sel[0])]));
      add.appendChild(h("option", { value: "*" }, ["The whole board"]));
      slideCandidates().filter(function (t) { return have.indexOf(t) < 0; }).forEach(function (t) { add.appendChild(h("option", { value: t }, [targetName(t) + " (" + targetKind(t) + ")"])); });
      add.addEventListener("change", function () {
        if (!add.value) return;
        setDeck({ slides: slidesNow().concat([{ target: add.value }]) });
        renderDeck();
      });
      deck.appendChild(add);
    }
    /* The AI drafts the slides from the board and the instructions. */
    function draftSlides() {
      var p = deckOf() || {};
      var ins = "Draft the presentation of this board (only the top board's sections, blocks, mindmaps, boxes that open canvases, or `*` for the whole board can be slides). " +
        (p.instructions ? "Its instructions: " + p.instructions + ". " : "Make it a short walk-through for someone new to the board. ") +
        "Reply with one `presentation` op: a title, the slides in the order to tell the story (start with an overview, `*`), a short title for each that says its point, and speaker notes (one or two sentences: what to say). Keep the transition as it is. Change nothing else on the board.";
      runRethink(ins);
    }
    // the order of the slides, drawn on the board while in presentation mode
    function drawDeckOrder(layer) {
      var p = deckOf();
      if (!deck || !p || !p.slides || !st.r) return;
      var ns = "http://www.w3.org/2000/svg", pts = [];
      p.slides.forEach(function (sl, i) {
        var r = slideRect(sl.target);
        if (!r) return;
        if (sl.target !== "*") {
          var frame = document.createElementNS(ns, "rect");
          frame.setAttribute("class", "dge-deck-frame");
          ["x", "y", "width", "height"].forEach(function (k, j) { frame.setAttribute(k, [r.x - 8, r.y - 8, r.w + 16, r.h + 16][j]); });
          frame.setAttribute("rx", 12);
          layer.appendChild(frame);
        }
        // the whole board: its badge above the board's top-left corner
        var cx = sl.target === "*" ? r.x - 30 : r.x - 8, cy = sl.target === "*" ? r.y - 30 : r.y - 8;
        pts.push([cx, cy, i, cx, cy, sl.target === "*"]);
      });
      // badge to badge, arcing above the boards between them
      for (var k = 0; k + 1 < pts.length; k++) {
        var a = pts[k], b = pts[k + 1];
        if (a[5] || b[5]) continue;
        var line = document.createElementNS(ns, "path");
        var dx = b[0] - a[0], dy = b[1] - a[1], len = Math.sqrt(dx * dx + dy * dy) || 1;
        var x0 = a[0] + dx / len * 16, y0 = a[1] + dy / len * 16, x1 = b[0] - dx / len * 18, y1 = b[1] - dy / len * 18;
        var mx = (x0 + x1) / 2, my = Math.min(y0, y1) - Math.min(70, len / 5 + 16);
        line.setAttribute("d", "M" + x0 + "," + y0 + " Q" + mx + "," + my + " " + x1 + "," + y1);
        line.setAttribute("class", "dge-deck-path");
        line.setAttribute("marker-end", "url(#dge-deck-arrow)");
        layer.appendChild(line);
      }
      pts.forEach(function (q) {
        var c = document.createElementNS(ns, "circle");
        c.setAttribute("cx", q[3]); c.setAttribute("cy", q[4]); c.setAttribute("r", 13);
        c.setAttribute("class", "dge-deck-num");
        var t = document.createElementNS(ns, "text");
        t.setAttribute("x", q[3]); t.setAttribute("y", q[4] + 4.5);
        t.setAttribute("class", "dge-deck-num-text");
        t.textContent = String(q[2] + 1);
        layer.appendChild(c); layer.appendChild(t);
      });
    }
    /* Playing it: the slides of the presentation (or, without one, the
       board's sections in reading order), full screen. */
    function present() {
      var p = deckOf(), list;
      if (st.canvasPath.length) openCanvas([]);
      if (p && p.slides && p.slides.length) {
        list = p.slides.map(function (sl) { return { target: sl.target, title: sl.title || targetName(sl.target), notes: sl.notes || "", transition: sl.transition || p.transition || "glide" }; });
      } else {
        list = slides().map(function (it) { return { target: it.id, title: it.title, notes: "", transition: "glide" }; });
      }
      if (!list.length) { fit(); return flash("Nothing to present yet: add sections, charts, tables or mindmaps"); }
      var bar = h("div", { class: "dge-present-bar" }, []);
      var notes = h("div", { class: "dge-present-notes", hidden: true });
      st.presenting = { list: list, at: -1, bar: bar, notes: notes, showNotes: false };
      el.classList.add("dge-presenting");
      el.appendChild(notes);
      el.appendChild(bar);
      if (el.requestFullscreen && !document.fullscreenElement) el.requestFullscreen().catch(function () { /* fine: in the window */ });
      presentStep(1);
    }
    function presentStep(d) {
      var p = st.presenting;
      if (!p) return;
      var at = Math.max(0, Math.min(p.list.length - 1, p.at + d));
      if (at === p.at) return;
      var first = p.at < 0;
      p.at = at;
      var it = p.list[at];
      var canvasId = (node(it.target) || {}).canvas;
      var go = function () {
        if (canvasId) { openCanvas([canvasId]); fit(); }
        else {
          if (st.canvasPath.length) openCanvas([]);
          var r = slideRect(it.target);
          if (it.target === "*" || !r) fit(); else focusOn(r, 4, 4, 0.88);
        }
      };
      var how = first ? "none" : it.transition;
      stage.style.transition = "";
      if (how === "fade") {
        stage.style.transition = "opacity 180ms ease";
        stage.style.opacity = "0";
        setTimeout(function () { stage.style.transition = ""; go(); requestAnimationFrame(function () { stage.style.transition = "opacity 240ms ease"; stage.style.opacity = "1"; }); }, 190);
      } else if (how === "glide" && !canvasId && !st.canvasPath.length) {
        stage.style.transition = "transform 650ms cubic-bezier(.4,0,.2,1)";
        go();
      } else if (how === "zoom" && !canvasId && !st.canvasPath.length) {
        stage.style.transition = "transform 380ms cubic-bezier(.4,0,.2,1)";
        fit();
        setTimeout(function () { stage.style.transition = "transform 520ms cubic-bezier(.4,0,.2,1)"; go(); }, 400);
      } else {
        go();
      }
      p.bar.innerHTML = "";
      p.bar.appendChild(h("button", { title: "Before (←)", onclick: function () { presentStep(-1); } }, ["←"]));
      p.bar.appendChild(h("span", {}, [(at + 1) + " / " + p.list.length + " · " + it.title]));
      p.bar.appendChild(h("button", { title: "Next (→)", onclick: function () { presentStep(1); } }, ["→"]));
      p.bar.appendChild(h("button", { title: "Speaker notes (N)", "aria-pressed": String(p.showNotes), onclick: function () { toggleNotes(); } }, ["Notes"]));
      p.bar.appendChild(h("button", { title: "Stop (Esc)", onclick: endPresent }, ["Done"]));
      p.notes.textContent = it.notes || "No notes for this slide.";
      p.notes.hidden = !p.showNotes;
    }
    function toggleNotes() {
      var p = st.presenting;
      if (!p) return;
      p.showNotes = !p.showNotes;
      p.notes.hidden = !p.showNotes;
      var b = p.bar.querySelector("[aria-pressed]");
      if (b) b.setAttribute("aria-pressed", String(p.showNotes));
    }
    function endPresent() {
      var p = st.presenting;
      if (!p) return;
      st.presenting = null;
      p.bar.remove();
      p.notes.remove();
      stage.style.transition = "";
      stage.style.opacity = "";
      el.classList.remove("dge-presenting");
      if (document.fullscreenElement && document.exitFullscreen) document.exitFullscreen().catch(function () { /* fine */ });
      if (st.canvasPath.length) openCanvas([]);
      fit();
    }

    /* Swimlanes with two lanes; the selected blocks go in the first. */
    function addSwimlanes() {
      var members = st.sel.filter(function (id) { return (node(id) && !node(id).parent) || (group(id) && !group(id).lanes); });
      var ids = freshIds(["swimlanes", "lane", "lane"]), id = ids[0], l1 = ids[1], l2 = ids[2];
      var res = applyOps([
        { op: "add_group", id: id, label: "Swimlanes", lanes: true },
        { op: "add_group", id: l1, label: "Lane 1", parent: id, members: members },
        { op: "add_group", id: l2, label: "Lane 2", parent: id }
      ].concat(members.filter(function (m) { return node(m); }).map(function (m) { return { op: "unpin", id: m }; })));
      if (res.ok) {
        select([id]);
        reveal(id);
        editLabel(id, true);
      }
    }
    /* The messages of the sequence diagram arrow `e` is in, top to bottom ([] if none). */
    function messagesOf(e) {
      var a = node(e.from), b = node(e.to), g = a && b && a.group === b.group && a.group && group(a.group);
      if (!g || !g.sequence) return [];
      return (st.doc.edges || []).filter(function (m) { var x = node(m.from), y = node(m.to); return x && y && x.group === g.id && y.group === g.id; });
    }
    /* A sequence diagram with two participants and a request and its reply. */
    function addSequence() {
      var ids = freshIds(["sequence", "client", "server"]), gid = ids[0];
      var res = applyOps([{ op: "merge", dsl: "sequence " + gid + ": Sequence {\n  " + ids[1] + ": Client\n  " + ids[2] + ": Server\n  " + ids[1] + " -> " + ids[2] + ": request\n  " + ids[2] + " ..> " + ids[1] + ": response\n}" }]);
      if (res.ok) {
        select([gid]);
        reveal(gid);
        editLabel(gid, true);
      }
    }
    /* A kanban board: To do, Doing, Done; the selected blocks go in To do. */
    function addKanban() {
      var members = st.sel.filter(function (id) { return node(id) && !node(id).parent && !chartOf(id); });
      var ids = freshIds(["kanban", "todo", "doing", "done"]), id = ids[0];
      var res = applyOps([
        { op: "add_group", id: id, label: "Kanban", kanban: true },
        { op: "add_group", id: ids[1], label: "To do", parent: id, members: members },
        { op: "add_group", id: ids[2], label: "Doing", parent: id },
        { op: "add_group", id: ids[3], label: "Done", parent: id }
      ].concat(members.map(function (m) { return { op: "unpin", id: m }; })));
      if (res.ok) {
        select([id]);
        reveal(id);
        editLabel(id, true);
      }
    }
    /* A card at the bottom of a kanban column; its text is typed next. */
    function addCard(col) {
      var id = freshId("card");
      if (!applyOps([{ op: "add_node", id: id, label: "Card", group: col }]).ok) return;
      select([id]);
      reveal(id);
      editLabel(id, true);
    }
    /* A kanban column (a lane of a kanban board). */
    function kanbanColumn(id) {
      var g = id && group(id), p = g && g.parent && group(g.parent);
      return !!(p && p.kanban);
    }
    function lanesOf(id) { return (st.doc.groups || []).filter(function (g) { return g.parent === id; }); }
    function addLane(container, after) {
      var lanes = lanesOf(container), kb = group(container).kanban, nid = freshId(kb ? "column" : "lane");
      var res = applyOps([{ op: "add_group", id: nid, label: (kb ? "Column " : "Lane ") + (lanes.length + 1), parent: container }]);
      if (!res.ok) return;
      // a new lane goes right after the one it was asked from (groups keep document order)
      if (after) {
        var gs = st.doc.groups.map(function (g) { return g.id; }).filter(function (x) { return x !== nid; });
        gs.splice(gs.indexOf(after) + 1, 0, nid);
        reorderGroups(gs);
      }
      select([nid]);
      editLabel(nid, true);
    }
    /* Groups in the given order, by rewriting the DSL: the core keeps groups in document order. */
    function reorderGroups(ids) {
      // the open canvas only (the whole board when none is open)
      var doc = JSON.parse(st.session.document());
      doc.groups.sort(function (a, b) { return ids.indexOf(a.id) - ids.indexOf(b.id); });
      var before = st.text;
      var res = JSON.parse(st.session.replace_here(JSON.stringify(doc)));
      if (!res.ok) { st.session.set_source(before); return; }
      // part of the change that made the lane: no undo step of its own
      st.text = st.session.source("dsl");
      refreshDoc();
      afterChange(true);
    }

    /* The innermost of `sections` under a point, if any. */
    function sectionAt(sections, x, y) {
      var best = null;
      sections.forEach(function (g) {
        if (x >= g.x && x <= g.x + g.w && y >= g.y && y <= g.y + g.h && (!best || g.w * g.h < best.w * best.h)) best = g;
      });
      return best ? best.id : null;
    }

    /* What dropping a move drag does. Live (while dragging) it keeps everything in its
       section, except a task, which may leave its chart. */
    function dropOps(d, live) {
      var ops = d.items.map(function (m) { return { op: "move", id: m.id, x: m.x + d.dx, y: m.y + d.dy }; });
      d.items.forEach(function (m) {
        if (!m.direct) return;
        var into = sectionAt(d.sections, m.x + d.dx + m.w / 2, m.y + d.dy + m.h / 2), was = node(m.id).group || null;
        if (live && into !== was && !chartOf(m.id)) return;
        var sq = d.sections.find(function (g) { return g.id === into && g.seq; });
        if (!live && sq && into === was) {
          // a participant goes where it was dropped, left to right
          var cx = m.x + d.dx + m.w / 2, at = 0;
          st.doc.nodes.forEach(function (c) {
            var g = c.group === into && c.id !== m.id && geom(c.id);
            if (g && g.x + g.w / 2 < cx) at++;
          });
          ops.push({ op: "update_node", id: m.id, group: into, index: at });
        } else if (!live && kanbanColumn(into)) {
          // a card goes where it was dropped among the column's cards
          var cy = m.y + d.dy + m.h / 2, index = 0;
          st.doc.nodes.forEach(function (c) {
            var g = c.group === into && c.id !== m.id && geom(c.id);
            if (g && g.y + g.h / 2 < cy) index++;
          });
          ops.push({ op: "update_node", id: m.id, group: into, index: index });
        } else if (into !== was) ops.push({ op: "update_node", id: m.id, group: into || "" });
        // lanes and charts are laid out automatically: no pin inside one
        var sec = d.sections.find(function (g) { return g.id === into; });
        if (sec && (sec.lane || sec.gantt || sec.matrix)) ops = ops.filter(function (o) { return !(o.op === "move" && o.id === m.id); }).concat([{ op: "unpin", id: m.id }]);
        // a risk dropped on a cell gets its probability and impact; off the grid, none
        if (sec && sec.matrix && into === was) {
          var mx = sec.matrix, cx = m.x + d.dx + m.w / 2, cy = m.y + d.dy + m.h / 2;
          var col = Math.floor((cx - mx.left) / mx.cell), row = Math.floor((cy - mx.top) / mx.cell);
          var on = col >= 0 && col < mx.levels && row >= 0 && row < mx.levels;
          ops.push({ op: "update_node", id: m.id, probability: on ? mx.levels - row : 0, impact: on ? col + 1 : 0 });
        }
        // a bar moved along its own chart starts on the day it was dropped at
        if (sec && sec.gantt && into === was) {
          var gt = sec.gantt, origin = gt.origin != null ? gt.origin : today();
          var at = m.x + d.dx + (node(m.id).milestone ? m.w / 2 : 0);
          var unit = snapDays(gt.scale), day = Math.round((at - gt.left) / gt.day / unit) * unit;
          if (gt.origin == null) ops.push({ op: "update_group", id: into, start: isoDay(origin) });
          ops.push({ op: "update_node", id: m.id, start: isoDay(origin + day) });
        }
      });
      return ops;
    }

    function duplicateSelection() {
      var ops = [], ids = [];
      st.sel.forEach(function (id) {
        var n = node(id), g = geom(id);
        if (!n || !g || mapInfo(id)) return;
        var nid = freshId(n.id + "-copy");
        while (ids.indexOf(nid) >= 0) nid = nid + "-x";
        ids.push(nid);
        ops.push({ op: "add_node", id: nid, label: n.label || n.id, shape: n.shape, color: n.color, detail: n.detail, group: n.group, x: Math.round(g.x + 24), y: Math.round(g.y + 24) });
      });
      if (ops.length && applyOps(ops).ok) select(ids);
    }

    function nudge(dx, dy) {
      var ops = [];
      st.sel.forEach(function (id) {
        var g = geom(id);
        if (node(id) && g && !isBranch(id)) ops.push({ op: "move", id: id, x: g.x + dx, y: g.y + dy });
        else if (blockOf(id)) { var b = geom(blockOf(id)); ops.push({ op: "move", id: blockOf(id), x: b.x + dx, y: b.y + dy }); }
      });
      if (ops.length) applyOps(ops);
    }

    /* The Arc browser (it sets its palette as CSS variables on the page). */
    function isArc() {
      try { return !!getComputedStyle(document.documentElement).getPropertyValue("--arc-palette-title"); } catch (_) { return false; }
    }
    function onKey(e) {
      var tag = (e.target && e.target.tagName) || "";
      var typing = tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || (e.target && e.target.isContentEditable);
      var mod = e.metaKey || e.ctrlKey;
      if (!modal.hidden) {
        if (e.key === "Escape") closeModal();
        return;
      }
      if (mod && e.key.toLowerCase() === "s") {
        // Arc uses ⌘S for its sidebar: save, but let the browser have the key
        if (st.answer) { e.preventDefault(); flash("This answer is not saved with the board: Save as canvas keeps it"); return; }
        if (opts.onSave) {
          if (!isArc()) e.preventDefault();
          opts.onSave(st.text, api);
        }
        return;
      }
      if (!aiMenu.hidden && e.key === "Escape") { e.preventDefault(); closeAiMenu(); return; }
      if (blocksMenu && !blocksMenu.hidden && e.key === "Escape") { e.preventDefault(); closeBlocks(); return; }
      if (boardMenu && !boardMenu.hidden && e.key === "Escape") { e.preventDefault(); boardMenu.hidden = true; return; }
      if (!askPanel.hidden && e.key === "Escape") { e.preventDefault(); closeAsk(); return; }
      if (st.answer && e.key === "Escape" && !st.sel.length) { e.preventDefault(); closeAnswer(); return; }
      if (mod && e.key.toLowerCase() === "k" && !view) { e.preventDefault(); askAI(); return; }
      if (mod && e.key.toLowerCase() === "f") { e.preventDefault(); openFind(); return; }
      if (st.presenting) {
        if (e.key === "Escape") { e.preventDefault(); endPresent(); }
        else if (e.key === "n" || e.key === "N") { e.preventDefault(); toggleNotes(); }
        else if (e.key === "ArrowRight" || e.key === " " || e.key === "PageDown") { e.preventDefault(); presentStep(1); }
        else if (e.key === "ArrowLeft" || e.key === "PageUp") { e.preventDefault(); presentStep(-1); }
        return;
      }
      if (mod && e.key === "/" && hasCode) { e.preventDefault(); showCode = !showCode; applyChrome(); syncCode(true); return; }
      if (typing || !el.contains(document.activeElement) && document.activeElement !== document.body) return;
      if (e.key === "Alt") canvas.classList.add("out");
      if (e.key === " " && !st.space) { st.space = true; canvas.classList.add("space"); e.preventDefault(); return; }
      if (mod && e.key.toLowerCase() === "z") { e.preventDefault(); if (e.shiftKey) redo(); else undo(); return; }
      if (mod && e.key.toLowerCase() === "y") { e.preventDefault(); redo(); return; }
      if (mod && e.shiftKey && e.key.toLowerCase() === "c") { e.preventDefault(); if (st.sel.length) copyPrompt(); else copyBoardPrompt(); return; }
      if (view) {
        if (e.key === "f") fit();
        return;
      }
      var one = st.sel.length === 1 && mapInfo(st.sel[0]) ? st.sel[0] : null;
      if (one && mod && !e.shiftKey && e.key.toLowerCase() === "c" && !String(root.getSelection ? root.getSelection() : "")) {
        e.preventDefault();
        copyText(st.session.outline(one)).then(function () { flash("Outline copied"); });
        return;
      }
      // a to-do item: Enter adds the next, Tab / ⇧Tab change its level (Space stays panning)
      if (st.sel.length === 1 && !mod && todoOf(st.sel[0])) {
        var ti = node(st.sel[0]);
        if (e.key === "Enter") { e.preventDefault(); addTodoItem(todoOf(ti.id).id, ti.id); return; }
        if (e.key === "Tab") { e.preventDefault(); applyOps([{ op: "update_node", id: ti.id, level: Math.max(0, (ti.level || 0) + (e.shiftKey ? -1 : 1)) }]); select([ti.id]); return; }
      }
      // a screen's component: ⌥↑ ⌥↓ move it
      if (st.sel.length === 1 && e.altKey && !mod && (e.key === "ArrowUp" || e.key === "ArrowDown") && node(st.sel[0]) && screenOf(st.sel[0])) {
        e.preventDefault(); moveComponent(st.sel[0], e.key === "ArrowUp" ? -1 : 1); return;
      }
      if (one && !mod) {
        if (e.key === "Tab") { e.preventDefault(); if (e.shiftKey) outdent(one); else addTopic(one); return; }
        if (e.key === "Enter") { e.preventDefault(); if (node(one).parent) addSibling(one, !e.shiftKey); else addTopic(one); return; }
        if (e.key === "F2") { e.preventDefault(); editLabel(one); return; }
        if (e.key === ".") { toggleFold(one); return; }
        if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) { e.preventDefault(); moveSibling(one, e.key === "ArrowUp" ? -1 : 1); return; }
        if (/^Arrow/.test(e.key)) { e.preventDefault(); if (navigate(e.key)) { reveal(st.sel[0]); return; } }
      }
      if (mod && e.key.toLowerCase() === "a") { e.preventDefault(); select(st.doc.nodes.map(function (n) { return n.id; })); return; }
      if (mod && e.key.toLowerCase() === "d") { e.preventDefault(); duplicateSelection(); return; }
      if (mod && e.key.toLowerCase() === "g") { e.preventDefault(); groupSelection(); return; }
      if (mod) return;
      switch (e.key) {
        case "v": case "V": setTool("select"); break;
        case "h": case "H": setTool("hand"); break;
        case "t": case "T": setTool("mindmap"); break;
        case "n": case "N": case "r": setTool("node"); break;
        case "s": case "S": setTool("sticky"); break;
        case "c": case "C": setTool("comment"); break;
        case "x": case "X": setTool("connect"); break;
        case "d": case "D": setTool("decision"); break;
        case "f": fit(); break;
        case "m": case "M": toggleMinimap(); break;
        case "z": case "Z": setTool(st.tool === "zoom" ? "select" : "zoom"); break;
        case "0": zoomTo(1); break;
        case "+": case "=": zoomTo(st.zoom * 1.2); break;
        case "-": zoomTo(st.zoom / 1.2); break;
        case "Enter": if (st.sel.length === 1) { e.preventDefault(); editLabel(st.sel[0]); } break;
        case "Delete": case "Backspace": e.preventDefault(); deleteSelection(); break;
        case "Escape": if (st.linking) endLinking(); else if (!bubble.hidden) closeBubble(); else if (st.tool !== "select") setTool("select"); else select([]); closeMenu(); break;
        case "ArrowLeft": e.preventDefault(); nudge(e.shiftKey ? -40 : -8, 0); break;
        case "ArrowRight": e.preventDefault(); nudge(e.shiftKey ? 40 : 8, 0); break;
        case "ArrowUp": e.preventDefault(); nudge(0, e.shiftKey ? -40 : -8); break;
        case "ArrowDown": e.preventDefault(); nudge(0, e.shiftKey ? 40 : 8); break;
        case "?": flash("V select · H hand · T mindmap · N box · S sticky · X connect · C comment · double-click to add/edit · " + MOD + "G section · " + MOD + "⇧C copy as prompt · " + MOD + "D duplicate · F fit · M minimap · Z magnifier · space+drag pan. Mindmaps: Tab branch · Enter sibling · ⇧Tab outdent (on the centre: a new centre above it) · right-click for more · F2 rename · . fold · arrows move · ⌥↑↓ reorder · paste a list", false, 9000); break;
        default: return;
      }
    }
    function onKeyUp(e) {
      if (e.key === " ") { st.space = false; canvas.classList.remove("space"); }
      if (e.key === "Alt") canvas.classList.remove("out");
    }
    document.addEventListener("keydown", onKey);
    document.addEventListener("keyup", onKeyUp);

    // ---------------------------------------------------------------- panel
    function field(label, input) {
      return h("label", { class: "dge-field" }, [h("span", {}, [label]), input]);
    }
    function textInput(value, onCommit, attrs) {
      var i = h("input", Object.assign({ type: "text", value: value || "" }, attrs || {}));
      i.addEventListener("change", function () { onCommit(i.value); });
      i.addEventListener("keydown", function (e) { if (e.key === "Enter") i.blur(); e.stopPropagation(); });
      return i;
    }
    function selectInput(options, value, onChange) {
      var s = h("select", { class: "dge-select" });
      options.forEach(function (o) {
        var v = Array.isArray(o) ? o[0] : o, l = Array.isArray(o) ? o[1] : o;
        var opt = h("option", { value: v }, [l]);
        if (v === (value || "")) opt.selected = true;
        s.appendChild(opt);
      });
      s.addEventListener("change", function () { onChange(s.value); });
      return s;
    }
    function swatches(value, onPick) {
      var row = h("div", { class: "dge-swatches" });
      row.appendChild(h("button", { class: "dge-swatch none" + (!value ? " on" : ""), title: "Default", onclick: function () { onPick(""); } }));
      COLORS.forEach(function (c) {
        row.appendChild(h("button", { class: "dge-swatch" + (value === c ? " on" : ""), title: c, style: "background:" + SWATCH[c], onclick: function () { onPick(c); } }));
      });
      return row;
    }

    function renderInspector() {
      inspector.innerHTML = "";
      if (!st.doc) return;
      var sel = st.sel.filter(kindOf);
      if (view) {
        var d0 = st.doc;
        inspector.appendChild(h("h3", {}, [d0.title || "Diagram"]));
        if (d0.description) inspector.appendChild(h("p", { class: "dge-muted" }, [d0.description]));
        return;
      }
      if (!sel.length) {
        var d = st.doc, s = d.settings || {};
        inspector.appendChild(h("h3", {}, ["Diagram"]));
        inspector.appendChild(field("Title", textInput(d.title, function (v) { applyOps([{ op: "set", key: "title", value: v }]); }, { placeholder: "Untitled" })));
        inspector.appendChild(field("Description", textInput(d.description, function (v) { applyOps([{ op: "set", key: "description", value: v }]); })));
        inspector.appendChild(field("Layout", selectInput([["layered", "Flow (layered)"], ["grid", "Grid"]], s.layout || "layered", function (v) { applyOps([{ op: "set", key: "layout", value: v }]); })));
        inspector.appendChild(h("p", { class: "dge-muted" }, [d.nodes.length + " nodes · " + (d.edges || []).length + " edges · " + (d.groups || []).length + " groups"]));
        var about = textInput("", function (v) { if (v.trim()) { newMap(v); } }, { placeholder: "e.g. Team offsite" });
        inspector.appendChild(field("✦ Mindmap about…", about));
        inspector.appendChild(h("p", { class: "dge-tip" }, ["Press T and click to start a mindmap, or paste a list. Double-click the canvas to add a box. Drag a box's dot onto another box to connect them. Press ? for shortcuts."]));
        return;
      }
      if (sel.length > 1) {
        inspector.appendChild(h("h3", {}, [sel.length + " selected"]));
        var nodes = sel.filter(node);
        inspector.appendChild(field("Colour", swatches(null, function (c) {
          applyOps(sel.map(function (id) { return node(id) ? { op: "update_node", id: id, color: c } : edge(id) ? { op: "update_edge", id: id, color: c } : { op: "update_group", id: id, color: c }; }));
        })));
        if (nodes.length) inspector.appendChild(field("Shape", selectInput([""].concat(SHAPES), "", function (v) { if (v) applyOps(nodes.map(function (id) { return { op: "update_node", id: id, shape: v }; })); })));
        inspector.appendChild(h("div", { class: "dge-actions" }, [
          h("button", { onclick: groupSelection }, ["Section (" + MOD + "G)"]),
          h("button", { onclick: function () { applyOps(nodes.map(function (id) { return { op: "unpin", id: id }; })); } }, ["Auto-place"]),
          h("button", { class: "danger", onclick: deleteSelection }, ["Delete"])
        ]));
        return;
      }
      var id = sel[0], kind = kindOf(id);
      if (kind === "node" && mapInfo(id)) return renderTopicInspector(id);
      var commentBtn = h("button", { onclick: function () { st.composer = { on: id }; renderPanel(); var ta = commentsEl.querySelector(".dge-composer textarea"); if (ta) ta.focus(); } }, ["Comment"]);
      if (kind === "node") {
        var n = node(id);
        inspector.appendChild(h("h3", {}, ["Node ", h("code", {}, [id])]));
        inspector.appendChild(field("Label", textInput(n.label || n.id, function (v) { applyOps([{ op: "update_node", id: id, label: v }]); })));
        inspector.appendChild(field("Detail", textInput(n.detail, function (v) { applyOps([{ op: "update_node", id: id, detail: v }]); }, { placeholder: "e.g. Rust · Axum" })));
        inspector.appendChild(field("Shape", selectInput(SHAPES, n.shape || "box", function (v) { applyOps([{ op: "update_node", id: id, shape: v }]); })));
        inspector.appendChild(field("Colour", swatches(n.color, function (c) { applyOps([{ op: "update_node", id: id, color: c }]); })));
        inspector.appendChild(field("Section", selectInput([["", "—"]].concat((st.doc.groups || []).map(function (g) { return [g.id, g.label || g.id]; })), n.group || "", function (v) { applyOps([{ op: "update_node", id: id, group: v }]); })));
        inspector.appendChild(field("Link", textInput(n.link, function (v) { applyOps([{ op: "update_node", id: id, link: v }]); }, { placeholder: "https://…" })));
        inspector.appendChild(field("Tags", textInput((n.tags || []).join(" "), function (v) { applyOps([{ op: "update_node", id: id, tags: v.split(/\s+/).filter(Boolean) }]); }, { placeholder: "service team-a" })));
        inspector.appendChild(field("Id", textInput(id, function (v) { var res = applyOps([{ op: "rename", id: id, to: v.trim() }]); if (res.ok) select([v.trim()]); })));
        var acts = [commentBtn];
        if (n.x != null) acts.push(h("button", { title: "Let the layout place it", onclick: function () { applyOps([{ op: "unpin", id: id }]); } }, ["Auto-place"]));
        acts.push(h("button", { class: "danger", onclick: deleteSelection }, ["Delete"]));
        inspector.appendChild(h("div", { class: "dge-actions" }, acts));
      } else if (kind === "edge") {
        var e = edge(id);
        inspector.appendChild(h("h3", {}, ["Edge ", h("code", {}, [e.from + " → " + e.to])]));
        inspector.appendChild(field("Label", textInput(e.label, function (v) { applyOps([{ op: "update_edge", id: id, label: v }]); })));
        inspector.appendChild(field("Line", selectInput(["solid", "dashed", "dotted", "thick"], e.line || "solid", function (v) { applyOps([{ op: "update_edge", id: id, line: v }]); })));
        inspector.appendChild(field("Arrow", selectInput([["end", "→ end"], ["start", "← start"], ["both", "↔ both"], ["none", "— none"]], e.arrow || "end", function (v) { applyOps([{ op: "update_edge", id: id, arrow: v }]); })));
        inspector.appendChild(field("Colour", swatches(e.color, function (c) { applyOps([{ op: "update_edge", id: id, color: c }]); })));
        inspector.appendChild(h("div", { class: "dge-actions" }, [
          commentBtn,
          h("button", { onclick: function () { var r = applyOps([{ op: "update_edge", id: id, from: e.to, to: e.from }]); if (r.ok) select([]); } }, ["Reverse"]),
          h("button", { class: "danger", onclick: deleteSelection }, ["Delete"])
        ]));
      } else {
        var g = group(id);
        inspector.appendChild(h("h3", {}, ["Section ", h("code", {}, [id])]));
        inspector.appendChild(field("Label", textInput(g.label || g.id, function (v) { applyOps([{ op: "update_group", id: id, label: v }]); })));
        inspector.appendChild(field("Colour", swatches(g.color, function (c) { applyOps([{ op: "update_group", id: id, color: c }]); })));
        inspector.appendChild(field("Inside", selectInput([["", "—"]].concat((st.doc.groups || []).filter(function (x) { return x.id !== id; }).map(function (x) { return [x.id, x.label || x.id]; })), g.parent || "", function (v) { applyOps([{ op: "update_group", id: id, parent: v }]); })));
        inspector.appendChild(h("div", { class: "dge-actions" }, [
          commentBtn,
          h("button", { onclick: function () { applyOps([{ op: "remove", id: id, keep_members: true }]); select([]); } }, ["Remove section"]),
          h("button", { class: "danger", onclick: function () { applyOps([{ op: "remove", id: id, keep_members: false }]); select([]); } }, ["Delete with contents"])
        ]));
      }
    }

    function renderTopicInspector(id) {
      var n = node(id), m = mapInfo(id), centre = !n.parent;
      inspector.appendChild(h("h3", {}, [centre ? "Mindmap " : "Topic ", h("code", {}, [id])]));
      inspector.appendChild(field("Label", textInput(n.label || n.id, function (v) { applyOps([{ op: "update_node", id: id, label: v }]); })));
      inspector.appendChild(field("Note", textInput(n.detail, function (v) { applyOps([{ op: "update_node", id: id, detail: v }]); }, { placeholder: "a line under the topic" })));
      if (centre) inspector.appendChild(field("Grows", selectInput([["right", "→ To the right"], ["both", "↔ Both sides"], ["left", "← To the left"], ["down", "↓ Down (org chart)"]], n.mindmap || "right", function (v) { applyOps([{ op: "update_node", id: id, mindmap: v }]); })));
      inspector.appendChild(field(centre ? "Colour" : "Branch colour", swatches(n.color, function (c) { applyOps([{ op: "update_node", id: id, color: c }]); })));
      inspector.appendChild(field("Link", textInput(n.link, function (v) { applyOps([{ op: "update_node", id: id, link: v }]); }, { placeholder: "https://…" })));
      inspector.appendChild(field("Tags", textInput((n.tags || []).join(" "), function (v) { applyOps([{ op: "update_node", id: id, tags: v.split(/\s+/).filter(Boolean) }]); })));
      inspector.appendChild(h("div", { class: "dge-actions" }, [
        h("button", { title: "Tab", onclick: function () { addTopic(id); } }, ["+ Branch"]),
        n.parent ? h("button", { title: "Enter", onclick: function () { addSibling(id, true); } }, ["+ Sibling"]) : null,
        m.children ? h("button", { title: ".", onclick: function () { toggleFold(id); } }, [n.collapsed ? "Unfold (" + m.folded + ")" : "Fold"]) : null
      ]));
      var askIn = h("textarea", { rows: 2, placeholder: "Optional: what should it add? e.g. “risks”, “questions for legal”" });
      askIn.addEventListener("keydown", function (e) { e.stopPropagation(); if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) expandTopic(id, askIn.value); });
      inspector.appendChild(h("div", { class: "dge-ai" }, [
        askIn,
        h("button", { class: "dge-primary", title: "Ask the model for subtopics (" + MOD + "↵)", onclick: function () { expandTopic(id, askIn.value); } }, [st.thinking[id] ? "✦ Thinking…" : "✦ Grow this topic"])
      ]));
      var root = m.root, placed = placedTopics(root).length;
      if (placed) inspector.appendChild(h("div", { class: "dge-actions" }, [
        n.parent && n.x != null ? h("button", { title: "Put this topic back in the automatic layout", onclick: function () { applyOps([{ op: "unpin", id: id }]); } }, ["Auto-place"]) : null,
        h("button", { title: "Put every topic of this map back in the automatic layout", onclick: function () { resetArrangement(root); } }, ["Reset arrangement" + (placed > 1 ? " (" + placed + ")" : "")])
      ]));
      inspector.appendChild(h("div", { class: "dge-actions" }, [
        h("button", { title: "The branch as an indented outline (" + MOD + "C)", onclick: function () { copyText(st.session.outline(id)).then(function () { flash("Outline copied"); }); } }, ["Copy outline"]),
        h("button", { title: "The whole map as Markdown: the centre is the title, branches are sections. Paste it as a prompt.", onclick: function () { copyText(st.session.markdown(root)).then(function () { flash("Markdown copied — use it as a prompt"); }); } }, ["Copy as prompt"])
      ]));
      inspector.appendChild(h("div", { class: "dge-actions" }, [
        h("button", { onclick: function () { st.composer = { on: id }; renderPanel(); var ta = commentsEl.querySelector(".dge-composer textarea"); if (ta) ta.focus(); } }, ["Comment"]),
        n.parent ? h("button", { title: "Make this branch a mindmap of its own", onclick: function () { var g = geom(id); applyOps([{ op: "update_node", id: id, parent: "" }, { op: "move", id: id, x: Math.round(g.x + 40), y: Math.round(g.y + 80) }]); } }, ["Detach"]) : null,
        h("button", { class: "danger", onclick: deleteSelection }, [centre ? "Delete map" : "Delete branch"])
      ]));
      inspector.appendChild(h("p", { class: "dge-tip" }, ["Tab: branch · Enter: sibling · ⇧Tab: outdent (on the centre: a new centre above it) · F2: rename · arrows: move around · ⌥↑↓: reorder · drag a topic onto another to move it, or anywhere to place it by hand."]));
    }

    function renderComments() {
      commentsEl.innerHTML = "";
      if (!st.doc) return;
      var all = st.doc.comments || [];
      var open = all.filter(function (c) { return c.status !== "resolved"; });
      var head = h("div", { class: "dge-comments-head" }, [
        h("h3", {}, ["Comments ", h("span", { class: "dge-count" }, [String(open.length)])]),
        h("label", { class: "dge-check" }, [h("input", { type: "checkbox", checked: st.showResolved, onchange: function (e) { st.showResolved = e.target.checked; render(); } }), " resolved"])
      ]);
      commentsEl.appendChild(head);
      if (!view) {
        commentsEl.appendChild(h("div", { class: "dge-actions" }, [
          h("button", { class: "dge-primary", disabled: !open.length, title: "Turn open comments into an LLM prompt", onclick: function () { openPrompt(); } }, ["Copy as prompt…"]),
          h("button", { title: "Paste an LLM reply (ops JSON or a diagram) to apply it", onclick: openApply }, ["Apply reply…"])
        ]));
      }
      if (!st.author && !view) {
        var nameIn = textInput("", function (v) { st.author = v.trim(); lsSet("diagram.author", st.author); renderComments(); }, { placeholder: "Your name (for comments)" });
        commentsEl.appendChild(nameIn);
      }
      if (st.composer) {
        var ta = h("textarea", { rows: 3, placeholder: st.composer.on ? "Comment on " + labelOf(st.composer.on) + "…" : "Comment on the diagram…" });
        ta.addEventListener("keydown", function (e) {
          e.stopPropagation();
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) post();
          if (e.key === "Escape") { st.composer = null; renderComments(); }
        });
        var post = function () {
          var text = ta.value.trim();
          if (!text) return;
          var op = { op: "add_comment", text: text };
          if (st.composer.on) op.on = st.composer.on;
          else { op.x = st.composer.x; op.y = st.composer.y; }
          if (st.author) op.author = st.author;
          st.composer = null;
          var res = applyOps([op]);
          if (res.ok) { st.comment = res.created[0]; markSelection(); renderComments(); }
        };
        commentsEl.appendChild(h("div", { class: "dge-composer" }, [
          ta,
          h("div", { class: "dge-actions" }, [
            h("button", { class: "dge-primary", onclick: post }, ["Comment (" + MOD + "↵)"]),
            h("button", { onclick: function () { st.composer = null; renderComments(); } }, ["Cancel"])
          ])
        ]));
      }
      var list = st.showResolved ? all : open;
      if (!list.length && !st.composer) {
        commentsEl.appendChild(h("p", { class: "dge-muted" }, [view ? "No comments." : "No open comments. Press M and click something to comment on it."]));
      }
      list.forEach(function (c) {
        var resolved = c.status === "resolved";
        var card = h("div", { class: "dge-card" + (resolved ? " resolved" : "") + (st.comment === c.id ? " active" : ""), "data-cid": c.id });
        card.addEventListener("click", function (e) {
          if (e.target.closest("button,textarea,input")) return;
          st.comment = c.id;
          if (c.on && kindOf(c.on)) st.sel = [c.on];
          markSelection();
          renderPanel();
        });
        var target = c.on ? h("button", { class: "dge-link", onclick: function () { select([c.on]); } }, [labelOf(c.on)]) : h("span", { class: "dge-muted" }, ["canvas"]);
        card.appendChild(h("div", { class: "dge-card-head" }, [commentAvatar(c), h("b", {}, [c.author || "anonymous"]), " on ", target, h("code", { class: "dge-cid" }, [c.id])]));
        card.appendChild(h("div", { class: "dge-card-text" }, [c.text]));
        (c.replies || []).forEach(function (r) {
          card.appendChild(h("div", { class: "dge-reply" }, [h("b", {}, [r.author || "anonymous"]), " ", r.text]));
        });
        if (c.resolution) card.appendChild(h("div", { class: "dge-resolution" }, ["✓ " + c.resolution]));
        if (!view) {
          var rin = textInput("", function (v) {
            if (!v.trim()) return;
            var op = { op: "reply", id: c.id, text: v.trim() };
            if (st.author) op.author = st.author;
            applyOps([op]);
          }, { placeholder: "Reply…", class: "dge-reply-in" });
          card.appendChild(rin);
          card.appendChild(h("div", { class: "dge-actions small" }, [
            resolved
              ? h("button", { onclick: function () { applyOps([{ op: "reopen_comment", id: c.id }]); } }, ["Reopen"])
              : h("button", { onclick: function () { applyOps([{ op: "resolve_comment", id: c.id }]); } }, ["Resolve"]),
            h("button", { onclick: function () { openPrompt([c.id]); } }, ["Prompt"]),
            h("button", { class: "danger", onclick: function () { applyOps([{ op: "remove", id: c.id }]); } }, ["Delete"])
          ]));
        }
        commentsEl.appendChild(card);
      });
    }

    function renderPanel() {
      if (panel.hidden) return;
      renderInspector();
      renderComments();
    }

    // ------------------------------------------------- menus, modals, toast
    var toastTimer = null;
    function flash(msg, isError, ms) {
      clearTimeout(toastTimer);
      // an empty message hides the toast
      if (!msg) { toast.className = "dge-toast"; return; }
      toast.textContent = msg;
      toast.className = "dge-toast show" + (isError ? " error" : "");
      clearTimeout(toastTimer);
      toastTimer = setTimeout(function () { toast.className = "dge-toast"; }, ms || (isError ? 5000 : 1800));
    }

    /* A message with a button that does something; it stays a while. */
    function flashAction(msg, label, act) {
      clearTimeout(toastTimer);
      toast.innerHTML = "";
      toast.appendChild(h("span", {}, [msg]));
      toast.appendChild(h("button", { class: "dge-toast-act", onclick: function () { toast.className = "dge-toast"; act(); } }, [label]));
      toast.className = "dge-toast show act";
      toastTimer = setTimeout(function () { toast.className = "dge-toast"; }, 12000);
    }

    function closeMenu() { menu.hidden = true; }

    /* A comment's avatar: its author's initials (lines of text without one). */
    function commentAvatar(c) {
      var a = (c.author || "").trim(), w = a.split(/\s+/).filter(Boolean);
      var ini = (w.length > 1 ? w[0].charAt(0) + w[1].charAt(0) : a.replace(/[^\p{L}\p{N}]/gu, "").slice(0, 2)).toUpperCase();
      return h("span", { class: "dge-num" + (c.status === "resolved" ? " done" : ""), title: a || "No author" }, [ini || "…"]);
    }
    function repliesText(c) { var k = (c.replies || []).length; return k ? k + (k === 1 ? " reply" : " replies") : ""; }
    /* Comment popovers: a box to write a comment, or a thread with replies. */
    function closeBubble() { if (!bubble.hidden) { bubble.hidden = true; bubble.innerHTML = ""; st.comment = null; markSelection(); } }
    function placeBubble(cx, cy) {
      var host = el.getBoundingClientRect();
      bubble.hidden = false;
      var x = cx - host.left + 14, y = cy - host.top + 14;
      bubble.style.left = Math.max(8, Math.min(x, host.width - bubble.offsetWidth - 8)) + "px";
      // stay clear of the floating tool bar at the bottom
      bubble.style.top = Math.max(8, Math.min(y, host.height - bubble.offsetHeight - 68)) + "px";
    }
    function commentBox(placeholder, onSend, label) {
      var ta = h("textarea", { rows: 2, placeholder: placeholder });
      var send = function () { var v = ta.value.trim(); if (v) onSend(v); };
      ta.addEventListener("keydown", function (e) {
        e.stopPropagation();
        if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); }
        if (e.key === "Escape") { e.preventDefault(); closeBubble(); canvas.focus({ preventScroll: true }); }
      });
      return { ta: ta, el: h("div", { class: "dge-bubble-box" }, [ta, h("button", { class: "dge-primary", title: "Enter (⇧Enter for a new line)", onclick: send }, [label])]) };
    }
    function openComposer(target, cx, cy) {
      bubble.innerHTML = "";
      var box = commentBox(target.on ? "Comment on " + labelOf(target.on) + "…" : "Add a comment…", function (text) {
        var op = { op: "add_comment", text: text };
        if (target.on) op.on = target.on; else { op.x = target.x; op.y = target.y; }
        if (st.author) op.author = st.author;
        closeBubble();
        setTool("select");
        applyOps([op]);
        canvas.focus({ preventScroll: true });
      }, "Comment");
      bubble.appendChild(box.el);
      placeBubble(cx, cy);
      setTimeout(function () { box.ta.focus(); });
    }
    function openThread(cid, cx, cy) {
      var all = st.doc.comments || [], c = all.find(function (x) { return x.id === cid; });
      if (!c) return;
      st.comment = cid;
      markSelection();
      bubble.innerHTML = "";
      var resolved = c.status === "resolved";
      var head = h("div", { class: "dge-bubble-head" }, [
        commentAvatar(c),
        h("span", { class: "dge-muted" }, [(resolved ? "Resolved comment " : "Comment ") + (c.on ? "on " + labelOf(c.on) : "on the board")]),
        h("span", { class: "dge-spacer" }),
        view ? null : h("button", { title: resolved ? "Reopen" : "Resolve", onclick: function () { applyOps([{ op: resolved ? "reopen_comment" : "resolve_comment", id: cid }]); closeBubble(); } }, [resolved ? "Reopen" : "✓ Resolve"]),
        view ? null : h("button", { class: "danger", title: "Delete the thread", onclick: function () { applyOps([{ op: "remove", id: cid }]); closeBubble(); } }, ["Delete"])
      ]);
      bubble.appendChild(head);
      [{ author: c.author, text: c.text }].concat(c.replies || []).forEach(function (m) {
        bubble.appendChild(h("div", { class: "dge-msg" }, [h("b", {}, [m.author || "anonymous"]), h("div", {}, [m.text])]));
      });
      if (c.resolution) bubble.appendChild(h("div", { class: "dge-resolution" }, ["✓ " + c.resolution]));
      if (!view) {
        var box = commentBox("Reply…", function (text) {
          var op = { op: "reply", id: cid, text: text };
          if (st.author) op.author = st.author;
          applyOps([op]);
          openThread(cid, cx, cy);
        }, "Reply");
        bubble.appendChild(box.el);
        setTimeout(function () { box.ta.focus(); });
      }
      placeBubble(cx, cy);
    }
    document.addEventListener("pointerdown", function (e) { if (!bubble.hidden && !bubble.contains(e.target) && !canvas.contains(e.target)) closeBubble(); });
    /* A popup stays inside the editor, however small it is, and scrolls
       instead: one that opens upwards (`up`, from the bottom bar or the AI
       button) stops below the top bar, one that drops down stops at the
       bottom edge. */
    function fitPopup(pop, up) {
      if (!pop || pop.hidden) return;
      pop.style.maxHeight = "none";
      var r = pop.getBoundingClientRect(), c = canvas.getBoundingClientRect(), e = el.getBoundingClientRect();
      var room = up ? r.bottom - c.top - 8 : e.bottom - r.top - 8;
      pop.style.maxHeight = Math.max(80, Math.floor(room)) + "px";
    }
    window.addEventListener("resize", function () {
      [[blocksMenu, true], [aiMenu, true], [askPanel, true], [boardMenu], [historyMenu]].forEach(function (p) { fitPopup(p[0], p[1]); });
    });
    /* `anchor`: an element (the menu drops below it) or a {x, y} client point. */
    function openMenu(anchor, items) {
      menu.innerHTML = "";
      items.forEach(function (it) {
        if (it === "-") return menu.appendChild(h("hr"));
        menu.appendChild(h("button", { title: it[2] || "", onclick: function () { closeMenu(); it[1](); } }, [it[0]]));
      });
      var r = anchor.getBoundingClientRect ? anchor.getBoundingClientRect() : { left: anchor.x, bottom: anchor.y - 4 }, host = el.getBoundingClientRect();
      menu.hidden = false;
      menu.style.maxHeight = Math.max(80, host.height - 16) + "px";
      menu.style.top = Math.max(8, Math.min(r.bottom - host.top + 4, host.height - menu.offsetHeight - 8)) + "px";
      menu.style.left = Math.max(8, Math.min(r.left - host.left, host.width - menu.offsetWidth - 8)) + "px";
    }
    document.addEventListener("pointerdown", function (e) { if (!menu.hidden && !menu.contains(e.target) && !e.target.closest(".dge-toolbar")) closeMenu(); });
    document.addEventListener("pointerdown", function (e) { if (!aiMenu.hidden && !aiMenu.contains(e.target) && !aiFab.contains(e.target)) closeAiMenu(); });
    document.addEventListener("pointerdown", function (e) {
      if (blocksMenu && !blocksMenu.hidden && !blocksMenu.contains(e.target) && !(blocksBtn && blocksBtn.contains(e.target))) closeBlocks();
      if (boardMenu && !boardMenu.hidden && !boardMenu.contains(e.target) && !e.target.closest("[data-act=board]")) boardMenu.hidden = true;
      if (historyMenu && !historyMenu.hidden && !historyMenu.contains(e.target) && !e.target.closest("[data-act=history]")) historyMenu.hidden = true;
      if (widgetsMenu && !widgetsMenu.contains(e.target) && !e.target.closest("[data-act=widgets]")) closeWidgets();
    });

    function baseName() {
      return slug((st.doc && st.doc.title) || opts.name || "board");
    }
    function openExport(anchor) {
      var items = [
        ["Download SVG", function () { download(baseName() + ".svg", exportSVG(), "image/svg+xml"); }],
        ["Download PNG", function () { exportPNG(2).then(function (b) { download(baseName() + ".png", b); }); }],
        ["Copy PNG to clipboard", function () {
          if (!root.ClipboardItem) return flash("This browser cannot copy images", true);
          navigator.clipboard.write([new ClipboardItem({ "image/png": exportPNG(2) })]).then(function () { flash("PNG copied"); }, function (e) { flash(String(e), true); });
        }],
        ["Copy SVG", function () { copyText(exportSVG()).then(function () { flash("SVG copied"); }); }],
        "-",
        ["Download .shape", function () { download(baseName() + ".shape", st.text); }],
        ["Download JSON", function () { download(baseName() + ".json", st.session.source("json"), "application/json"); }],
        ["Copy as Mermaid", function () { copyText(st.session.source("mermaid")).then(function () { flash("Mermaid copied — paste it into GitHub Markdown"); }); }],
        ["Copy as a prompt to build it", function () { copyBoardPrompt(); }],
        ["Copy LLM brief (Markdown)", function () { copyText(st.session.brief()).then(function () { flash("Brief copied — paste it into a chat as context"); }); }]
      ];
      if (opts.shareUrl) {
        items.push("-");
        items.push(["Copy share link", function () {
          encodeShare(st.text).then(function (hash) { return copyText(opts.shareUrl.replace(/#.*$/, "") + "#" + hash); }).then(function () { flash("Link copied — the diagram is in the URL"); });
        }]);
      }
      openMenu(anchor, items);
    }

    function showModal(title, content, actions) {
      modal.innerHTML = "";
      var box = h("div", { class: "dge-dialog", role: "dialog", "aria-label": title }, [
        h("div", { class: "dge-dialog-head" }, [h("h3", {}, [title]), h("button", { class: "dge-x", title: "Close (Esc)", onclick: closeModal }, ["×"])]),
        content,
        h("div", { class: "dge-actions end" }, actions)
      ]);
      modal.appendChild(box);
      modal.hidden = false;
      box.tabIndex = -1;
      box.focus({ preventScroll: true });
      modal.onclick = function (e) { if (e.target === modal) closeModal(); };
      box.addEventListener("keydown", function (e) { e.stopPropagation(); if (e.key === "Escape") closeModal(); });
    }
    function closeModal() { modal.hidden = true; modal.innerHTML = ""; canvas.focus({ preventScroll: true }); }

    function openPrompt(only) {
      var all = (st.doc && st.doc.comments) || [];
      var state = { target: "diagram", reply: "ops", include_resolved: false, comments: only ? only.slice() : [], instructions: "" };
      var preview = h("textarea", { class: "dge-preview", readonly: true, rows: 14 });
      function update() {
        var o = { target: state.target, reply: state.reply, include_resolved: state.include_resolved, comments: state.comments, instructions: state.instructions || null, file: opts.file || null };
        preview.value = st.session.prompt(JSON.stringify(o));
      }
      function radio(name, value, label, desc) {
        var input = h("input", { type: "radio", name: name, value: value, checked: state[name] === value, onchange: function () { state[name] = value; update(); } });
        return h("label", { class: "dge-radio" }, [input, h("span", {}, [h("b", {}, [label]), desc ? h("small", {}, [desc]) : null])]);
      }
      var checks = h("div", { class: "dge-checklist" });
      all.forEach(function (c, i) {
        var inp = h("input", { type: "checkbox", checked: state.comments.indexOf(c.id) >= 0 || (!state.comments.length && c.status !== "resolved"), onchange: function () {
          var chosen = Array.prototype.slice.call(checks.querySelectorAll("input:checked")).map(function (x) { return x.value; });
          state.comments = chosen;
          update();
        } });
        inp.value = c.id;
        checks.appendChild(h("label", { class: "dge-check" + (c.status === "resolved" ? " resolved" : "") }, [inp, " " + (i + 1) + ". " + c.text.slice(0, 80)]));
      });
      var extra = h("textarea", { rows: 2, placeholder: "Extra instructions (optional), e.g. “keep it to one service”" });
      extra.addEventListener("input", function () { state.instructions = extra.value; update(); });
      var content = h("div", { class: "dge-prompt" }, [
        h("div", { class: "dge-cols" }, [
          h("div", {}, [
            h("h4", {}, ["The agent should…"]),
            radio("target", "diagram", "Update the diagram", "reply with edits you apply here"),
            radio("target", "code", "Change the code", "for Claude Code, Codex & co. in your repo"),
            radio("target", "discuss", "Discuss", "answer questions, change nothing"),
            h("h4", {}, ["Reply format"]),
            radio("reply", "ops", "Operations (JSON)", "precise; applied atomically"),
            radio("reply", "dsl", "Whole diagram", "simplest for weaker models")
          ]),
          h("div", {}, [h("h4", {}, ["Comments"]), all.length ? checks : h("p", { class: "dge-muted" }, ["No comments yet."]), extra])
        ]),
        preview
      ]);
      update();
      showModal("Prompt from comments", content, [
        h("button", { onclick: function () { download(baseName() + "-prompt.md", preview.value, "text/markdown"); } }, ["Download .md"]),
        h("button", { onclick: openApply }, ["Apply reply…"]),
        h("button", { class: "dge-primary", onclick: function () { copyText(preview.value).then(function () { flash("Prompt copied — paste it into your LLM, then use Apply reply"); }); } }, ["Copy prompt"])
      ]);
    }

    function openApply() {
      var ta = h("textarea", { class: "dge-preview", rows: 14, placeholder: "Paste the LLM's reply here: a ```json {\"ops\": [...]} block, or a ```shape block with the whole diagram." });
      var out = h("div", { class: "dge-muted" });
      showModal("Apply an LLM reply", h("div", {}, [ta, out]), [
        h("button", { onclick: closeModal }, ["Cancel"]),
        h("button", { class: "dge-primary", onclick: function () {
          var res = applyReply(ta.value);
          if (res.ok) { closeModal(); flash(res.message); }
          else out.textContent = res.error;
        } }, ["Apply"])
      ]);
      setTimeout(function () { ta.focus(); }, 0);
    }

    /* Apply a chat reply: ops JSON, or a whole diagram in a ```shape block. */
    function applyReply(text) {
      var m = /```(?:shape|diagram|dsl)\s*\n([\s\S]*?)```/.exec(text);
      if (m && !/```json/.test(text)) {
        var test = new wasm.Diagram();
        var r = JSON.parse(test.set_source(m[1]));
        test.free();
        if (!r.ok) return { ok: false, error: r.errors.map(function (e) { return "line " + e.line + ": " + e.message; }).join("\n") };
        load(m[1], true);
        afterChange(true);
        return { ok: true, message: "Diagram replaced from the reply" };
      }
      var before = st.text;
      var out = JSON.parse(st.session.apply(text));
      if (!out.ok) {
        st.session.set_source(before);
        refreshDoc();
        return { ok: false, error: out.error };
      }
      st.undo.push(before);
      st.redo = [];
      refreshDoc();
      st.text = st.session.source("dsl");
      afterChange(true);
      return { ok: true, message: "Applied " + out.count + " changes" };
    }

    // -------------------------------------------------------------- exports
    function exportSVG() {
      return JSON.parse(st.session.render(JSON.stringify({ comments: "none", theme: st.dark ? "dark" : "light", today: isoDay(today()), perspective: st.perspective || null, open: st.perspective ? st.perspOpen : null }))).svg;
    }
    function exportPNG(scale) {
      scale = scale || 2;
      var r = JSON.parse(st.session.render(JSON.stringify({ comments: "none", theme: st.dark ? "dark" : "light", today: isoDay(today()), perspective: st.perspective || null, open: st.perspective ? st.perspOpen : null })));
      return new Promise(function (resolve, reject) {
        var img = new Image();
        var url = URL.createObjectURL(new Blob([r.svg], { type: "image/svg+xml" }));
        img.onload = function () {
          var c = document.createElement("canvas");
          c.width = Math.ceil(r.view.w * scale);
          c.height = Math.ceil(r.view.h * scale);
          var ctx = c.getContext("2d");
          ctx.scale(scale, scale);
          ctx.drawImage(img, 0, 0, r.view.w, r.view.h);
          URL.revokeObjectURL(url);
          c.toBlob(function (b) { b ? resolve(b) : reject(new Error("PNG export failed")); }, "image/png");
        };
        img.onerror = function () { URL.revokeObjectURL(url); reject(new Error("PNG export failed")); };
        img.src = url;
      });
    }

    /* The branch chip: in git, which branch this is and how much changed on
       it (since it left main, or since the last commit on main). A click
       shows those changes on the board. */
    var since = null, branchTimer = null;
    function refreshBranch() {
      if (!opts.since || !toolbar.branch) return;
      Promise.resolve(opts.since()).then(function (v) {
        since = v && v.source != null ? v : null;
        paintBranch();
      }).catch(function () { since = null; paintBranch(); });
    }
    function paintBranch() {
      var b = toolbar.branch;
      if (!b) return;
      if (!since) { b.hidden = true; return; }
      var n = wasm.count_changes ? wasm.count_changes(since.source, st.text) : -1;
      var onMain = !since.branch || since.branch === since.main;
      var what = n < 0 ? "" : n === 0 ? (onMain ? "no changes since the last commit" : "no changes since " + since.main)
        : n + (n === 1 ? " change" : " changes") + (onMain ? " not committed" : " since " + since.main);
      b.innerHTML = icon("branch") + "<span></span>";
      b.lastChild.textContent = (since.branch || since.main || "git") + (what ? " · " + what : "");
      b.title = n > 0 ? "Show what changed since " + since.label + " (click again to stop)" : "In git: " + since.label;
      b.classList.toggle("on", st.base != null && st.baseLabel === since.label);
      b.hidden = false;
    }
    function toggleBranchChanges() {
      if (!since) return;
      if (st.base != null && st.baseLabel === since.label) setBase(null);
      else setBase(since.source, since.label);
      paintBranch();
    }
    function setBase(src, label) {
      st.base = src;
      st.baseLabel = label;
      var r = JSON.parse(st.session.set_base(src == null ? undefined : src));
      if (!r.ok) flash("Base version: " + (r.errors || []).map(function (e) { return e.message; }).join("; "), true);
      toolbar.baseChip.hidden = src == null;
      toolbar.baseChip.innerHTML = "";
      if (src != null) {
        toolbar.baseChip.appendChild(document.createTextNode(since && label === since.label ? "Changes shown " : "Changes since " + (label || "base") + " "));
        toolbar.baseChip.appendChild(h("button", { title: "Copy the change summary", onclick: function () { copyText(st.session.diff_markdown()).then(function () { flash("Change summary copied"); }); } }, ["copy"]));
        toolbar.baseChip.appendChild(h("button", { title: "Stop comparing", onclick: function () { setBase(null); } }, ["×"]));
      }
      render();
    }

    // ------------------------------------------------------------------ api
    var resizeObs = root.ResizeObserver ? new ResizeObserver(function () { if (!st.fitted && st.r) { fit(); st.fitted = true; } }) : null;
    if (resizeObs) resizeObs.observe(canvas);

    var api = {
      getSource: function (fmt) {
        // the board, also while a temporary answer is open
        var m = st.answer ? st.answer.main : null;
        if (m) return !fmt || fmt === "dsl" ? m.text : m.session.source(fmt);
        return !fmt || fmt === "dsl" ? st.text : st.session.source(fmt);
      },
      setSource: function (src, o) {
        if (st.answer) { st.answer.main.session.set_source(src); st.answer.main.text = src; return; }
        load(src, !(o && o.silent === true) && !!st.doc);
        syncCode(true);
        showErrors();
        render();
        if (o && o.fit) fit();
      },
      apply: function (ops) { return applyReply(typeof ops === "string" ? ops : JSON.stringify(Array.isArray(ops) ? { ops: ops } : ops)); },
      getDocument: function () { return JSON.parse(st.session.document()); },
      errors: function () { return st.errors.slice(); },
      setBase: setBase,
      author: function () { return st.author || ""; },
      prompt: function (o) { return st.session.prompt(JSON.stringify(o || {})); },
      brief: function () { return st.session.brief(); },
      exportSVG: exportSVG,
      exportPNG: exportPNG,
      select: function (ids) { select(ids || []); },
      goTo: function (id) { followLink("#" + id); },
      resolveConflict: resolveConflict,
      openCanvas: function (path) { return openCanvas(path || []); },
      canvasPath: function () { return (st.canvasPath || []).slice(); },
      fit: fit,
      setTheme: function (t) { st.dark = t === "dark"; applyChrome(); render(); },
      setMode: function (m) { view = m === "view"; el.classList.toggle("dge-view", view); buildToolbar(); applyChrome(); render(); },
      addButton: function (b) {
        var button = h("button", { title: b.title || "", onclick: function (e) { b.onClick(e, api); } }, [b.label]);
        (st.extraButtons = st.extraButtons || []).push(button);
        toolbar.extra.appendChild(button);
        return button;
      },
      flash: flash,
      on: function (name, fn) { (listeners[name] = listeners[name] || []).push(fn); return api; },
      destroy: function () {
        document.removeEventListener("keydown", onKey);
        document.removeEventListener("keyup", onKeyUp);
        document.removeEventListener("paste", onPaste);
        if (resizeObs) resizeObs.disconnect();
        st.session.free();
        el.innerHTML = "";
        el.classList.remove("dge", "dge-view");
      },
      session: st.session
    };

    /* Tooltips for the editor's buttons: sooner and clearer than the
       browser's. A control's `title` becomes its tooltip: the name, its
       shortcut as a key, then what it does. */
    var tip = h("div", { class: "dge-tip", role: "tooltip", hidden: true });
    el.appendChild(tip);
    var tipTimer = null, tipShownAt = 0, tipFor = null;
    function tipParts(text) {
      var m = /^([^:(]+?)\s*(?:\(([^)]{1,12})\))?\s*(?::\s*([\s\S]*))?$/.exec(text);
      var more = m && m[3] ? m[3].charAt(0).toUpperCase() + m[3].slice(1) : "";
      return m ? { name: m[1], key: m[2] || "", more: more } : { name: text, key: "", more: "" };
    }
    function showTip(target) {
      var text = target.getAttribute("data-tip");
      if (!text) return;
      tip.classList.remove("dge-card-tip");
      var t = tipParts(text);
      tip.innerHTML = "";
      var head = h("div", { class: "dge-tip-head" }, [t.name]);
      if (t.key) head.appendChild(h("kbd", {}, [t.key]));
      tip.appendChild(head);
      if (t.more) tip.appendChild(h("div", { class: "dge-tip-more" }, [t.more]));
      tip.hidden = false;
      var box = el.getBoundingClientRect(), r = target.getBoundingClientRect();
      var w = tip.offsetWidth, ht = tip.offsetHeight;
      var x = Math.max(8, Math.min(r.left + r.width / 2 - w / 2 - box.left, box.width - w - 8));
      // above what sits in the lower half, below what sits in the upper half
      var below = r.top + r.height / 2 < box.top + box.height / 2;
      var y = below ? r.bottom - box.top + 8 : r.top - box.top - ht - 8;
      tip.style.left = x + "px";
      tip.style.top = y + "px";
      tipShownAt = Date.now();
    }
    function hideTip() {
      clearTimeout(tipTimer);
      // the title goes back while the pointer is away (selectors, other tools)
      if (tipFor && tipFor.hasAttribute("data-tip")) { tipFor.setAttribute("title", tipFor.getAttribute("data-tip")); tipFor.removeAttribute("data-tip"); }
      tipFor = null;
      if (!tip.hidden) { tip.hidden = true; tipShownAt = Date.now(); }
    }
    el.addEventListener("mouseover", function (e) {
      var t = e.target.closest && e.target.closest("button[title], select[title], label[title], button[data-tip], select[data-tip], label[data-tip]");
      if (!t || !el.contains(t) || t === tipFor) return;
      hideTip();
      // the browser's own tooltip would show too: the title waits in data-tip
      if (t.hasAttribute("title")) { t.setAttribute("data-tip", t.getAttribute("title")); t.removeAttribute("title"); }
      tipFor = t;
      // right after another tooltip, the next one shows at once
      tipTimer = setTimeout(function () { if (tipFor === t) showTip(t); }, Date.now() - tipShownAt < 600 ? 0 : 350);
    });
    el.addEventListener("mouseout", function (e) {
      if (tipFor && !(e.relatedTarget && tipFor.contains(e.relatedTarget))) hideTip();
    });
    /* A comment pin's preview: who wrote it, how it starts, its replies. */
    canvas.addEventListener("mouseover", function (e) {
      var pin = e.target.closest && e.target.closest(".dg-pin");
      if (!pin || pin === tipFor || st.drag) return;
      var c = (st.doc.comments || []).find(function (x) { return x.id === pin.getAttribute("data-comment"); });
      if (!c) return;
      hideTip();
      tipFor = pin;
      tipTimer = setTimeout(function () {
        if (tipFor !== pin || !bubble.hidden) return;
        tip.innerHTML = "";
        tip.classList.remove("dge-card-tip");
        var head = h("div", { class: "dge-tip-head" }, [(c.author || "Comment") + (c.status === "resolved" ? " · resolved" : "")]);
        if (repliesText(c)) head.appendChild(h("kbd", {}, [repliesText(c)]));
        tip.appendChild(head);
        tip.appendChild(h("div", { class: "dge-tip-more" }, [c.text.length > 160 ? c.text.slice(0, 157) + "…" : c.text]));
        tip.hidden = false;
        var box = el.getBoundingClientRect(), r = pin.getBoundingClientRect();
        tip.style.left = Math.max(8, Math.min(r.left - box.left, box.width - tip.offsetWidth - 8)) + "px";
        tip.style.top = (r.top - box.top - tip.offsetHeight - 8 < 8 ? r.bottom - box.top + 8 : r.top - box.top - tip.offsetHeight - 8) + "px";
        tipShownAt = Date.now();
      }, Date.now() - tipShownAt < 600 ? 0 : 250);
    });
    /* A box's hover card: its whole title, its detail (a kanban card keeps it
       off the card, marked ≡) and what else is known: owner, dates, tags,
       link, open comments. Nothing to add beyond the title: no card. */
    function nodeFacts(n, summary) {
      var meta = [];
      if (n.owner) meta.push("owner: " + n.owner);
      if (n.date) meta.push(n.date + (n.important ? " ★" : ""));
      if (n.start) meta.push("from " + n.start + (n.days ? ", " + n.days + " days" : ""));
      if (n.probability && n.impact) meta.push("probability " + n.probability + ", impact " + n.impact);
      if (n.done) meta.push(n.done + "% done");
      (n.tags || []).forEach(function (t) { meta.push("#" + t); });
      if (n.link) meta.push("→ " + n.link.replace(/^https?:\/\//, ""));
      var open = (st.doc.comments || []).filter(function (c) { return c.on === n.id && c.status !== "resolved"; }).length;
      if (open) meta.push(open + (open === 1 ? " open comment" : " open comments"));
      var body = n.detail || (summary && summary !== n.label && summary !== n.id ? summary : "");
      return { title: n.label || n.id, body: body, meta: meta };
    }
    canvas.addEventListener("mouseover", function (e) {
      var g = e.target.closest && e.target.closest(".dg-node");
      if (!g || g === tipFor || st.drag || e.target.closest(".dg-pin, .dg-link, .dg-open, .dge-ctl")) return;
      var n = node(g.getAttribute("data-id"));
      if (!n) return;
      // the drawing's own hover text (a table row's cells…) goes in the card instead
      var t = g.querySelector(":scope > title");
      if (t) { g.setAttribute("data-summary", t.textContent); t.remove(); }
      var f = nodeFacts(n, g.getAttribute("data-summary"));
      if (!f.body && !f.meta.length) return;
      hideTip();
      tipFor = g;
      tipTimer = setTimeout(function () {
        if (tipFor !== g || !bubble.hidden || !menu.hidden || st.drag) return;
        tip.innerHTML = "";
        tip.classList.add("dge-card-tip");
        tip.appendChild(h("div", { class: "dge-tip-head" }, [f.title]));
        if (f.body) tip.appendChild(h("div", { class: "dge-tip-body" }, [f.body]));
        if (f.meta.length) tip.appendChild(h("div", { class: "dge-tip-more" }, [f.meta.join(" · ")]));
        tip.hidden = false;
        var box = el.getBoundingClientRect(), r = g.getBoundingClientRect();
        tip.style.left = Math.max(8, Math.min(r.left - box.left, box.width - tip.offsetWidth - 8)) + "px";
        tip.style.top = (r.bottom - box.top + 8 + tip.offsetHeight > box.height - 8 ? r.top - box.top - tip.offsetHeight - 8 : r.bottom - box.top + 8) + "px";
        tipShownAt = Date.now();
      }, Date.now() - tipShownAt < 600 ? 0 : 450);
    });
    el.addEventListener("pointerdown", hideTip, true);
    el.addEventListener("wheel", hideTip, { passive: true });

    buildToolbar();
    buildTabs();
    refreshBranch();
    // a commit made elsewhere moves the branch's base: look again on return
    if (opts.since) root.addEventListener("focus", refreshBranch);
    applyChrome();
    load(opts.source || "", false);
    if (opts.base != null) {
      st.session.set_base(opts.base);
      st.base = opts.base;
    }
    syncCode(true);
    showErrors();
    render();
    if (opts.base != null) setBase(opts.base, opts.baseLabel);
    restorePerspective();
    if (st.perspective) render();
    // who you are (when git does not know), then whose eyes to look through
    setTimeout(function () {
      var then = function () { if (opts.askPerspective) askPerspective(); };
      if (opts.requireName && !st.author && !view) askName(then); else then();
    }, 300);
    // a team's recipes reach the AI's prompts and Insert recipe…
    // a schema block whose file changed offers to update
    if (opts.sources) setTimeout(checkSchemas, 1200);
    if (opts.recipes) Promise.resolve(opts.recipes()).then(function (files) { if (files && st.session.set_recipes) st.session.set_recipes(JSON.stringify(files)); }).catch(function () {});
    return api;
  }

  root.DiagramEditor = { create: create, encodeShare: encodeShare, decodeShare: decodeShare, slug: slug };
})(typeof window !== "undefined" ? window : this);
