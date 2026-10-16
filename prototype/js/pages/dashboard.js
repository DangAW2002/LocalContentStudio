PAGES.register({
  id: "dashboard",
  label: "Tổng quan",
  main({ st, D }) {
    const header = UI.pageHeader({
      title: "Tổng quan",
      meta: `${I("folder")}${D.workspace.name} ${UI.metaSep} 4 dự án ${UI.metaSep} cập nhật 23:12`,
      actions: `
        <button class="btn" data-tip="Mở gói .lcs từ máy khác" data-tip-sub="Kiểm tra toàn vẹn trước khi nhập">${I("import")}Nhập gói</button>
        <button class="btn" data-go="research" data-tip="Thu thập nội dung từ một URL">${I("globe")}Thu thập URL</button>
        <button class="btn primary" data-action="new-project" data-tip="Tạo dự án mới" data-kbd="Ctrl+N">${I("plus")}Tạo dự án</button>`,
    });

    const body = UI.byState(st, {
      loading: () => `<div class="col-4"><div class="grid-3">${UI.skeletonRows(1, 88)}${UI.skeletonRows(1, 88)}${UI.skeletonRows(1, 88)}</div>${UI.skeletonRows(6)}</div>`,
      empty: () => `
        <div class="panel" style="max-width:880px;margin:24px auto">
          ${UI.empty({
            icon: "sparkles",
            title: "Bắt đầu dự án đầu tiên",
            desc: "Mọi dữ liệu nằm trên máy bạn. Bạn có thể làm toàn bộ quy trình mà không cần AI hay kết nối mạng.",
            actions: `<button class="btn primary" data-action="new-project">${I("plus")}Tạo dự án</button><button class="btn">${I("folder-open")}Mở thư mục dự án</button><button class="btn">${I("import")}Nhập gói portable</button>`,
          })}
          <hr class="divider">
          <div class="grid-3" style="padding:20px">
            ${[["research", "1. Thu thập nguồn", "Dán URL để lấy số liệu và trích dẫn có kiểm chứng.", "research"],
               ["brand", "2. Thiết lập thương hiệu", "Màu, phông, giọng điệu và từ cần tránh.", "brand"],
               ["design", "3. Thiết kế và xuất", "Carousel, poster, infographic; xuất PNG/SVG/HTML.", "design"]].map(([ic, t, d, go]) => `
              <button class="onboard-card" data-go="${go}">${I(ic, "icon-lg")}<span class="strong">${t}</span><span class="xs muted">${d}</span></button>`).join("")}
          </div>
        </div>`,
      error: () => `<div class="panel">${UI.empty({
          variant: "danger", icon: "database",
          title: "Không mở được catalog của workspace",
          desc: "Catalog được tạo bởi phiên bản mới hơn (schema v3). Phiên bản này chỉ đọc được tới schema v2. Dữ liệu của bạn chưa bị thay đổi.",
          actions: `<button class="btn primary">${I("eye")}Mở ở chế độ chỉ đọc</button><button class="btn">${I("history")}Khôi phục bản sao lưu 08:00</button><button class="btn">${I("download")}Tải phiên bản mới</button>`,
          meta: `Đường dẫn: <span class="mono">${D.workspace.path}\\catalog.sqlite</span> · Mã: <span class="copyable mono" data-copy="c-91ab02">c-91ab02</span>`,
        })}</div>`,
      blocked: () => `${UI.callout("danger", "hard-drive", "Ổ D: gần đầy: còn 118 MB", "Không thể lưu revision mới, xuất tệp hay sao lưu. Bạn vẫn xem và chỉnh sửa được; bản nháp được giữ trong bộ nhớ.", `<button class="btn sm">${I("trash")}Dọn bộ đệm xem trước (1,2 GB)</button><button class="btn sm">${I("folder-open")}Chuyển workspace sang ổ khác…</button>`)}
        <div style="height:16px"></div>${normal()}`,
      normal: () => normal(),
    });

    function normal() {
      const attention = [
        ["warning", "shield-alert", "2 tệp đang cách ly", "Cần xác nhận nguồn và giấy phép trước khi dùng.", `<button class="btn sm" data-go="library">Duyệt quyền</button>`],
        ["warning", "alert", "Gói “Poster cuối tuần → Instagram” cần đối soát", "Mất kết nối trước khi có biên nhận. Không gửi lại.", `<button class="btn sm" data-go="publishing">Đối soát</button>`],
        ["danger", "x-circle", "Render MP4 thất bại", "Thiếu phông “Be Vietnam Pro” ở cảnh 3.", `<button class="btn sm" data-action="job-error">Xem lỗi</button>`],
        ["neutral", "history", "“Infographic xu hướng” dùng Brand v1", "Brand v2 đã phát hành từ 02/10.", `<button class="btn sm" data-go="brand">So sánh</button>`],
      ];
      return `<div class="col-4">
        <section class="panel" aria-labelledby="att-h">
          <div class="panel-head"><h2 id="att-h">Cần xử lý</h2><span class="badge warning">4</span></div>
          <div class="list">${attention.map(([v, ic, t, d, a]) => `
            <div class="list-item two-line" style="cursor:default">
              <span style="color:var(--cs-${v === "neutral" ? "fg-secondary" : v})">${I(ic)}</span>
              <span class="lines"><span class="strong">${t}</span><span class="xs muted">${d}</span></span>
              <span class="meta">${a}</span>
            </div>`).join("")}</div>
        </section>

        <div class="grid-4 panel" style="display:grid">
          <div class="stat"><span class="label">Dự án</span><span class="stat-value">4</span><span class="xs muted">1 lỗi thời thương hiệu</span></div>
          <div class="stat"><span class="label">Tài nguyên</span><span class="stat-value">8</span><span class="xs"><span style="color:var(--cs-warning)">2 cách ly</span> · <span style="color:var(--cs-danger)">1 thu hồi</span></span></div>
          <div class="stat"><span class="label">Thương hiệu</span><span class="stat-value">v2</span><span class="xs muted">v3 đang là bản nháp</span></div>
          <div class="stat"><span class="label">Dung lượng</span><span class="stat-value">1,2 GB</span>${UI.progress(24, "Dung lượng đã dùng")}<span class="xs muted">trên quota 5 GB</span></div>
        </div>

        <section class="panel" aria-labelledby="proj-h">
          <div class="panel-head">
            <h2 id="proj-h">Dự án</h2>
            <div class="input-affix hide-sm" style="width:220px">${I("search")}<input class="input sm" placeholder="Lọc dự án…" aria-label="Lọc dự án"></div>
            <div class="segmented" aria-label="Kiểu hiển thị"><button aria-pressed="true" aria-label="Danh sách" data-tip="Danh sách">${I("list")}</button><button aria-pressed="false" aria-label="Lưới" data-tip="Lưới">${I("layout-grid")}</button></div>
          </div>
          <div style="overflow-x:auto">
          <table class="table">
            <thead><tr><th class="sortable" aria-sort="none">Tên ${I("sort")}</th><th class="hide-sm">Revision</th><th class="hide-md">Thương hiệu</th><th>Trạng thái</th><th class="sortable hide-sm" aria-sort="descending">Cập nhật ${I("chev-down")}</th><th><span class="sr-only">Hành động</span></th></tr></thead>
            <tbody data-selectable>
              ${D.projects.map((p) => `
                <tr tabindex="0" ondblclick="APP.go('design')">
                  <td><div class="row-3"><span class="proj-thumb" aria-hidden="true">${I(p.type.startsWith("Video") ? "video" : "design")}</span><div class="col" style="gap:0;min-width:0"><a href="#/design" data-go="design" class="strong truncate" style="color:var(--cs-fg-primary)">${p.name}</a><span class="xs muted">${p.type} · ${p.size}</span></div></div></td>
                  <td class="hide-sm"><span class="mono xs">${p.rev}</span></td>
                  <td class="hide-md">${p.brand === "v1" ? `<span class="badge warning" data-tip="Có phiên bản mới hơn: v2" tabindex="0">${I("alert")}Brand v1</span>` : `<span class="xs">Brand ${p.brand}</span>`}</td>
                  <td>${UI.badge(p.stage)}</td>
                  <td class="hide-sm xs muted nowrap">${p.updated}</td>
                  <td class="num"><div class="row-actions row" style="justify-content:flex-end;gap:2px">
                    <button class="btn sm" data-go="design">Mở</button>
                    <button class="btn sm quiet icon" data-menu="project" aria-haspopup="menu" aria-label="Thêm hành động cho ${p.name}" data-tip="Thêm hành động">${I("more")}</button>
                  </div></td>
                </tr>`).join("")}
            </tbody>
          </table></div>
        </section>

        <div class="grid-2" style="align-items:start">
          <section class="panel" aria-labelledby="act-h">
            <div class="panel-head"><h2 id="act-h">Hoạt động gần đây</h2><button class="btn quiet sm" data-go="automation">Nhật ký đầy đủ${I("chev-right")}</button></div>
            <div class="list">${D.audit.slice(0, 5).map((a) => `
              <div class="list-item two-line" style="cursor:default">
                <span style="color:var(--cs-fg-secondary)">${I(a.kind === "agent" ? "bot" : a.kind === "system" ? "cpu" : "user")}</span>
                <span class="lines"><span>${a.actor} <span class="muted">· ${a.op}</span></span><span class="xs muted">${a.target} · ${a.result}</span></span>
                <span class="meta mono">${a.t}</span>
              </div>`).join("")}</div>
          </section>
          <section class="panel" aria-labelledby="health-h">
            <div class="panel-head"><h2 id="health-h">Tình trạng hệ thống</h2><button class="btn quiet sm" data-go="settings" data-settings="caps">Chi tiết${I("chev-right")}</button></div>
            <div class="panel-body"><dl class="kv">
              <dt>Catalog</dt><dd class="row"><span class="dot success"></span>Ổn định · schema v2 · WAL</dd>
              <dt>Sao lưu</dt><dd class="row"><span class="dot success"></span>08:00 hôm nay · 214/214 hash khớp</dd>
              <dt>Worker</dt><dd class="row"><span class="dot info"></span>2 tiến trình · 1 lỗi gần đây</dd>
              <dt>AI</dt><dd class="row"><span class="dot success"></span>gpt-4.1-mini · hỏi trước khi gửi · <a href="#" data-go="settings" data-settings="ai">Cấu hình</a></dd>
              <dt>Social Harness</dt><dd class="row"><span class="dot success"></span>Đã kết nối · localhost:3099</dd>
              <dt>Renderer</dt><dd>PNG, SVG, HTML, MP4 khả dụng · PDF chưa có</dd>
            </dl></div>
          </section>
        </div>
      </div>`;
    }

    return `<div class="page">${header}<div class="page-body">${body}</div></div>`;
  },
});

window.MENUS.project = () => [
  { icon: "design", text: "Mở trong Thiết kế", action: () => APP.go("design") },
  { icon: "writing", text: "Mở trong Soạn thảo", action: () => APP.go("writing") },
  "-",
  { icon: "copy", text: "Nhân bản" },
  { icon: "writing", text: "Đổi tên", kbd: "F2" },
  { icon: "download", text: "Xuất…", action: () => ACTIONS.export() },
  { icon: "package", text: "Xuất gói portable (.lcs)", disabled: "Có 1 tài nguyên bị thu hồi quyền. Thay hoặc gỡ trước khi xuất gói." },
  "-",
  { icon: "folder", text: "Lưu trữ" },
  { icon: "trash", text: "Xóa dự án…", danger: true, action: () => UI.confirm({ title: "Xóa dự án?", body: "<p>Dự án chuyển vào thùng rác 30 ngày. Revision đã bàn giao sang Social vẫn được giữ để đối soát.</p>", confirmLabel: "Xóa dự án", destructive: true, onConfirm: () => UI.toast({ type: "success", title: "Đã chuyển vào thùng rác", actionLabel: "Hoàn tác", action: () => {} }) }) },
];
