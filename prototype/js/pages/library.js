PAGES.register({
  id: "library",
  label: "Thư viện",
  sidebar({ D }) {
    const cnt = (k) => D.assets.filter((a) => a.state === k).length;
    const grp = (title, items) => `<div class="list-group-label section-label">${title}</div><div class="list" data-selectable>${items.map(([ic, t, n, sel, color]) => `<button class="list-item ${sel ? "is-selected" : ""}" aria-pressed="${!!sel}">${I(ic)}<span style="${color ? `color:var(--cs-${color})` : ""}">${t}</span><span class="meta">${n}</span></button>`).join("")}</div>`;
    return {
      title: "Bộ lọc",
      actions: `<button class="btn quiet sm" data-tip="Bỏ mọi bộ lọc">Đặt lại</button>`,
      body:
        grp("Phạm vi", [["layers", "Tất cả", D.assets.length, true], ["folder", "Workspace: Marketing Q4", 5], ["globe", "Toàn cục", 3]]) +
        grp("Quyền sử dụng", [["shield-alert", "Đang cách ly", cnt("QUARANTINED"), false, "warning"], ["shield-check", "Đủ quyền dùng", cnt("RIGHTS_READY")], ["x-circle", "Đã thu hồi", cnt("REVOKED"), false, "danger"]]) +
        grp("Loại", [["image", "Ảnh", 4], ["video", "Video", 1], ["audio", "Âm thanh", 1], ["file", "Vector", 2]]) +
        `<div class="list-group-label section-label">Thẻ</div><div class="row wrap" style="padding:0 12px 12px">${["q4", "sản-phẩm", "logo", "đội-ngũ", "biểu-đồ"].map((t) => `<button class="chip" style="cursor:pointer">${I("tag")}${t}</button>`).join("")}</div>`,
      foot: `<div class="col" style="gap:4px"><div class="row xs"><span class="muted">Dung lượng</span><span class="spacer"></span><span>1,2 GB / 5 GB</span></div>${UI.progress(24, "Quota")}</div>`,
    };
  },
  main({ st, D }) {
    const header = UI.pageHeader({
      title: "Thư viện tài nguyên",
      meta: `${D.assets.length} tệp ${UI.metaSep} 1,2 GB ${UI.metaSep} chống trùng theo SHA-256`,
      actions: `<button class="btn" data-tip="Liên kết tới tệp nằm ngoài Studio" data-tip-sub="Studio theo dõi thay đổi và báo khi tệp bị di chuyển">${I("link")}Liên kết tệp ngoài</button>
        <button class="btn primary" data-action="upload" data-tip="Nhập tệp từ máy" data-tip-sub="Tệp mới luôn vào trạng thái cách ly">${I("upload")}Nhập tệp</button>`,
    });
    const toolbar = `<div class="page-toolbar">
        <div class="input-affix" style="flex:1 1 220px;max-width:360px">${I("search")}<input class="input sm" placeholder="Tìm theo tên, nguồn, giấy phép…" aria-label="Tìm tài nguyên"></div>
        <select class="select sm" style="width:auto" aria-label="Sắp xếp"><option>Mới nhập trước</option><option>Tên A–Z</option><option>Dung lượng</option><option>Dùng nhiều nhất</option></select>
        <span class="spacer"></span>
        <span class="xs muted hide-sm" id="lib-sel">1 tệp đã chọn</span>
        <div class="segmented" aria-label="Kiểu hiển thị"><button aria-pressed="true" aria-label="Lưới" data-tip="Lưới">${I("layout-grid")}</button><button aria-pressed="false" aria-label="Danh sách" data-tip="Danh sách">${I("list")}</button></div>
      </div>`;
    const card = (a, sel) => `
      <div class="asset-card ${sel ? "is-selected" : ""}" tabindex="0" role="option" aria-selected="${sel}" aria-label="${a.name}, ${UI.status[a.state][2]}">
        <div class="thumb" style="aspect-ratio:4/3;${a.kind === "Ảnh" || a.kind === "Video" ? `background:hsl(${a.tone} 40% 72%)` : ""}">
          ${I(a.kind === "Âm thanh" ? "audio" : a.kind === "Video" ? "video" : a.kind === "Vector" ? "file" : "image", "icon-xl")}
          <span class="thumb-tag">${a.ext}</span>
          ${a.state !== "RIGHTS_READY" ? `<span class="thumb-state ${a.state === "REVOKED" ? "danger" : "warning"}">${I(a.state === "REVOKED" ? "x-circle" : "shield-alert")}</span>` : ""}
          <div class="card-hover">
            ${a.state === "QUARANTINED" ? `<button class="btn sm primary" data-action="rights-review" data-asset="${a.id}">${I("shield-check")}Xác nhận quyền</button>`
              : a.state === "REVOKED" ? `<button class="btn sm">${I("refresh")}Thay tệp</button>`
              : `<button class="btn sm" data-go="design" data-tip="Chèn vào trang đang mở">${I("plus")}Dùng trong thiết kế</button>`}
          </div>
        </div>
        <div class="col" style="padding:8px 10px;gap:4px">
          <span class="truncate strong">${a.name}</span>
          <div class="row" style="justify-content:space-between"><span class="xs muted truncate">${a.dims} · ${a.size}</span>${a.scope === "Global" ? `<span data-tip="Thư viện toàn cục" tabindex="0" class="muted">${I("globe")}</span>` : ""}</div>
          ${UI.badge(a.state)}
        </div>
      </div>`;
    const body = UI.byState(st, {
      loading: () => `<div class="asset-grid">${Array.from({ length: 8 }, () => `<div class="skeleton" style="aspect-ratio:4/3.8"></div>`).join("")}</div>`,
      empty: () => `<div class="dropzone" style="min-height:360px;justify-content:center">${I("upload", "icon-xl")}<h2 style="color:var(--cs-fg-primary)">Thư viện còn trống</h2><span>Kéo tệp vào đây, hoặc nhập từ nghiên cứu. Tệp mới luôn vào trạng thái cách ly cho tới khi bạn xác nhận quyền.</span><div class="row"><button class="btn primary" data-action="upload">${I("upload")}Nhập tệp</button><button class="btn" data-go="research">${I("globe")}Lấy từ nghiên cứu</button></div></div>`,
      error: () => `${UI.callout("danger", "file", "Không tìm thấy tệp gốc của “khach-hang-noi.mp4”", "Tệp liên kết đã bị di chuyển hoặc xóa khỏi D:\\Video\\. Các bản xuất cũ vẫn còn; bản xuất mới dùng tệp này sẽ bị chặn.", `<button class="btn sm primary">${I("link")}Tìm lại tệp…</button><button class="btn sm">Thay bằng tệp khác</button><button class="btn sm">Gỡ khỏi dự án</button>`)}<div style="height:16px"></div><div class="asset-grid" role="listbox" aria-label="Tài nguyên">${D.assets.map((a, i) => card(a, i === 1)).join("")}</div>`,
      blocked: () => `${UI.callout("danger", "hard-drive", "Đã vượt quota workspace (5,0 / 5 GB)", "Không thể nhập thêm tệp. Xóa tệp không dùng, dọn bộ đệm hoặc tăng quota trong Cài đặt.", `<button class="btn sm">Xem tệp không được dùng (3)</button><button class="btn sm" data-go="settings" data-settings="storage">Mở cài đặt lưu trữ</button>`)}<div style="height:16px"></div><div class="asset-grid" role="listbox" aria-label="Tài nguyên">${D.assets.map((a, i) => card(a, i === 1)).join("")}</div>`,
      normal: () => `${UI.callout("warning", "shield-alert", "2 tệp đang cách ly", "Các tệp này chưa dùng được để xuất hoặc bàn giao cho tới khi bạn xác nhận nguồn và giấy phép.", `<button class="btn sm" data-action="rights-review" data-asset="as-2">Duyệt lần lượt</button>`)}
        <div style="height:16px"></div>
        <div class="asset-grid" role="listbox" aria-label="Tài nguyên" data-selectable>${D.assets.map((a, i) => card(a, i === 1)).join("")}</div>`,
    });
    return `<div class="page">${header}${toolbar}<div class="page-body">${body}</div></div>`;
  },
  inspector({ st, D }) {
    if (st === "empty") return null;
    const a = D.assets[1];
    if (st === "loading") return { title: "Chi tiết", body: `<div style="padding:12px">${UI.skeletonRows(1, 160)}<div style="height:12px"></div>${UI.skeletonText(4)}</div>` };
    return {
      title: "Chi tiết tài nguyên",
      actions: `<button class="btn quiet sm icon" data-menu="asset" aria-haspopup="menu" aria-label="Thêm hành động" data-tip="Thêm hành động">${I("more")}</button>`,
      body: `
        <div class="thumb" style="aspect-ratio:4/3;background:hsl(${a.tone} 40% 72%)">${I("image", "icon-xl")}</div>
        <div class="col" style="padding:12px;gap:8px">
          <strong class="md" style="overflow-wrap:anywhere">${a.name}</strong>
          ${UI.badge(a.state)}
          <button class="btn primary block" data-action="rights-review" data-asset="${a.id}">${I("shield-check")}Xác nhận quyền sử dụng</button>
          <div class="unavailable"><button class="btn block" disabled>${I("plus")}Dùng trong thiết kế</button><span class="reason">${I("lock")}Cần xác nhận quyền trước</span></div>
        </div>
        <details class="insp-section" open><summary>${I("chev-right", "chev")}Thông tin</summary><div class="insp-content"><dl class="kv xs">
          <dt>Định dạng</dt><dd>${a.ext} · ${a.dims}</dd><dt>Dung lượng</dt><dd>${a.size}</dd><dt>SHA-256</dt><dd><span class="hash copyable" data-copy="${a.hash}" data-tip="Nhấn để sao chép">${a.hash}…e91c</span></dd>
          <dt>Chữ ký tệp</dt><dd>Khớp JPEG</dd><dt>Phạm vi</dt><dd>Workspace</dd><dt>Nhập lúc</dt><dd>Hôm nay 22:31</dd></dl></div></details>
        <details class="insp-section" open><summary>${I("chev-right", "chev")}Nguồn & quyền</summary><div class="insp-content"><dl class="kv xs">
          <dt>Nguồn</dt><dd class="muted">Chưa xác nhận</dd><dt>Giấy phép</dt><dd class="muted">Chưa xác nhận</dd><dt>Ghi công</dt><dd class="muted">—</dd><dt>Cho phép</dt><dd class="muted">—</dd></dl></div></details>
        <details class="insp-section"><summary>${I("chev-right", "chev")}Đang dùng ở<span class="spacer"></span><span class="badge">0</span></summary><div class="insp-content xs muted">Chưa được dùng trong dự án nào.</div></details>
        <details class="insp-section"><summary>${I("chev-right", "chev")}Lịch sử</summary><div class="insp-content xs"><div>22:31 · Bạn nhập tệp</div><div>22:31 · Hệ thống: kiểm chữ ký, băm, cách ly</div><div>22:57 · Claude Code (CLI) cố xác nhận quyền · <span style="color:var(--cs-danger)">bị từ chối</span></div></div></details>
        <details class="insp-section"><summary>${I("chev-right", "chev")}Khác</summary><div class="insp-content">
          <button class="btn sm block" data-tip="Tạo bản ghi mới ở phạm vi kia" data-tip-sub="Quyền sử dụng được đặt lại">${I("globe")}Sao chép sang thư viện toàn cục</button>
          <button class="btn sm block danger-outline" data-action="asset-delete">${I("trash")}Xóa tệp…</button></div></details>`,
    };
  },
});

window.MENUS.asset = () => [
  { icon: "eye", text: "Xem toàn màn hình" },
  { icon: "folder-open", text: "Mở thư mục chứa" },
  { icon: "copy", text: "Sao chép SHA-256" },
  "-",
  { icon: "x-circle", text: "Thu hồi quyền…", disabled: "Tệp chưa được cấp quyền" },
  { icon: "trash", text: "Xóa…", danger: true, action: () => ACTIONS["asset-delete"]() },
];
ACTIONS["asset-delete"] = () => UI.confirm({
  title: "Xóa “anh-san-pham-moi.jpg”?",
  body: `<p>Tệp không được dùng trong revision nào nên có thể xóa. Bản gốc được giữ trong thùng rác 30 ngày.</p><p class="xs muted">Tệp đã ghim trong revision hoặc gói bàn giao sẽ không xóa được; khi đó nút xóa bị khóa và nêu lý do.</p>`,
  confirmLabel: "Xóa tệp", destructive: true,
  onConfirm: () => UI.toast({ type: "success", title: "Đã chuyển vào thùng rác", actionLabel: "Hoàn tác", action: () => {} }),
});
