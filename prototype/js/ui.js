/* Thư viện UI dùng chung cho prototype: trạng thái, tooltip, toast, dialog, menu, tabs. */
(function () {
  const UI = {};
  window.UI = UI;

  const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
  UI.esc = esc;
  UI.norm = (s) => String(s).normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/đ/g, "d").replace(/Đ/g, "D").toLowerCase();

  /* ---------- Nhãn trạng thái: không bao giờ hiển thị enum thô ---------- */
  const S = {
    // Quyền tài nguyên
    QUARANTINED: ["warning", "shield-alert", "Đang cách ly", "Tệp mới chưa được xác nhận nguồn và giấy phép. Chưa thể dùng để xuất hoặc bàn giao."],
    RIGHTS_READY: ["success", "shield-check", "Đủ quyền dùng", "Người dùng đã xác nhận nguồn, giấy phép và quyền xuất bản."],
    REVOKED: ["danger", "x-circle", "Đã thu hồi quyền", "Mọi bản xuất và gói bàn giao dùng tài nguyên này bị chặn."],
    // Revision / nội dung
    SAVED: ["success", "check", "Đã lưu", "Revision đã được ghi xuống catalog cục bộ."],
    UNSAVED: ["warning", "clock", "Có thay đổi chưa lưu", "Bản nháp tự động lưu, nhưng chưa tạo revision."],
    AUTOSAVED: ["neutral", "history", "Đã tự lưu nháp", "Bản nháp phiên làm việc, chưa phải revision."],
    STALE: ["warning", "alert", "Lỗi thời", "Nguồn phía trên đã thay đổi sau khi tạo mục này."],
    PREVIEW_READY: ["info", "eye", "Có bản xem trước", "Renderer đã tạo bản xem trước từ revision đã lưu."],
    EXPORTED: ["success", "check-circle", "Đã xuất và kiểm hash", "Tệp đầu ra đã được đọc lại và khớp SHA-256."],
    // Bàn giao / xuất bản
    NOT_PREPARED: ["neutral", "package", "Chưa đóng gói", ""],
    PREPARED: ["neutral", "package", "Đã đóng gói", "Gói bàn giao bất biến đã được tạo, chưa gửi."],
    TRANSFERRING: ["info", "loader", "Đang truyền", "Đang gửi gói sang Social Harness."],
    ACCEPTED: ["success", "receipt", "Social đã nhận", "Có biên nhận hợp lệ từ Social Harness. Chưa có nghĩa là đã đăng."],
    AWAITING_APPROVAL: ["neutral", "clock", "Chờ duyệt tại Social", "Người duyệt trong Social Harness chưa quyết định."],
    APPROVED: ["success", "check", "Social đã duyệt", "Người duyệt đã chấp thuận đúng revision này."],
    QUEUED: ["info", "clock", "Đang chờ đăng", "Đã xếp lịch tại Social. Chưa đăng."],
    SENT: ["info", "publishing", "Đã gửi tới nền tảng", "Nền tảng chưa xác nhận bài viết."],
    UNKNOWN: ["warning", "help", "Chưa rõ kết quả", "Không nhận được xác nhận. Cần đối soát trước khi làm gì tiếp."],
    NEEDS_RECONCILIATION: ["warning", "alert", "Cần đối soát", "Kết nối bị gián đoạn trước khi có biên nhận. Không gửi lại."],
    USER_REPORTED: ["neutral", "user", "Người dùng báo đã đăng", "Chưa có xác minh từ nền tảng."],
    VERIFIED: ["success", "shield-check", "Đã xác minh", "Nền tảng đã xác nhận bài viết qua permalink."],
    REJECTED: ["danger", "x-circle", "Bị từ chối", "Social Harness từ chối gói. Xem lý do."],
    NOT_CONNECTED: ["neutral", "wifi-off", "Chưa kết nối Social", "Chưa cấu hình hoặc không liên lạc được Social Harness."],
    // Tiến trình
    PENDING: ["neutral", "clock", "Đang chờ", ""],
    RUNNING: ["info", "loader", "Đang chạy", ""],
    VERIFYING: ["info", "shield", "Đang kiểm đầu ra", ""],
    DONE: ["success", "check", "Hoàn tất", ""],
    FAILED: ["danger", "x-circle", "Thất bại", ""],
    CANCELLED: ["neutral", "x", "Đã hủy", ""],
    // Brand
    PUBLISHED: ["success", "check", "Đã phát hành", "Phiên bản bất biến; tài liệu có thể ghim vào."],
    DRAFT: ["neutral", "writing", "Bản nháp", "Có thể chỉnh sửa; chưa dùng được để ghim."],
    // Năng lực
    CAP_VERIFIED: ["success", "check", "Đã kiểm chứng", "Có bằng chứng chạy thật trên máy này."],
    CAP_PROPOSED: ["neutral", "info", "Chưa kiểm chứng", "Có trong kế hoạch hoặc mã, chưa có bằng chứng chạy thật."],
    CAP_BLOCKED: ["danger", "lock", "Bị chặn", "Thiếu điều kiện an toàn hoặc phụ thuộc."],
    CAP_MISSING: ["warning", "alert", "Chưa cài", "Không tìm thấy công cụ trên máy."],
    // Nghiên cứu
    FETCHED: ["success", "check", "Đã thu thập", ""],
    BLOCKED_SSRF: ["danger", "shield-alert", "Bị chặn: địa chỉ nội bộ", "URL trỏ tới mạng riêng hoặc localhost."],
    UNTRUSTED: ["warning", "alert", "Dữ liệu chưa tin cậy", "Nội dung web chỉ là dữ liệu, không phải chỉ dẫn."],
  };
  UI.status = S;
  UI.badge = (key, opts = {}) => {
    const s = S[key] || ["neutral", "info", key, ""];
    const label = opts.label || s[2];
    const tip = opts.tip ?? s[3];
    return `<span class="badge ${s[0]}" ${tip ? `data-tip="${esc(tip)}"` : ""} tabindex="${tip ? 0 : -1}">${I(s[1])}${esc(label)}</span>`;
  };
  UI.chipBadge = (variant, icon, label, tip) =>
    `<span class="badge ${variant}" ${tip ? `data-tip="${esc(tip)}" tabindex="0"` : ""}>${icon ? I(icon) : ""}${esc(label)}</span>`;

  /* ---------- Khối trạng thái chuẩn ---------- */
  UI.empty = ({ icon = "folder", title, desc, actions = "", meta = "", variant = "" }) => `
    <div class="state ${variant}">
      <div class="state-icon">${I(icon, "icon-xl")}</div>
      <h2>${title}</h2>
      ${desc ? `<p>${desc}</p>` : ""}
      ${actions ? `<div class="state-actions">${actions}</div>` : ""}
      ${meta ? `<div class="state-meta">${meta}</div>` : ""}
    </div>`;
  UI.skeletonRows = (n = 6, h = 44) =>
    `<div class="col" aria-busy="true" aria-label="Đang tải">${Array.from({ length: n }, (_, i) => `<div class="skeleton" style="height:${h}px;opacity:${1 - i * 0.1}"></div>`).join("")}</div>`;
  UI.skeletonText = (n = 3) =>
    `<div class="col">${Array.from({ length: n }, (_, i) => `<div class="skeleton sk-line" style="width:${90 - i * 18}%"></div>`).join("")}</div>`;
  UI.callout = (variant, icon, title, body = "", actions = "") => `
    <div class="callout ${variant}" role="${variant === "danger" ? "alert" : "status"}">
      ${I(icon)}
      <div class="callout-body">
        <div class="callout-title">${title}</div>
        ${body ? `<div>${body}</div>` : ""}
        ${actions ? `<div class="callout-actions">${actions}</div>` : ""}
      </div>
    </div>`;
  UI.progress = (pct, label) =>
    pct == null
      ? `<div class="progress indeterminate" role="progressbar" aria-label="${esc(label || "Đang xử lý")}"><span></span></div>`
      : `<div class="progress" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${pct}" aria-label="${esc(label || "Tiến độ")}"><span style="width:${pct}%"></span></div>`;
  /* Header trang: tiêu đề, meta (phạm vi/revision/thời gian), hành động */
  UI.pageHeader = ({ title, meta = "", actions = "" }) => `
    <div class="page-header">
      <div class="page-title"><h1>${title}</h1>${meta ? `<div class="meta">${meta}</div>` : ""}</div>
      ${actions ? `<div class="page-actions">${actions}</div>` : ""}
    </div>`;
  /* Chọn nội dung theo trạng thái của prototype (Có dữ liệu / Đang tải / Trống / Lỗi / Bị chặn) */
  UI.byState = (st, map) => (map[st] ?? map.normal)();
  UI.metaSep = `<span aria-hidden="true">·</span>`;
  UI.kbd = (...keys) => keys.map((k) => `<span class="kbd">${k}</span>`).join("");

  /* ---------- Tooltip (hover + focus, không hover-only) ---------- */
  let tipEl, tipTimer, tipTarget;
  function showTip(target, immediate) {
    clearTimeout(tipTimer);
    const run = () => {
      const text = target.getAttribute("data-tip");
      if (!text) return;
      tipTarget = target;
      tipEl ||= Object.assign(document.createElement("div"), { className: "tooltip", role: "tooltip", id: "cs-tooltip" });
      if (!tipEl.isConnected) document.body.append(tipEl);
      const kbd = target.getAttribute("data-kbd");
      const sub = target.getAttribute("data-tip-sub");
      tipEl.innerHTML = `<span>${esc(text)}${sub ? `<span class="tip-sub">${esc(sub)}</span>` : ""}</span>${kbd ? `<span class="row" style="gap:2px">${kbd.split("+").map((k) => `<span class="kbd">${esc(k)}</span>`).join("")}</span>` : ""}`;
      target.setAttribute("aria-describedby", "cs-tooltip");
      const r = target.getBoundingClientRect();
      const pos = target.getAttribute("data-tip-pos") || "bottom";
      tipEl.classList.remove("show");
      tipEl.style.left = "0px"; tipEl.style.top = "0px";
      const t = tipEl.getBoundingClientRect();
      let x, y;
      if (pos === "right") { x = r.right + 8; y = r.top + r.height / 2 - t.height / 2; }
      else if (pos === "left") { x = r.left - t.width - 8; y = r.top + r.height / 2 - t.height / 2; }
      else if (pos === "top") { x = r.left + r.width / 2 - t.width / 2; y = r.top - t.height - 8; }
      else { x = r.left + r.width / 2 - t.width / 2; y = r.bottom + 8; if (y + t.height > innerHeight - 4) y = r.top - t.height - 8; }
      x = Math.max(4, Math.min(x, innerWidth - t.width - 4));
      tipEl.style.left = x + "px"; tipEl.style.top = y + "px";
      requestAnimationFrame(() => tipEl.classList.add("show"));
    };
    immediate ? run() : (tipTimer = setTimeout(run, 350));
  }
  function hideTip() {
    clearTimeout(tipTimer);
    if (tipEl) tipEl.classList.remove("show");
    if (tipTarget) tipTarget.removeAttribute("aria-describedby");
    tipTarget = null;
  }
  UI.hideTip = hideTip;
  document.addEventListener("mouseover", (e) => { if (tipTarget && !tipTarget.isConnected) hideTip(); const t = e.target.closest("[data-tip]"); if (t && t !== tipTarget) showTip(t, false); else if (!t && tipTarget) hideTip(); });
  document.addEventListener("keydown", () => { if (tipTarget && !tipTarget.isConnected) hideTip(); });
  document.addEventListener("focusin", (e) => { const t = e.target.closest("[data-tip]"); if (t && t.matches(":focus-visible")) showTip(t, true); });
  document.addEventListener("focusout", hideTip);
  document.addEventListener("mousedown", hideTip);
  document.addEventListener("keydown", (e) => { if (e.key === "Escape" && tipTarget) hideTip(); });
  document.addEventListener("scroll", hideTip, true);

  /* ---------- Toast ---------- */
  UI.toast = ({ type = "info", title, desc = "", action, actionLabel, timeout = 5000 }) => {
    let host = document.querySelector(".toasts");
    if (!host) {
      host = Object.assign(document.createElement("div"), { className: "toasts" });
      host.setAttribute("aria-live", "polite");
      document.body.append(host);
    }
    const icon = { success: "check-circle", warning: "alert", danger: "alert-circle", info: "info" }[type];
    const el = document.createElement("div");
    el.className = `toast ${type}`;
    if (type === "danger") el.setAttribute("role", "alert");
    el.innerHTML = `${I(icon)}<div class="toast-body"><strong>${title}</strong>${desc ? `<span class="muted xs">${desc}</span>` : ""}${actionLabel ? `<div><button class="link-btn xs" data-toast-action>${actionLabel}</button></div>` : ""}</div><button class="btn quiet sm icon" aria-label="Đóng thông báo">${I("x")}</button>`;
    const close = () => el.remove();
    el.querySelector('[aria-label="Đóng thông báo"]').onclick = close;
    if (action) el.querySelector("[data-toast-action]").onclick = () => { action(); close(); };
    host.append(el);
    if (timeout && type !== "danger") setTimeout(close, timeout);
  };

  /* ---------- Dialog: focus trap + khôi phục focus ---------- */
  const stack = [];
  UI.dialog = ({ title, body, footer = "", size = "", guard = false, onOpen, label }) => {
    const prev = document.activeElement;
    const bd = document.createElement("div");
    bd.className = "backdrop";
    const id = "dlg-" + Math.random().toString(36).slice(2, 8);
    bd.innerHTML = `<div class="dialog ${size}" role="dialog" aria-modal="true" aria-labelledby="${id}">
        <div class="dialog-head"><h2 id="${id}">${title}</h2><button class="btn quiet icon" data-close data-tip="Đóng" data-kbd="Esc" aria-label="Đóng">${I("x")}</button></div>
        <div class="dialog-body">${body}</div>
        ${footer ? `<div class="dialog-foot">${footer}</div>` : ""}
      </div>`;
    const api = {
      el: bd.firstElementChild,
      close() {
        hideTip();
        bd.remove();
        stack.splice(stack.indexOf(api), 1);
        prev && prev.focus && prev.focus();
      },
      guard,
    };
    bd.addEventListener("mousedown", (e) => { if (e.target === bd && !api.guard) api.close(); });
    bd.addEventListener("click", (e) => { if (e.target.closest("[data-close]")) api.close(); });
    document.body.append(bd);
    stack.push(api);
    onOpen && onOpen(api);
    const first = api.el.querySelector("[autofocus]") || api.el.querySelector(".dialog-body input, .dialog-body select, .dialog-body textarea, .dialog-body button, .dialog-foot .btn.primary") || api.el.querySelector("button");
    first && first.focus();
    return api;
  };
  UI.confirm = ({ title, body, confirmLabel = "Xác nhận", destructive = false, onConfirm }) =>
    UI.dialog({
      title,
      body,
      footer: `<span class="foot-note"></span><button class="btn" data-close>Hủy</button><button class="btn ${destructive ? "destructive" : "primary"}" data-ok>${confirmLabel}</button>`,
      onOpen: (d) => d.el.querySelector("[data-ok]").addEventListener("click", () => { d.close(); onConfirm && onConfirm(); }),
    });
  document.addEventListener("keydown", (e) => {
    const top = stack[stack.length - 1];
    if (!top) return;
    if (e.key === "Escape") {
      if (top.guard) { UI.toast({ type: "warning", title: "Còn quyết định chưa lưu", desc: "Dùng nút Hủy để đóng có chủ ý." }); return; }
      e.stopPropagation(); top.close();
    }
    if (e.key === "Tab") {
      const f = [...top.el.querySelectorAll('button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])')].filter((x) => x.offsetParent);
      if (!f.length) return;
      const a = f[0], z = f[f.length - 1];
      if (e.shiftKey && document.activeElement === a) { e.preventDefault(); z.focus(); }
      else if (!e.shiftKey && document.activeElement === z) { e.preventDefault(); a.focus(); }
    }
  }, true);
  UI.dialogOpen = () => stack.length > 0;

  /* ---------- Menu ngữ cảnh ---------- */
  let openMenu;
  UI.closeMenu = () => { if (openMenu) { openMenu.el.remove(); openMenu.trigger?.setAttribute("aria-expanded", "false"); openMenu = null; } };
  UI.menu = (trigger, items) => {
    UI.closeMenu();
    const el = document.createElement("div");
    el.className = "menu";
    el.setAttribute("role", "menu");
    el.innerHTML = items.map((it) => {
      if (it === "-") return `<div class="menu-sep" role="separator"></div>`;
      if (it.label && it.group) return `<div class="menu-label section-label">${it.label}</div>`;
      return `<button role="menuitem" class="${it.danger ? "danger" : ""}" ${it.disabled ? `aria-disabled="true" data-tip="${esc(it.disabled)}" data-tip-pos="right"` : ""}>${it.icon ? I(it.icon) : `<span style="width:16px"></span>`}<span>${it.text}</span>${it.kbd ? `<span class="kbd">${it.kbd}</span>` : ""}</button>`;
    }).join("");
    document.body.append(el);
    const btns = [...el.querySelectorAll('[role="menuitem"]')];
    const acts = items.filter((it) => it !== "-" && !it.group);
    btns.forEach((b, i) => b.addEventListener("click", () => { if (acts[i].disabled) return; UI.closeMenu(); acts[i].action && acts[i].action(); }));
    const r = trigger.getBoundingClientRect();
    const m = el.getBoundingClientRect();
    let x = r.right - m.width; if (x < 4) x = r.left;
    let y = r.bottom + 4; if (y + m.height > innerHeight - 4) y = r.top - m.height - 4;
    el.style.left = Math.max(4, x) + "px"; el.style.top = y + "px";
    trigger.setAttribute("aria-expanded", "true");
    openMenu = { el, trigger };
    const enabled = btns.filter((b) => b.getAttribute("aria-disabled") !== "true");
    enabled[0] && enabled[0].focus();
    el.addEventListener("keydown", (e) => {
      const i = enabled.indexOf(document.activeElement);
      if (e.key === "ArrowDown") { e.preventDefault(); enabled[(i + 1) % enabled.length].focus(); }
      if (e.key === "ArrowUp") { e.preventDefault(); enabled[(i - 1 + enabled.length) % enabled.length].focus(); }
      if (e.key === "Escape") { e.stopPropagation(); UI.closeMenu(); trigger.focus(); }
    });
  };
  document.addEventListener("mousedown", (e) => { if (openMenu && !openMenu.el.contains(e.target) && !openMenu.trigger.contains(e.target)) UI.closeMenu(); });

  /* ---------- Tabs, segmented, chọn trong danh sách, sao chép ---------- */
  document.addEventListener("click", (e) => {
    const tab = e.target.closest('[role="tab"]');
    if (tab) selectTab(tab);
    const seg = e.target.closest(".segmented > button, [data-exclusive] > button");
    if (seg) { seg.parentElement.querySelectorAll("button").forEach((b) => b.setAttribute("aria-pressed", b === seg ? "true" : "false")); }
    const sel = e.target.closest("[data-selectable] > *");
    if (sel && !e.target.closest("button:not(.list-item), input, a")) {
      sel.parentElement.querySelectorAll(":scope > .is-selected").forEach((x) => x.classList.remove("is-selected"));
      sel.classList.add("is-selected");
    }
    const cp = e.target.closest("[data-copy]");
    if (cp) { navigator.clipboard?.writeText(cp.getAttribute("data-copy")).catch(() => {}); UI.toast({ type: "success", title: "Đã sao chép", desc: cp.getAttribute("data-copy"), timeout: 2200 }); }
  });
  function selectTab(tab) {
    const list = tab.closest('[role="tablist"]');
    list.querySelectorAll('[role="tab"]').forEach((t) => {
      const on = t === tab;
      t.setAttribute("aria-selected", on);
      t.tabIndex = on ? 0 : -1;
      const p = document.getElementById(t.getAttribute("aria-controls"));
      if (p) p.hidden = !on;
    });
  }
  document.addEventListener("keydown", (e) => {
    const tab = e.target.closest?.('[role="tab"]');
    if (!tab || !["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) return;
    const tabs = [...tab.closest('[role="tablist"]').querySelectorAll('[role="tab"]')];
    let i = tabs.indexOf(tab);
    i = e.key === "ArrowRight" ? (i + 1) % tabs.length : e.key === "ArrowLeft" ? (i - 1 + tabs.length) % tabs.length : e.key === "Home" ? 0 : tabs.length - 1;
    tabs[i].focus(); selectTab(tabs[i]); e.preventDefault();
  });
  UI.tabs = (id, items, cls = "") =>
    `<div class="tabs ${cls}" role="tablist">${items.map((t, i) => `<button role="tab" id="${id}-t${i}" aria-controls="${id}-p${i}" aria-selected="${i === (items.sel || 0)}" tabindex="${i === (items.sel || 0) ? 0 : -1}">${t.icon ? I(t.icon) : ""}${t.label}${t.count != null ? `<span class="count">${t.count}</span>` : ""}</button>`).join("")}</div>`;
  UI.tabPanel = (id, i, html, sel = 0, cls = "") => `<div role="tabpanel" id="${id}-p${i}" aria-labelledby="${id}-t${i}" class="${cls}" ${i === sel ? "" : "hidden"}>${html}</div>`;
})();
