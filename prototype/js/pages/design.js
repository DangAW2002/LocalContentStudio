(function () {
  /* Tài liệu thiết kế mẫu (dữ liệu nội dung: màu là màu đầu ra, không đổi theo theme) */
  const W = 1080, H = 1350;
  const EL = [
    { id: "bg", name: "Nền", kind: "Hình khối", icon: "square", x: 0, y: 0, w: 1080, h: 1350, locked: true, style: "background:#FFFFFF" },
    { id: "band", name: "Dải màu chủ đạo", kind: "Hình khối", icon: "square", x: 0, y: 1120, w: 1080, h: 230, locked: true, style: "background:#EEF2F8" },
    { id: "logo", name: "Logo", kind: "Ảnh", icon: "image", x: 80, y: 72, w: 240, h: 56, html: `<span style="font-weight:800;color:#334BD1;font-size:calc(40px*var(--s))">LS</span><span style="font-weight:600;color:#182132;font-size:calc(28px*var(--s));margin-left:calc(12px*var(--s))">Local Studio</span>`, style: "display:flex;align-items:center", locked: true },
    { id: "idx", name: "Số trang", kind: "Chữ", icon: "type", x: 880, y: 80, w: 120, h: 48, html: "1 / 6", style: "color:#526078;font-size:calc(28px*var(--s));text-align:right;justify-content:flex-end" },
    { id: "head", name: "Tiêu đề", kind: "Chữ", icon: "type", x: 80, y: 200, w: 920, h: 250, html: "Giảm 42% thời gian làm nội dung", style: "color:#182132;font-weight:800;font-size:calc(92px*var(--s));line-height:1.08;letter-spacing:-.01em", bind: "brief.headline" },
    { id: "sub", name: "Mô tả", kind: "Chữ", icon: "type", x: 80, y: 470, w: 860, h: 110, html: "Từ nghiên cứu đến bản xuất, mọi thứ nằm trên máy bạn.", style: "color:#526078;font-size:calc(38px*var(--s));line-height:1.35" },
    { id: "img", name: "Ảnh sản phẩm", kind: "Ảnh", icon: "image", x: 80, y: 610, w: 920, h: 470, asset: "banner-chinh.png", style: "background:linear-gradient(135deg,#7f93e8,#334BD1)" },
    { id: "cta", name: "Nút kêu gọi", kind: "Hình khối + chữ", icon: "square", x: 80, y: 1180, w: 440, h: 104, html: "Vuốt để xem →", style: "background:#334BD1;color:#fff;font-weight:700;font-size:calc(36px*var(--s));justify-content:center" },
    { id: "cite", name: "Ghi nguồn", kind: "Chữ", icon: "type", x: 560, y: 1200, w: 440, h: 64, html: "Nguồn: Báo cáo xu hướng Q4 [1]", style: "color:#526078;font-size:calc(22px*var(--s));justify-content:flex-end;text-align:right" },
  ];
  const S = { sel: "head", zoom: 42, fit: true, board: 0, grid: false, safe: true, tool: "select" };

  const num = (lbl, val, tip) => `<label class="num-field" data-tip="${tip}"><span>${lbl}</span><input value="${val}" aria-label="${tip}"></label>`;

  PAGES.register({
    id: "design",
    label: "Thiết kế",
    sidebar({ st }) {
      if (st === "empty") return null;
      const pages = `<div class="board-list" data-selectable>${Array.from({ length: 6 }, (_, i) => `
          <button class="board-item ${i === S.board ? "is-selected" : ""}" data-board="${i}" aria-label="Trang ${i + 1}" aria-current="${i === S.board}">
            <span class="board-n">${i + 1}</span><span class="board-thumb"><span style="background:#334BD1;height:14%;width:60%"></span><span style="background:${["#7f93e8", "#9fb0ef", "#c3cdf5", "#7f93e8", "#9fb0ef", "#EEF2F8"][i]};height:36%;width:100%"></span><span style="background:#334BD1;height:8%;width:40%"></span></span>
            ${i === 2 ? `<span class="board-warn" data-tip="Chữ có thể tràn khung" tabindex="0">${I("alert")}</span>` : ""}
          </button>`).join("")}</div>`;
      const layers = `<div class="list" role="tree" aria-label="Lớp" data-selectable>${[...EL].reverse().map((e) => `
          <div class="list-item layer-row ${e.id === S.sel ? "is-selected" : ""}" role="treeitem" aria-selected="${e.id === S.sel}" tabindex="0" data-pick="${e.id}">
            ${I(e.icon)}<span class="truncate">${e.name}</span>
            <span class="item-actions" style="${e.locked ? "opacity:1" : ""}">
              <button class="btn sm quiet icon" aria-label="Ẩn ${e.name}" data-tip="Ẩn/hiện">${I("eye")}</button>
              <button class="btn sm quiet icon" aria-label="${e.locked ? "Mở khóa" : "Khóa"} ${e.name}" data-tip="${e.locked ? "Đang khóa · nhấn để mở" : "Khóa"}">${I(e.locked ? "lock" : "unlock")}</button>
            </span>
          </div>`).join("")}</div>`;
      const assets = `<div style="padding:8px"><div class="input-affix">${I("search")}<input class="input sm" placeholder="Tìm tài nguyên" aria-label="Tìm tài nguyên"></div></div>
        <div class="mini-assets">${DATA.assets.filter((a) => a.kind !== "Âm thanh").map((a) => a.state === "RIGHTS_READY"
          ? `<button class="mini-asset" draggable="true" data-tip="${a.name}" data-tip-sub="Kéo vào trang hoặc nhấn Enter để chèn"><span class="thumb" style="background:hsl(${a.tone} 40% 72%)">${I(a.kind === "Video" ? "video" : "image")}</span></button>`
          : `<button class="mini-asset" aria-disabled="true" data-tip="${a.name}: ${UI.status[a.state][2]}" data-tip-sub="Không chèn được cho tới khi đủ quyền"><span class="thumb" style="opacity:.4">${I("lock")}</span></button>`).join("")}</div>
        <div class="xs muted" style="padding:8px 12px">Chỉ tài nguyên đủ quyền mới chèn được. <a href="#" data-go="library">Mở thư viện</a></div>`;
      const templates = `<div class="list">${["Danh sách 5 ý", "Trước / sau", "Số liệu nổi bật", "Trích dẫn khách hàng", "Câu hỏi mở đầu"].map((t) => `<button class="list-item">${I("layout-grid")}<span>${t}</span><span class="meta">6 trang</span></button>`).join("")}</div>`;
      return {
        title: "Carousel Q4",
        body: `${UI.tabs("ds", [{ label: "Trang" }, { label: "Lớp" }, { label: "Tài nguyên" }, { label: "Mẫu" }], "sm")}
          ${UI.tabPanel("ds", 0, pages)}${UI.tabPanel("ds", 1, layers)}${UI.tabPanel("ds", 2, assets)}${UI.tabPanel("ds", 3, templates)}`,
        foot: `<button class="btn block" data-tip="Thêm trang sau trang hiện tại">${I("plus")}Thêm trang</button>`,
      };
    },
    main({ st }) {
      const toolbar = `<div class="design-toolbar" role="toolbar" aria-label="Công cụ thiết kế">
          <div class="segmented" aria-label="Công cụ">
            ${[["select", "cursor", "Chọn", "V"], ["text", "type", "Chữ", "T"], ["shape", "square", "Hình khối", "R"], ["image", "image", "Ảnh", "I"], ["hand", "hand", "Kéo khung nhìn", "H"]].map(([k, ic, t, kb]) => `<button aria-pressed="${S.tool === k}" aria-label="${t}" data-tip="${t}" data-kbd="${kb}" data-tool="${k}">${I(ic)}</button>`).join("")}
          </div>
          <span class="vdivider"></span>
          <button class="btn quiet sm icon" aria-label="Hoàn tác" data-tip="Hoàn tác: đổi màu CTA" data-kbd="Ctrl+Z">${I("undo")}</button>
          <button class="btn quiet sm icon" aria-label="Làm lại" data-tip="Làm lại" data-kbd="Ctrl+Y" disabled>${I("redo")}</button>
          <span class="vdivider hide-sm"></span>
          <div class="row hide-sm" style="gap:0">
            ${[["align-left", "Căn trái"], ["align-center", "Căn giữa ngang"], ["align-right", "Căn phải"], ["align-top", "Căn trên"], ["align-middle", "Căn giữa dọc"]].map(([ic, t]) => `<button class="btn quiet sm icon" aria-label="${t}" data-tip="${t}" data-tip-sub="Theo trang khi chọn 1 phần tử">${I(ic)}</button>`).join("")}
            <button class="btn quiet sm icon" aria-label="Nhóm" data-tip="Nhóm" data-kbd="Ctrl+G">${I("group")}</button>
          </div>
          <span class="spacer"></span>
          <button class="btn quiet sm icon" aria-pressed="${S.grid}" data-dz="grid" aria-label="Lưới" data-tip="Hiện lưới 8 px">${I("grid")}</button>
          <button class="btn quiet sm icon" aria-pressed="${S.safe}" data-dz="safe" aria-label="Vùng an toàn" data-tip="Hiện vùng an toàn" data-tip-sub="Theo hồ sơ kênh Instagram">${I("frame")}</button>
          <span class="vdivider"></span>
          <div class="zoom-ctl row" style="gap:0">
            <button class="btn quiet sm icon" data-dz="out" aria-label="Thu nhỏ" data-tip="Thu nhỏ" data-kbd="Ctrl+-">${I("zoom-out")}</button>
            <button class="btn quiet sm zoom-val" data-menu="zoom" aria-haspopup="menu" aria-label="Mức thu phóng ${S.zoom}%" data-tip="Chọn mức thu phóng">${S.zoom}%</button>
            <button class="btn quiet sm icon" data-dz="in" aria-label="Phóng to" data-tip="Phóng to" data-kbd="Ctrl+=">${I("zoom-in")}</button>
            <button class="btn quiet sm" data-dz="fit" aria-pressed="${S.fit}" data-tip="Vừa khung" data-kbd="Ctrl+0">Vừa khung</button>
          </div>
          <span class="vdivider"></span>
          <button class="btn sm" data-tip="Xem trước từ revision đã lưu" data-tip-sub="rev-4 · renderer studio-svg 0.3">${I("eye")}Xem trước</button>
          <button class="btn sm primary" data-action="export" data-tip="Xuất PNG, SVG, HTML" data-kbd="Ctrl+E">${I("download")}Xuất</button>
        </div>`;

      const s = S.zoom / 100;
      const els = EL.map((e) => {
        const err = st === "error" && e.id === "img";
        const blk = st === "blocked" && e.id === "img";
        const inner = e.asset
          ? (err ? `<div class="missing">${I("alert-circle", "icon-xl")}<span>Không tìm thấy “${e.asset}”</span></div>`
            : blk ? `<div class="missing">${I("x-circle", "icon-xl")}<span>Quyền sử dụng đã bị thu hồi</span></div>`
            : `<span style="margin:auto;color:rgba(255,255,255,.85);display:flex;flex-direction:column;align-items:center;gap:6px;font-size:calc(22px*var(--s))">${I("image", "icon-xl")}${e.asset}</span>`)
          : e.html || "";
        return `<div class="el ${e.id === S.sel ? "is-selected" : ""} ${e.locked ? "is-locked" : ""} ${err || blk ? "is-broken" : ""}" data-el="${e.id}" tabindex="${e.locked ? -1 : 0}" role="button" aria-label="${e.name}, ${e.kind}${e.id === S.sel ? ", đang chọn" : ""}"
          style="left:${(e.x / W) * 100}%;top:${(e.y / H) * 100}%;width:${(e.w / W) * 100}%;height:${(e.h / H) * 100}%;${e.style}">${inner}
          ${e.id === S.sel ? `<span class="sel-size">${e.w} × ${e.h}</span>${["nw", "n", "ne", "e", "se", "s", "sw", "w"].map((h) => `<span class="handle h-${h}"></span>`).join("")}` : ""}
          ${st === "blocked" && e.id === "head" ? "" : ""}
        </div>`;
      }).join("");

      const canvas = UI.byState(st, {
        loading: () => `<div class="canvas-scroll"><div class="skeleton" style="width:${W * s}px;height:${H * s}px"></div></div>`,
        empty: () => `<div class="canvas-scroll">${UI.empty({ icon: "design", title: "Dự án chưa có trang nào", desc: "Thêm trang trống hoặc bắt đầu từ mẫu. Khổ mặc định theo hồ sơ kênh: 1080 × 1350.", actions: `<button class="btn primary">${I("plus")}Thêm trang trống</button><button class="btn">${I("layout-grid")}Chọn mẫu</button>` })}</div>`,
        normal: () => `<div class="canvas-scroll" data-canvas>
            <div class="artboard-wrap">
              <div class="artboard-label"><span>Trang ${S.board + 1} · 1080 × 1350</span><span class="muted">Brand v2</span></div>
              <div class="artboard ${S.grid ? "show-grid" : ""}" style="width:${W * s}px;height:${H * s}px;--s:${s}" role="group" aria-label="Trang ${S.board + 1}">
                ${els}
                ${S.safe ? `<div class="safe-zone" style="top:${(60 / H) * 100}%;bottom:${(190 / H) * 100}%;left:${(60 / W) * 100}%;right:${(60 / W) * 100}%" aria-hidden="true"><span>Vùng an toàn</span></div>` : ""}
              </div>
            </div>
          </div>`,
      });
      const banner = st === "error"
        ? UI.callout("danger", "image", "Thiếu tệp gốc của “Ảnh sản phẩm”", "Tệp banner-chinh.png đã bị di chuyển. Bản xuất trang 1 bị chặn; các trang khác vẫn xuất được.", `<button class="btn sm primary">${I("link")}Tìm lại tệp</button><button class="btn sm">Thay ảnh khác</button>`)
        : st === "blocked"
        ? UI.callout("danger", "shield-alert", "Không thể xuất: 1 tài nguyên bị thu hồi quyền", "“Ảnh sản phẩm” dùng tệp có giấy phép đã hết hạn. Thay ảnh hoặc xin lại quyền trong Thư viện.", `<button class="btn sm primary">Thay ảnh</button><button class="btn sm" data-go="library">Mở trong thư viện</button>`)
        : "";
      const strip = st === "empty" ? "" : `<div class="board-strip">
          <span class="xs muted hide-sm">Trang</span>
          ${Array.from({ length: 6 }, (_, i) => `<button class="strip-item ${i === S.board ? "is-selected" : ""}" data-board="${i}" aria-label="Trang ${i + 1}" data-tip="Trang ${i + 1}${i === 2 ? " · có cảnh báo" : ""}">${i + 1}${i === 2 ? `<span class="badge-dot"></span>` : ""}</button>`).join("")}
          <button class="strip-item add" aria-label="Thêm trang" data-tip="Thêm trang">${I("plus")}</button>
          <span class="spacer"></span>
          <button class="btn quiet sm" data-tip="1 cảnh báo bố cục" data-tip-sub="Trang 3: tiêu đề có thể tràn khi dùng phông thay thế" style="color:var(--cs-warning)">${I("alert")}1 cảnh báo</button>
        </div>`;
      return `<div class="page design-page">${toolbar}${banner ? `<div style="padding:12px 16px 0">${banner}</div>` : ""}${canvas}${strip}</div>`;
    },
    inspector({ st }) {
      if (st === "empty" || st === "loading") return null;
      const e = EL.find((x) => x.id === S.sel);
      if (!e) {
        return {
          title: "Trang " + (S.board + 1),
          body: `<details class="insp-section" open><summary>${I("chev-right", "chev")}Khổ trang</summary><div class="insp-content">
              <select class="select sm"><option>1080 × 1350 (4:5)</option><option>1080 × 1080</option><option>1080 × 1920</option></select>
              <div class="grid-2">${num("W", 1080, "Rộng")}${num("H", 1350, "Cao")}</div></div></details>
            <details class="insp-section" open><summary>${I("chev-right", "chev")}Nền</summary><div class="insp-content">
              <div class="token-select"><span class="swatch" style="background:#FFFFFF;width:20px;height:20px"></span><select class="select sm"><option>brand.paper · #FFFFFF</option><option>brand.mist · #EEF2F8</option></select></div></div></details>
            <details class="insp-section" open><summary>${I("chev-right", "chev")}Hướng dẫn</summary><div class="insp-content">
              <label class="switch"><input type="checkbox" checked>Vùng an toàn (Instagram)</label><label class="switch"><input type="checkbox">Lưới 8 px</label><label class="switch"><input type="checkbox" checked>Bắt dính vào phần tử</label></div></details>`,
        };
      }
      const isText = e.kind === "Chữ" || e.id === "cta";
      return {
        title: e.name,
        actions: `<span class="badge">${e.kind}</span><button class="btn quiet sm icon" data-menu="element" aria-haspopup="menu" aria-label="Thêm hành động" data-tip="Thêm hành động">${I("more")}</button>`,
        body: `${UI.tabs("di", [{ label: "Thiết kế" }, { label: "Thương hiệu" }, { label: "AI", icon: "sparkles" }], "sm")}
          ${UI.tabPanel("di", 0, `
          <details class="insp-section" open><summary>${I("chev-right", "chev")}Vị trí & kích thước</summary><div class="insp-content">
            <div class="grid-2">${num("X", e.x, "Vị trí X (px)")}${num("Y", e.y, "Vị trí Y (px)")}${num("W", e.w, "Rộng (px)")}${num("H", e.h, "Cao (px)")}${num("°", 0, "Xoay (độ)")}
              <button class="btn sm" aria-pressed="true" data-tip="Giữ tỉ lệ khi đổi kích thước">${I("link")}Giữ tỉ lệ</button></div>
          </div></details>
          ${isText ? `<details class="insp-section" open><summary>${I("chev-right", "chev")}Chữ</summary><div class="insp-content">
            <select class="select sm" aria-label="Kiểu chữ"><option>Tiêu đề lớn · Brand v2</option><option>Tiêu đề</option><option>Nội dung</option></select>
            <div class="grid-2"><select class="select sm" aria-label="Phông"><option>Inter</option></select><select class="select sm" aria-label="Độ đậm"><option>ExtraBold 800</option><option>Bold 700</option></select>
            ${num("Aa", 92, "Cỡ chữ (px)")}${num("↕", "1.08", "Giãn dòng")}</div>
            <div class="segmented" data-exclusive aria-label="Căn chữ"><button aria-pressed="true" aria-label="Căn trái" data-tip="Căn trái">${I("align-left")}</button><button aria-pressed="false" aria-label="Căn giữa" data-tip="Căn giữa">${I("align-center")}</button><button aria-pressed="false" aria-label="Căn phải" data-tip="Căn phải">${I("align-right")}</button></div>
            <div class="token-select"><span class="swatch" style="background:#182132;width:20px;height:20px"></span><select class="select sm" aria-label="Màu chữ"><option>brand.ink · #182132</option><option>brand.primary · #334BD1</option><option>brand.accent · #F2A93B (không đạt tương phản)</option></select></div>
          </div></details>
          <details class="insp-section" open><summary>${I("chev-right", "chev")}Nội dung</summary><div class="insp-content">
            <textarea class="textarea" rows="2" style="font-size:13px;min-height:60px" aria-label="Nội dung chữ">${e.html}</textarea>
            ${e.bind ? `<div class="row xs muted">${I("link")}Gắn với <span class="mono">${e.bind}</span><span class="spacer"></span><button class="link-btn xs">Bỏ gắn</button></div>` : ""}
            <div class="row xs" style="color:var(--cs-success)">${I("check")}Vừa khung · còn 2 dòng trống</div>
          </div></details>` : ""}
          ${e.asset ? `<details class="insp-section" open><summary>${I("chev-right", "chev")}Ảnh</summary><div class="insp-content">
            <div class="row"><span class="thumb" style="width:48px;height:36px;background:#7f93e8"></span><div class="col" style="gap:0;min-width:0"><span class="truncate xs strong">${e.asset}</span>${UI.badge("RIGHTS_READY")}</div></div>
            <div class="segmented" data-exclusive><button aria-pressed="true">Phủ kín</button><button aria-pressed="false">Vừa khung</button><button aria-pressed="false">Cắt…</button></div>
            <button class="btn sm">${I("refresh")}Thay ảnh</button></div></details>` : ""}
          <details class="insp-section"><summary>${I("chev-right", "chev")}Hiệu ứng</summary><div class="insp-content">${num("%", 100, "Độ mờ (%)")}</div></details>`)}
          ${UI.tabPanel("di", 1, `<div class="insp-content" style="padding-top:12px">
            <div class="row" style="gap:8px;color:var(--cs-success)">${I("check-circle")}<span>Đúng kiểu chữ “Tiêu đề lớn”</span></div>
            <div class="row" style="gap:8px;color:var(--cs-success)">${I("check-circle")}<span>Màu từ bảng Brand v2</span></div>
            <div class="row" style="gap:8px;color:var(--cs-success)">${I("check-circle")}<span>Tương phản 16.1:1</span></div>
            <div class="row" style="gap:8px;color:var(--cs-success)">${I("check-circle")}<span>Không chứa từ cấm</span></div>
            <hr class="divider"><button class="btn sm">${I("refresh")}Áp lại toàn bộ Brand v2 cho trang</button></div>`)}
          ${UI.tabPanel("di", 2, `<div class="insp-content" style="padding-top:12px">
            <span class="xs muted">AI chỉ thấy phần tử đang chọn, brief và quy tắc thương hiệu.</span>
            <textarea class="textarea" rows="2" style="font-size:13px;min-height:60px" placeholder="Ví dụ: viết lại tiêu đề ngắn hơn 6 từ"></textarea>
            <button class="btn primary sm" data-action="ai-disclosure">${I("sparkles")}Tạo đề xuất</button></div>`)}`,
      };
    },
    mount(root) {
      const main = root.querySelector("#main");
      const canvas = main.querySelector("[data-canvas]");
      const pick = (id) => { S.sel = id; APP.rerender(); const n = document.querySelector(`[data-el="${id}"]`); n && n.focus({ preventScroll: true }); };
      if (canvas && S.fit) {
        /* "Vừa khung": tính tỉ lệ theo vùng canvas thật */
        const r = canvas.getBoundingClientRect();
        const z = Math.max(10, Math.floor(Math.min((r.width - 48) / W, (r.height - 72) / H) * 100));
        if (r.width > 0 && Math.abs(z - S.zoom) >= 2) { S.zoom = z; requestAnimationFrame(() => APP.rerender()); return; }
      }
      if (canvas) {
        canvas.addEventListener("click", (e) => {
          const el = e.target.closest("[data-el]");
          if (!el) { S.sel = null; APP.rerender(); return; }
          const def = EL.find((x) => x.id === el.dataset.el);
          if (def.locked) { UI.toast({ title: `“${def.name}” đang khóa`, desc: "Mở khóa trong bảng Lớp để chỉnh sửa." }); return; }
          pick(def.id);
        });
        canvas.addEventListener("keydown", (e) => {
          const el = e.target.closest("[data-el]");
          if (el && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); pick(el.dataset.el); }
          if (e.key === "Escape") { S.sel = null; APP.rerender(); }
          if (el && el.classList.contains("is-selected") && e.key.startsWith("Arrow")) { e.preventDefault(); const d = EL.find((x) => x.id === el.dataset.el); const k = e.shiftKey ? 10 : 1; if (e.key === "ArrowLeft") d.x -= k; if (e.key === "ArrowRight") d.x += k; if (e.key === "ArrowUp") d.y -= k; if (e.key === "ArrowDown") d.y += k; pick(d.id); }
        });
      }
      main.querySelectorAll("[data-dz]").forEach((b) => b.addEventListener("click", () => {
        const k = b.dataset.dz;
        if (k === "in") { S.zoom = Math.min(200, S.zoom + 10); S.fit = false; }
        if (k === "out") { S.zoom = Math.max(10, S.zoom - 10); S.fit = false; }
        if (k === "fit") S.fit = true;
        if (k === "grid") S.grid = !S.grid;
        if (k === "safe") S.safe = !S.safe;
        APP.rerender();
      }));
      main.querySelectorAll("[data-tool]").forEach((b) => b.addEventListener("click", () => { S.tool = b.dataset.tool; }));
      root.querySelectorAll("[data-board]").forEach((b) => b.addEventListener("click", () => { S.board = +b.dataset.board; APP.rerender(); }));
      root.querySelectorAll("[data-pick]").forEach((b) => b.addEventListener("click", (e) => { if (!e.target.closest(".item-actions")) pick(b.dataset.pick); }));
    },
  });

  window.MENUS.zoom = () => [25, 42, 50, 75, 100, 150, 200].map((z) => ({ icon: z === S.zoom ? "check" : null, text: z + "%", action: () => { S.zoom = z; S.fit = false; APP.rerender(); } })).concat(["-", { icon: S.fit ? "check" : null, text: "Vừa khung", kbd: "Ctrl+0", action: () => { S.fit = true; APP.rerender(); } }]);
  window.MENUS.element = () => [
    { icon: "copy", text: "Nhân bản", kbd: "Ctrl+D" },
    { icon: "group", text: "Nhóm", kbd: "Ctrl+G" },
    { icon: "lock", text: "Khóa" },
    { icon: "layers", text: "Đưa lên trên cùng" },
    "-",
    { icon: "trash", text: "Xóa", kbd: "Del", danger: true },
  ];
})();
