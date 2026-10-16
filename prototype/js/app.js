/* Khung ứng dụng: điều hướng, shell, phím tắt, command palette, Task Center, thanh prototype. */
(function () {
  const D = window.DATA;
  const list = [];
  window.PAGES = { register: (p) => list.push(p), get: (id) => list.find((p) => p.id === id), list };
  const ACTIONS = {};
  window.ACTIONS = ACTIONS;

  const store = {
    get: (k, d) => { try { return JSON.parse(localStorage.getItem("cs." + k)) ?? d; } catch { return d; } },
    set: (k, v) => localStorage.setItem("cs." + k, JSON.stringify(v)),
  };

  const APP = (window.APP = {
    page: store.get("page", "dashboard"),
    state: "normal",
    theme: store.get("theme", "light"),
    sidebar: "open",
    inspector: "open",
    go(id, opts = {}) {
      if (!PAGES.get(id)) return;
      APP.page = id;
      store.set("page", id);
      if (location.hash !== "#/" + id) history.replaceState(null, "", "#/" + id);
      render(opts);
    },
    rerender: () => render({ keepScroll: true }),
  });

  const ctx = () => ({ st: APP.state, D, go: APP.go, rerender: APP.rerender });
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => [...r.querySelectorAll(s)];

  /* ---------- Điều hướng chính (IA trong product plan) ---------- */
  const NAV = [
    { id: "dashboard", label: "Tổng quan", icon: "dashboard", key: "1" },
    { id: "research", label: "Nghiên cứu", icon: "research", key: "2" },
    { id: "library", label: "Thư viện", icon: "library", key: "3", dot: true, dotTip: "2 tệp chờ duyệt quyền" },
    { id: "brand", label: "Thương hiệu", icon: "brand", key: "4" },
    "-",
    { id: "writing", label: "Soạn thảo", icon: "writing", key: "5" },
    { id: "design", label: "Thiết kế", icon: "design", key: "6" },
    { id: "motion", label: "Chuyển động", icon: "motion", key: "7" },
    "-",
    { id: "automation", label: "Tự động hóa & AI", icon: "automation", key: "8" },
    { id: "publishing", label: "Bàn giao", icon: "publishing", key: "9", dot: true, dotTip: "1 gói cần đối soát" },
  ];
  const TABBAR = ["dashboard", "library", "writing", "design", "publishing"];

  function shell() {
    document.getElementById("root").innerHTML = `
    <div class="viewport" id="viewport" data-width="${store.get("width", "fill")}"><div class="frame">
      <div class="app" id="app">
        <header class="titlebar">
          <div class="brand-mark"><span class="logo" aria-hidden="true">LS</span><span class="name">Local Content Studio</span></div>
          <button class="btn quiet icon show-sm" data-action="toggle-sidebar" aria-label="Mở danh sách" data-tip="Danh sách" >${I("menu")}</button>
          <div class="vdivider hide-sm"></div>
          <button class="ws-switch" id="ws-switch" aria-haspopup="menu" data-tip="Đổi workspace" data-tip-sub="Mỗi workspace có dữ liệu và quyền riêng">${I("folder")}<span class="ws-name truncate">${D.workspace.name}</span>${I("chev-down")}</button>
          <nav class="crumbs" id="crumbs" aria-label="Vị trí"></nav>
          <button class="cmd-trigger" data-action="palette" aria-label="Tìm hoặc chạy lệnh" data-tip="Tìm dự án, tài nguyên, lệnh" data-kbd="Ctrl+K">${I("search")}<span class="label">Tìm hoặc chạy lệnh…</span>${UI.kbd("Ctrl", "K")}</button>
          <button class="save-state" id="save-state" aria-haspopup="menu"></button>
          <div class="tb-actions">
            <button class="btn quiet icon hide-sm" data-action="toggle-sidebar" id="tb-sidebar" aria-label="Ẩn/hiện thanh bên" data-tip="Ẩn/hiện thanh bên" data-kbd="Ctrl+B">${I("panel-left")}</button>
            <button class="btn quiet icon" data-action="toggle-inspector" id="tb-inspector" aria-label="Ẩn/hiện bảng thuộc tính" data-tip="Ẩn/hiện bảng thuộc tính" data-kbd="Ctrl+.">${I("panel-right")}</button>
            <button class="btn quiet icon has-count" data-action="tasks" aria-label="Task Center, 2 đang chạy, 1 lỗi" data-tip="Task Center" data-tip-sub="2 đang chạy · 1 lỗi" data-kbd="Ctrl+J">${I("activity")}<span class="count-dot">3</span></button>
            <button class="btn quiet icon" data-action="toggle-theme" id="tb-theme" aria-label="Đổi giao diện sáng/tối" data-tip="Đổi giao diện sáng/tối"></button>
            <button class="btn quiet icon" id="tb-help" aria-haspopup="menu" aria-label="Trợ giúp" data-tip="Trợ giúp và phím tắt" data-kbd="?">${I("help")}</button>
          </div>
        </header>

        <div class="workbench">
          <nav class="rail" aria-label="Khu vực làm việc" id="rail"></nav>
          <aside class="sidebar" id="sidebar" aria-label="Thanh bên"></aside>
          <main class="main" id="main" tabindex="-1"></main>
          <aside class="inspector" id="inspector" aria-label="Bảng thuộc tính"></aside>
          <div class="drawer-scrim for-sidebar" data-action="close-drawers"></div>
          <div class="drawer-scrim for-inspector" data-action="close-drawers"></div>
          <section class="taskcenter" id="taskcenter" hidden aria-label="Task Center"></section>
        </div>

        <footer class="statusbar" id="statusbar"></footer>
        <nav class="tabbar" id="tabbar" aria-label="Điều hướng nhanh"></nav>
      </div>
    </div></div>
    <div class="proto-bar" id="proto-bar" role="toolbar" aria-label="Điều khiển prototype"></div>
    <div class="sr-only" aria-live="polite" id="announcer"></div>`;

    $("#rail").innerHTML =
      NAV.map((n) => n === "-" ? `<div class="rail-sep"></div>` :
        `<button class="rail-btn" data-go="${n.id}" aria-label="${n.label}" data-tip="${n.label}" ${n.dotTip ? `data-tip-sub="${n.dotTip}"` : ""} data-tip-pos="right" data-kbd="Ctrl+${n.key}">${I(n.icon)}${n.dot ? `<span class="badge-dot" aria-hidden="true"></span>` : ""}</button>`).join("") +
      `<div class="rail-bottom"><div class="rail-sep"></div>
        <button class="rail-btn" data-go="settings" aria-label="Cài đặt" data-tip="Cài đặt" data-tip-pos="right" data-kbd="Ctrl+,">${I("settings")}</button></div>`;

    $("#tabbar").innerHTML = TABBAR.map((id) => {
      const n = NAV.find((x) => x.id === id);
      return `<button data-go="${id}">${I(n.icon)}<span>${n.label}</span></button>`;
    }).join("") ;

    $("#statusbar").innerHTML = `
      <button class="sb-item" data-go="settings" data-settings="storage" data-tip="Phạm vi dữ liệu hiện tại" data-tip-sub="Mọi thao tác chỉ đọc/ghi trong workspace này" data-tip-pos="top">${I("shield")}Workspace: ${D.workspace.name}</button>
      <button class="sb-item" data-go="settings" data-settings="storage" data-tip="Catalog SQLite (WAL) · schema v2" data-tip-sub="Kiểm toàn vẹn lúc 08:00 · không lỗi" data-tip-pos="top"><span class="dot success"></span>Catalog ổn định</button>
      <button class="sb-item hide-md" data-go="settings" data-settings="storage" data-tip="Bản sao lưu gần nhất" data-tip-sub="214 blob · đã kiểm hash" data-tip-pos="top">${I("history")}Sao lưu 08:00</button>
      <span class="spacer"></span>
      <button class="sb-item" data-action="tasks" data-tip="Mở Task Center" data-kbd="Ctrl+J" data-tip-pos="top">${I("loader")}2 đang chạy<span class="progress"><span style="width:67%"></span></span><span style="color:var(--cs-danger)">· 1 lỗi</span></button>
      <button class="sb-item" id="sb-ai" data-go="settings" data-settings="ai" data-tip-pos="top"></button>
      <button class="sb-item" id="sb-social" data-go="settings" data-settings="social" data-tip-pos="top"></button>
      <span class="sb-item hide-md" style="cursor:default">v0.1.0</span>`;

    protoBar();
    applyTheme();
  }

  /* ---------- Render trang ---------- */
  function render(opts = {}) {
    UI.hideTip();
    const p = PAGES.get(APP.page) || PAGES.get("dashboard");
    const c = ctx();
    const app = $("#app");
    const sb = p.sidebar ? p.sidebar(c) : null;
    const ins = p.inspector ? p.inspector(c) : null;

    $("#sidebar").innerHTML = sb ? `
      <div class="sidebar-head"><h2>${sb.title}</h2>${sb.actions || ""}</div>
      <div class="sidebar-body">${sb.body}</div>
      ${sb.foot ? `<div class="sidebar-foot">${sb.foot}</div>` : ""}
      <div class="resize-handle" data-resize="sidebar" role="separator" aria-orientation="vertical" aria-label="Kéo để đổi bề rộng thanh bên" tabindex="0"></div>` : "";
    app.dataset.sidebar = sb ? APP.sidebar : "none";
    $("#tb-sidebar").disabled = !sb;

    $("#inspector").innerHTML = ins ? `
      <div class="sidebar-head"><h2>${ins.title}</h2>${ins.actions || ""}<button class="btn quiet sm icon" data-action="toggle-inspector" aria-label="Đóng bảng thuộc tính" data-tip="Đóng bảng thuộc tính" data-kbd="Ctrl+.">${I("x")}</button></div>
      <div class="sidebar-body">${ins.body}</div>
      ${ins.foot ? `<div class="sidebar-foot">${ins.foot}</div>` : ""}
      <div class="resize-handle" data-resize="inspector" role="separator" aria-orientation="vertical" aria-label="Kéo để đổi bề rộng bảng thuộc tính" tabindex="0"></div>` : "";
    app.dataset.inspector = ins ? APP.inspector : "none";
    $("#tb-inspector").style.display = ins ? "" : "none";
    delete app.dataset.sbDrawer; delete app.dataset.inspDrawer;

    const main = $("#main");
    const scroll = opts.keepScroll ? main.querySelector(".page-body")?.scrollTop : 0;
    main.innerHTML = p.main(c);
    if (scroll) { const pb = main.querySelector(".page-body"); if (pb) pb.scrollTop = scroll; }
    p.mount && p.mount(app, c);

    $$("[data-go]", $("#rail")).forEach((b) => b.dataset.go === p.id ? b.setAttribute("aria-current", "page") : b.removeAttribute("aria-current"));
    $$("[data-go]", $("#tabbar")).forEach((b) => b.dataset.go === p.id ? b.setAttribute("aria-current", "page") : b.removeAttribute("aria-current"));

    const inProject = ["writing", "design", "motion", "publishing"].includes(p.id);
    $("#crumbs").innerHTML = `<span class="crumb-ws">${D.workspace.name}</span><span class="sep crumb-ws">/</span>` +
      (inProject ? `<button data-go="dashboard" data-tip="Mở danh sách dự án">${D.project.name}</button><span class="sep">/</span>` : "") +
      `<span class="current">${p.label}</span>`;
    const ss = $("#save-state");
    if (inProject) {
      ss.style.display = "";
      ss.innerHTML = APP.state === "error" ? `<span class="dot danger"></span><span class="label">Không lưu được</span>` : `<span class="dot success"></span><span class="label">Đã lưu ${D.project.rev} · ${D.project.savedAt}</span>`;
      ss.setAttribute("data-tip", "Revision đã lưu gần nhất");
      ss.setAttribute("data-tip-sub", "Nhấn để xem lịch sử revision · Ctrl+S để lưu revision mới");
    } else ss.style.display = "none";

    const off = APP.state === "blocked";
    const sbAi = $("#sb-ai"), sbSo = $("#sb-social");
    sbAi.innerHTML = off ? `${I("sparkles")}Không dùng AI` : `${I("sparkles")}AI: gpt-4.1-mini`;
    sbAi.dataset.tip = off ? "Chế độ không dùng AI" : "AI qua API · hỏi trước khi gửi dữ liệu";
    sbAi.dataset.tipSub = off ? "Mọi chức năng thủ công vẫn dùng được" : "Đã dùng 0,28 / 5,00 USD tháng này";
    sbSo.innerHTML = off ? `${I("wifi-off")}Social: chưa kết nối` : `<span class="dot success"></span>Social: đã kết nối`;
    sbSo.dataset.tip = off ? "Không liên lạc được Social Harness" : "Social Harness · localhost:3099";
    sbSo.dataset.tipSub = off ? "Bàn giao bị chặn; vẫn làm việc cục bộ được" : "Cập nhật trạng thái lúc 23:12";

    document.title = `${p.label} · Local Content Studio`;
    if (opts.focus) { const h = main.querySelector("h1"); if (h) { h.tabIndex = -1; h.focus(); } }
    $("#announcer").textContent = `Đã mở ${p.label}`;
  }

  /* ---------- Theme ---------- */
  function applyTheme() {
    document.documentElement.dataset.theme = APP.theme;
    const b = $("#tb-theme");
    if (b) b.innerHTML = I(APP.theme === "dark" ? "sun" : "moon");
    $$("#proto-bar [data-theme-btn]").forEach((x) => x.setAttribute("aria-pressed", x.dataset.themeBtn === APP.theme));
  }
  ACTIONS["toggle-theme"] = () => { APP.theme = APP.theme === "dark" ? "light" : "dark"; store.set("theme", APP.theme); applyTheme(); };

  /* ---------- Thanh bên / bảng thuộc tính / drawer ---------- */
  const vpWidth = () => $("#viewport").getBoundingClientRect().width;
  const appWidth = () => $("#app").getBoundingClientRect().width;
  ACTIONS["toggle-sidebar"] = () => {
    const app = $("#app");
    if (app.dataset.sidebar === "none") return;
    if (appWidth() <= 860) { app.dataset.sbDrawer = app.dataset.sbDrawer === "open" ? "" : "open"; if (app.dataset.sbDrawer) $("#sidebar button, #sidebar input")?.focus(); return; }
    APP.sidebar = APP.sidebar === "open" ? "closed" : "open";
    app.dataset.sidebar = APP.sidebar;
  };
  ACTIONS["toggle-inspector"] = () => {
    const app = $("#app");
    if (app.dataset.inspector === "none") return;
    if (appWidth() <= 1100) { app.dataset.inspDrawer = app.dataset.inspDrawer === "open" ? "" : "open"; return; }
    APP.inspector = APP.inspector === "open" ? "closed" : "open";
    app.dataset.inspector = APP.inspector;
  };
  ACTIONS["close-drawers"] = () => { const a = $("#app"); delete a.dataset.sbDrawer; delete a.dataset.inspDrawer; };

  /* Kéo đổi bề rộng panel (chuột) + phím mũi tên trên tay nắm */
  document.addEventListener("pointerdown", (e) => {
    const h = e.target.closest("[data-resize]");
    if (!h) return;
    const which = h.dataset.resize;
    const panel = $("#" + which);
    const startX = e.clientX, startW = panel.getBoundingClientRect().width;
    h.classList.add("dragging");
    const move = (ev) => {
      const dx = which === "sidebar" ? ev.clientX - startX : startX - ev.clientX;
      const w = Math.max(which === "sidebar" ? 220 : 260, Math.min(460, startW + dx));
      document.documentElement.style.setProperty(which === "sidebar" ? "--cs-sidebar-w" : "--cs-inspector-w", w + "px");
    };
    const up = () => { h.classList.remove("dragging"); removeEventListener("pointermove", move); removeEventListener("pointerup", up); };
    addEventListener("pointermove", move); addEventListener("pointerup", up);
  });
  document.addEventListener("keydown", (e) => {
    const h = e.target.closest?.("[data-resize]");
    if (!h || !["ArrowLeft", "ArrowRight"].includes(e.key)) return;
    const which = h.dataset.resize, v = which === "sidebar" ? "--cs-sidebar-w" : "--cs-inspector-w";
    const cur = parseInt(getComputedStyle(document.documentElement).getPropertyValue(v));
    const d = (e.key === "ArrowRight" ? 16 : -16) * (which === "sidebar" ? 1 : -1);
    document.documentElement.style.setProperty(v, Math.max(220, Math.min(460, cur + d)) + "px");
  });

  /* ---------- Task Center ---------- */
  ACTIONS.tasks = () => {
    const tc = $("#taskcenter");
    if (!tc.hidden) { tc.hidden = true; return; }
    tc.hidden = false;
    renderTasks();
    tc.querySelector("button")?.focus();
  };
  function jobRow(j) {
    const icon = { RUNNING: "loader", VERIFYING: "shield", FAILED: "x-circle", DONE: "check-circle", CANCELLED: "x", PENDING: "clock" }[j.state];
    const color = { RUNNING: "var(--cs-accent)", VERIFYING: "var(--cs-accent)", FAILED: "var(--cs-danger)", DONE: "var(--cs-success)" }[j.state] || "var(--cs-fg-secondary)";
    const actions = {
      RUNNING: `<button class="btn sm" data-action="job-cancel" data-tip="Hủy tiến trình" data-tip-sub="Đầu ra dở dang sẽ bị xóa; snapshot giữ nguyên">Hủy</button>`,
      VERIFYING: `<button class="btn sm" disabled data-tip="Không thể hủy khi đang kiểm đầu ra">Hủy</button>`,
      FAILED: `<button class="btn sm" data-action="job-error" data-tip="Xem log lỗi và gợi ý khắc phục">Xem lỗi</button><button class="btn sm primary" data-action="job-retry" data-tip="Chạy lại với cùng snapshot revision">Thử lại</button>`,
      DONE: `<button class="btn sm quiet" data-tip="Mở thư mục đầu ra">${I("folder-open")}</button>`,
      CANCELLED: `<button class="btn sm" data-action="job-retry" data-tip="Chạy lại với cùng snapshot">Chạy lại</button>`,
    }[j.state] || "";
    return `<div class="job-row" ${j.state === "FAILED" ? 'role="alert"' : ""}>
      <span style="color:${color}">${I(icon)}</span>
      <div class="col" style="gap:0;min-width:0"><span class="strong truncate">${j.type}</span><span class="xs muted truncate">${j.target} · bắt đầu ${j.started}</span></div>
      <div class="job-stage xs ${j.state === "FAILED" ? "" : "muted"}" style="${j.state === "FAILED" ? "color:var(--cs-danger)" : ""}">${j.error || j.stage}</div>
      <div class="job-progress row">${j.state === "DONE" || j.state === "CANCELLED" ? UI.badge(j.state) : `<div style="flex:1">${UI.progress(j.pct, j.type)}</div><span class="xs mono" style="width:34px;text-align:right">${j.pct == null ? "" : j.pct + "%"}</span>`}</div>
      <div class="row" style="justify-content:flex-end">${actions}</div>
    </div>`;
  }
  function renderTasks() {
    const J = D.jobs;
    const act = J.filter((j) => ["RUNNING", "VERIFYING", "PENDING"].includes(j.state));
    const fail = J.filter((j) => j.state === "FAILED");
    const done = J.filter((j) => ["DONE", "CANCELLED"].includes(j.state));
    $("#taskcenter").innerHTML = `
      <div class="panel-head" style="padding-left:var(--cs-space-3)">
        ${UI.tabs("tc", [{ label: "Đang chạy", count: act.length }, { label: "Lỗi", count: fail.length }, { label: "Đã xong", count: done.length }], "sm")}
        <span class="spacer"></span>
        <span class="xs muted hide-sm">Worker: 1/2 luồng · CPU 38%</span>
        <button class="btn quiet sm" data-tip="Xóa các mục đã xong khỏi danh sách" data-tip-sub="Không xóa tệp đầu ra">Dọn mục đã xong</button>
        <button class="btn quiet sm icon" data-action="tasks" aria-label="Đóng Task Center" data-tip="Đóng" data-kbd="Ctrl+J">${I("chev-down")}</button>
      </div>
      <div style="overflow:auto;flex:1" aria-live="polite">
        ${UI.tabPanel("tc", 0, act.map(jobRow).join("") || UI.empty({ icon: "activity", title: "Không có tiến trình nào đang chạy" }))}
        ${UI.tabPanel("tc", 1, fail.map(jobRow).join(""))}
        ${UI.tabPanel("tc", 2, done.map(jobRow).join(""))}
      </div>`;
  }
  ACTIONS["job-cancel"] = () => UI.confirm({ title: "Hủy tiến trình xuất PNG?", body: `<p>Trang 1–3 đã render sẽ bị xóa khỏi thư mục tạm. Revision <strong>rev-4</strong> không bị ảnh hưởng và có thể xuất lại bất cứ lúc nào.</p>`, confirmLabel: "Hủy tiến trình", destructive: true, onConfirm: () => UI.toast({ type: "info", title: "Đã hủy tiến trình xuất PNG" }) });
  ACTIONS["job-retry"] = () => UI.toast({ type: "info", title: "Đã xếp hàng chạy lại", desc: "Dùng đúng snapshot rev-1 · tài nguyên và phông đã ghim." });
  ACTIONS["job-error"] = () => UI.dialog({
    title: "Render MP4 thất bại",
    size: "lg",
    body: `${UI.callout("danger", "alert-circle", "Thiếu phông chữ “Be Vietnam Pro” cho phụ đề", "FFmpeg thoát với mã 1 ở cảnh 3/5. Snapshot rev-1 giữ nguyên; không có tệp đầu ra lỗi nào được lưu.")}
      <div class="col"><span class="label">Cách khắc phục</span>
        <div class="list">
          <button class="list-item">${I("download")}<span>Cài phông “Be Vietnam Pro” rồi chạy lại</span></button>
          <button class="list-item">${I("type")}<span>Đổi phụ đề sang phông có sẵn (Inter) trong Chuyển động</span></button>
          <button class="list-item">${I("image")}<span>Xuất storyboard PNG thay cho video</span></button>
        </div></div>
      <details class="insp-section" style="border:1px solid var(--cs-border-subtle)"><summary>${I("chev-right", "chev")}Log kỹ thuật</summary>
        <pre class="mono xs" style="margin:0;padding:12px;background:var(--cs-bg-raised);overflow:auto;max-height:180px">[22:41:09] render.start job-39 spec=sha256:4be1…
[22:41:31] scene 3/5 subtitles: font "Be Vietnam Pro" not found
[22:41:31] ffmpeg exited with code 1
correlation_id=c-7f21a9</pre></details>`,
    footer: `<span class="foot-note">Mã tương quan: <span class="copyable mono" data-copy="c-7f21a9">c-7f21a9</span></span><button class="btn" data-close>Đóng</button><button class="btn primary" data-close data-action="job-retry">Thử lại</button>`,
  });

  /* ---------- Command palette ---------- */
  function commands() {
    const nav = NAV.filter((n) => n !== "-").map((n) => ({ group: "Đi tới", icon: n.icon, text: n.label, kbd: "Ctrl+" + n.key, run: () => APP.go(n.id, { focus: true }) }));
    nav.push({ group: "Đi tới", icon: "settings", text: "Cài đặt", kbd: "Ctrl+,", run: () => APP.go("settings", { focus: true }) });
    const acts = [
      { icon: "file-plus", text: "Tạo dự án mới…", kbd: "Ctrl+N", run: () => ACTIONS["new-project"]() },
      { icon: "upload", text: "Nhập tệp vào thư viện…", run: () => ACTIONS.upload() },
      { icon: "globe", text: "Thu thập nội dung từ URL…", run: () => APP.go("research") },
      { icon: "commit", text: "Lưu revision", kbd: "Ctrl+S", run: () => ACTIONS["save-revision"]() },
      { icon: "download", text: "Xuất bản thiết kế…", kbd: "Ctrl+E", run: () => ACTIONS.export() },
      { icon: "package", text: "Chuẩn bị gói bàn giao…", run: () => APP.go("publishing") },
      { icon: "history", text: "Sao lưu ngay", run: () => UI.toast({ type: "info", title: "Đã bắt đầu sao lưu", desc: "Theo dõi trong Task Center." }) },
      { icon: "contrast", text: "Đổi giao diện sáng / tối", run: () => ACTIONS["toggle-theme"]() },
      { icon: "activity", text: "Mở Task Center", kbd: "Ctrl+J", run: () => ACTIONS.tasks() },
      { icon: "keyboard", text: "Xem phím tắt", kbd: "?", run: () => ACTIONS.shortcuts() },
    ].map((c) => ({ ...c, group: "Lệnh" }));
    const projs = D.projects.map((p) => ({ group: "Dự án", icon: "design", text: p.name, sub: `${p.type} · ${p.rev}`, run: () => APP.go("design") }));
    const assets = D.assets.map((a) => ({ group: "Tài nguyên", icon: a.kind === "Âm thanh" ? "audio" : a.kind === "Video" ? "video" : "image", text: a.name, sub: UI.status[a.state][2], run: () => APP.go("library") }));
    return [...acts, ...nav, ...projs, ...assets];
  }
  ACTIONS.palette = () => {
    if (UI.dialogOpen()) return;
    const all = commands();
    const dlg = UI.dialog({
      title: "Tìm hoặc chạy lệnh",
      size: "lg",
      body: `<div class="input-affix">${I("search")}<input class="input" id="pal-q" autofocus placeholder="Gõ tên màn hình, lệnh, dự án hoặc tệp… (không cần dấu)" role="combobox" aria-expanded="true" aria-controls="pal-list" aria-autocomplete="list"></div>
             <div id="pal-list" role="listbox" aria-label="Kết quả" style="max-height:52vh;overflow:auto;margin:0 -20px"></div>`,
      footer: `<span class="foot-note row" style="gap:10px">${UI.kbd("↑", "↓")} chọn ${UI.kbd("Enter")} chạy ${UI.kbd("Esc")} đóng</span>`,
      onOpen(d) {
        const q = d.el.querySelector("#pal-q"), listEl = d.el.querySelector("#pal-list");
        let items = [], idx = 0;
        const draw = () => {
          const t = UI.norm(q.value.trim());
          /* Xếp hạng: khớp đầu tên > khớp đầu một từ > khớp bất kỳ đâu; giữ thứ tự nhóm khi bằng điểm */
          const score = (c) => {
            const name = UI.norm(c.text);
            if (name.startsWith(t)) return 0;
            if ((" " + name).includes(" " + t)) return 1;
            return UI.norm(c.text + " " + (c.sub || "") + " " + c.group).includes(t) ? 2 : -1;
          };
          items = !t ? all.slice(0, 40) : all.map((c, i) => ({ c, i, s: score(c) })).filter((x) => x.s >= 0).sort((a, b) => a.s - b.s || a.i - b.i).map((x) => x.c).slice(0, 40);
          idx = 0;
          let g = "";
          listEl.innerHTML = items.length ? items.map((c, i) => {
            const head = c.group !== g ? `<div class="list-group-label section-label" style="padding-left:20px">${(g = c.group)}</div>` : "";
            return head + `<div class="list-item" role="option" id="pal-${i}" data-i="${i}" style="padding-left:20px;padding-right:20px">${I(c.icon)}<span class="lines"><span>${c.text}</span>${c.sub ? `<span class="xs muted">${c.sub}</span>` : ""}</span>${c.kbd ? `<span class="meta">${UI.kbd(...c.kbd.split("+"))}</span>` : ""}</div>`;
          }).join("") : UI.empty({ icon: "search", title: "Không có kết quả", desc: `Không tìm thấy “${UI.esc(q.value)}”. Thử từ khóa khác hoặc mở Thư viện để lọc nâng cao.` });
          mark();
        };
        const mark = () => {
          listEl.querySelectorAll("[role=option]").forEach((o) => o.setAttribute("aria-selected", +o.dataset.i === idx));
          const cur = listEl.querySelector(`#pal-${idx}`);
          if (cur) { q.setAttribute("aria-activedescendant", cur.id); cur.scrollIntoView({ block: "nearest" }); }
        };
        const run = (i) => { const c = items[i]; if (!c) return; d.close(); c.run(); };
        q.addEventListener("input", draw);
        q.addEventListener("keydown", (e) => {
          if (e.key === "ArrowDown") { e.preventDefault(); idx = Math.min(items.length - 1, idx + 1); mark(); }
          if (e.key === "ArrowUp") { e.preventDefault(); idx = Math.max(0, idx - 1); mark(); }
          if (e.key === "Enter") { e.preventDefault(); run(idx); }
        });
        listEl.addEventListener("click", (e) => { const o = e.target.closest("[role=option]"); if (o) run(+o.dataset.i); });
        listEl.addEventListener("mousemove", (e) => { const o = e.target.closest("[role=option]"); if (o && +o.dataset.i !== idx) { idx = +o.dataset.i; mark(); } });
        draw();
      },
    });
  };

  /* ---------- Phím tắt ---------- */
  const typing = (el) => el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName));
  document.addEventListener("keydown", (e) => {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === "k") { e.preventDefault(); ACTIONS.palette(); return; }
    if (UI.dialogOpen()) return;
    if (mod && /^[1-9]$/.test(e.key)) { const n = NAV.filter((x) => x !== "-")[+e.key - 1]; if (n) { e.preventDefault(); APP.go(n.id, { focus: true }); } return; }
    if (mod && e.key === ",") { e.preventDefault(); APP.go("settings", { focus: true }); return; }
    if (mod && e.key.toLowerCase() === "b") { e.preventDefault(); ACTIONS["toggle-sidebar"](); return; }
    if (mod && e.key === ".") { e.preventDefault(); ACTIONS["toggle-inspector"](); return; }
    if (mod && e.key.toLowerCase() === "j") { e.preventDefault(); ACTIONS.tasks(); return; }
    if (mod && e.key.toLowerCase() === "n") { e.preventDefault(); ACTIONS["new-project"](); return; }
    if (mod && e.key.toLowerCase() === "s") { e.preventDefault(); ACTIONS["save-revision"](); return; }
    if (mod && e.key.toLowerCase() === "e") { e.preventDefault(); ACTIONS.export(); return; }
    if (e.key === "?" && !typing(e.target)) { e.preventDefault(); ACTIONS.shortcuts(); return; }
    if (e.key === "Escape") { if (!$("#taskcenter").hidden) { $("#taskcenter").hidden = true; } ACTIONS["close-drawers"](); }
  });

  /* ---------- Ủy quyền sự kiện ---------- */
  document.addEventListener("click", (e) => {
    const go = e.target.closest("[data-go]");
    if (go && !go.disabled) {
      e.preventDefault();
      if (go.dataset.settings) store.set("settingsTab", go.dataset.settings);
      APP.go(go.dataset.go, { focus: e.detail === 0 });
      ACTIONS["close-drawers"]();
      return;
    }
    const act = e.target.closest("[data-action]");
    if (act && !act.disabled && act.getAttribute("aria-disabled") !== "true" && ACTIONS[act.dataset.action]) {
      e.preventDefault();
      ACTIONS[act.dataset.action](act, e);
    }
    const menuBtn = e.target.closest("[data-menu]");
    if (menuBtn && MENUS[menuBtn.dataset.menu]) { e.preventDefault(); UI.menu(menuBtn, MENUS[menuBtn.dataset.menu](menuBtn)); }
  });

  const MENUS = (window.MENUS = {
    workspace: () => [
      { label: "Workspace", group: true },
      ...D.workspaces.map((w) => ({ icon: w.id === D.workspace.id ? "check" : (w.id === "global" ? "globe" : "folder"), text: `${w.name}`, action: () => UI.toast({ title: `Đã chuyển sang ${w.name}`, desc: "Prototype: dữ liệu mẫu không đổi." }) })),
      "-",
      { icon: "plus", text: "Tạo workspace…" },
      { icon: "import", text: "Nhập gói portable…" },
      { icon: "settings", text: "Quản lý workspace", action: () => APP.go("settings") },
    ],
    help: () => [
      { icon: "keyboard", text: "Phím tắt", kbd: "?", action: () => ACTIONS.shortcuts() },
      { icon: "file-text", text: "Hướng dẫn sử dụng" },
      { icon: "list-checks", text: "Năng lực hệ thống", action: () => { store.set("settingsTab", "caps"); APP.go("settings"); } },
      "-",
      { icon: "package", text: "Tạo gói hỗ trợ (không chứa khóa bí mật)" },
      { icon: "info", text: "Giới thiệu Local Content Studio" },
    ],
    history: () => [
      { label: "Revision", group: true },
      { icon: "commit", text: "rev-4 · Đổi CTA trang 6 · 23:08", action: () => {} },
      { icon: "commit", text: "rev-3 · Áp dụng Brand v2 · 22:40" },
      { icon: "commit", text: "rev-2 · Thêm ảnh sản phẩm · 21:55" },
      "-",
      { icon: "git-compare", text: "So sánh với revision trước…" },
      { icon: "history", text: "Mở toàn bộ lịch sử" },
    ],
  });
  document.addEventListener("click", (e) => {
    if (e.target.closest("#ws-switch")) UI.menu($("#ws-switch"), MENUS.workspace());
    if (e.target.closest("#tb-help")) UI.menu($("#tb-help"), MENUS.help());
    if (e.target.closest("#save-state")) UI.menu($("#save-state"), MENUS.history());
  });

  /* ---------- Thanh điều khiển prototype ---------- */
  function protoBar() {
    const w = store.get("width", "fill");
    $("#proto-bar").innerHTML = `
      <span class="proto-tag">PROTOTYPE</span><span class="sep"></span>
      <label for="pb-state">Trạng thái</label>
      <select id="pb-state">
        <option value="normal">Có dữ liệu</option><option value="loading">Đang tải</option><option value="empty">Trống</option><option value="error">Lỗi</option><option value="blocked">Bị chặn</option>
      </select><span class="sep"></span>
      <button data-theme-btn="light">Sáng</button><button data-theme-btn="dark">Tối</button><span class="sep"></span>
      ${["fill", "1280", "768", "390", "320"].map((x) => `<button data-w="${x}" aria-pressed="${x === w}">${x === "fill" ? "Toàn màn" : x}</button>`).join("")}
      <span class="sep"></span><button class="proto-toggle" id="pb-toggle" aria-label="Thu gọn thanh prototype">–</button>`;
    $("#pb-state").value = APP.state;
    $("#pb-state").onchange = (e) => { APP.state = e.target.value; APP.rerender(); };
    $$("[data-theme-btn]").forEach((b) => (b.onclick = () => { APP.theme = b.dataset.themeBtn; store.set("theme", APP.theme); applyTheme(); }));
    $$("[data-w]").forEach((b) => (b.onclick = () => {
      $("#viewport").dataset.width = b.dataset.w; store.set("width", b.dataset.w);
      $$("[data-w]").forEach((x) => x.setAttribute("aria-pressed", x === b));
      ACTIONS["close-drawers"]();
    }));
    $("#pb-toggle").onclick = () => { const pb = $("#proto-bar"); pb.classList.toggle("collapsed"); $("#pb-toggle").textContent = pb.classList.contains("collapsed") ? "PROTOTYPE +" : "–"; };
  }

  /* ---------- Khởi động ---------- */
  document.addEventListener("DOMContentLoaded", () => {
    const fromHash = location.hash.replace("#/", "");
    if (PAGES.get(fromHash)) APP.page = fromHash;
    shell();
    render();
    addEventListener("hashchange", () => { const id = location.hash.replace("#/", ""); if (PAGES.get(id) && id !== APP.page) APP.go(id); });
  });
})();
