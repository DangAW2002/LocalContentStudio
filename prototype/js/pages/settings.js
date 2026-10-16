(function () {
  const TABS = [["general", "settings", "Chung"], ["ai", "sparkles", "AI & mô hình"], ["social", "publishing", "Kết nối Social"], ["storage", "database", "Lưu trữ & sao lưu"], ["caps", "list-checks", "Năng lực hệ thống"], ["plugins", "plug", "Plugin"], ["about", "info", "Giới thiệu"]];
  const tab = () => { const t = JSON.parse(localStorage.getItem("cs.settingsTab") || '"general"'); return TABS.some((x) => x[0] === t) ? t : "general"; };
  const setRow = (label, desc, control) => `<div class="set-row"><div class="col" style="gap:2px;min-width:0"><span class="strong">${label}</span>${desc ? `<span class="xs muted">${desc}</span>` : ""}</div><div class="set-control">${control}</div></div>`;

  PAGES.register({
    id: "settings",
    label: "Cài đặt",
    sidebar() {
      return { title: "Cài đặt", body: `<div class="list">${TABS.map(([k, ic, t]) => `<button class="list-item" data-stab="${k}" ${tab() === k ? 'aria-current="page"' : ""}>${I(ic)}<span>${t}</span>${k === "plugins" ? `<span class="meta"><span class="badge warning" style="height:18px">1</span></span>` : ""}</button>`).join("")}</div>` };
    },
    main({ st, D }) {
      const t = tab();
      const title = TABS.find((x) => x[0] === t)[2];
      const general = `<section class="panel set-group">
          ${setRow("Ngôn ngữ", "Ngôn ngữ giao diện và định dạng ngày, số", `<select class="select sm" style="width:200px"><option>Tiếng Việt</option><option>English</option></select>`)}
          ${setRow("Giao diện", "", `<div class="segmented" data-exclusive><button aria-pressed="${APP.theme === "light"}" data-action="theme-light">${I("sun")}Sáng</button><button aria-pressed="${APP.theme === "dark"}" data-action="theme-dark">${I("moon")}Tối</button><button aria-pressed="false">${I("contrast")}Theo hệ thống</button></div>`)}
          ${setRow("Mật độ", "Gọn giảm khoảng cách trong bảng và danh sách", `<div class="segmented" data-exclusive><button aria-pressed="true">Tiêu chuẩn</button><button aria-pressed="false">Gọn</button></div>`)}
          ${setRow("Giảm chuyển động", "Mặc định theo cài đặt Windows", `<label class="switch"><input type="checkbox"><span class="sr-only">Giảm chuyển động</span></label>`)}
          ${setRow("Mở lại dự án gần nhất khi khởi động", "", `<label class="switch"><input type="checkbox" checked><span class="sr-only">Mở lại dự án</span></label>`)}
          ${setRow("Phím tắt", "Không dùng Ctrl+Shift để tránh xung đột bộ gõ tiếng Việt", `<button class="btn sm" data-action="shortcuts">${I("keyboard")}Xem phím tắt</button>`)}
        </section>`;
      const ai = `<div class="col-4">
          <div class="grid-3 mode-cards" data-exclusive role="radiogroup" aria-label="Chế độ AI">
            <button class="np-type" role="radio" aria-pressed="${st === "blocked"}" aria-checked="${st === "blocked"}">${I("x-circle", "icon-lg")}<span class="strong">Không dùng AI</span><span class="xs muted">Mọi chức năng thủ công vẫn đầy đủ</span></button>
            <button class="np-type" role="radio" aria-pressed="${st !== "blocked"}" aria-checked="${st !== "blocked"}">${I("cloud", "icon-lg")}<span class="strong">API (khóa riêng)</span><span class="xs muted">OpenAI-compatible · hỏi trước khi gửi</span></button>
            <button class="np-type" role="radio" aria-pressed="false" data-tip="Không tìm thấy Ollama hoặc LM Studio">${I("cpu", "icon-lg")}<span class="strong">Mô hình cục bộ</span><span class="xs muted">Dữ liệu không rời máy</span>${UI.badge("CAP_MISSING")}</button>
          </div>
          <section class="panel set-group"><div class="panel-head"><h2>Nhà cung cấp API</h2>${st === "error" ? UI.chipBadge("danger", "x-circle", "Kết nối thất bại") : UI.chipBadge("success", "check", "Kết nối thử 22:50")}</div>
            ${st === "error" ? `<div style="padding:12px 16px 0">${UI.callout("danger", "key", "Khóa API bị từ chối (401)", "Khóa có thể đã hết hạn hoặc bị thu hồi. Các tác vụ AI đang chờ đã được hủy; bản nháp không bị ảnh hưởng.", `<button class="btn sm primary">Thay khóa</button><button class="btn sm">Chuyển sang không dùng AI</button>`)}</div>` : ""}
            ${setRow("Tên hiển thị", "", `<input class="input sm" value="OpenAI" style="width:240px">`)}
            ${setRow("Địa chỉ API", "", `<input class="input sm mono" value="https://api.openai.com/v1" style="width:240px">`)}
            ${setRow("Khóa API", "Lưu trong Windows Credential Manager. Không ghi vào dự án, log, bản xuất hay gói hỗ trợ.", `<div class="row"><input class="input sm mono" value="sk-••••••••••••3f9a" readonly style="width:150px" aria-label="Khóa API đã lưu"><button class="btn sm">Thay</button><button class="btn sm danger-outline">Xóa</button></div>`)}
            ${setRow("Mô hình", "", `<select class="select sm" style="width:240px"><option>gpt-4.1-mini</option><option>gpt-4.1</option></select>`)}
            ${setRow("Thời gian chờ", "", `<select class="select sm" style="width:240px"><option>60 giây</option><option>120 giây</option></select>`)}
            ${setRow("Giới hạn chi phí", "Tác vụ dừng khi chạm giới hạn tháng", `<div class="row"><input class="input sm" value="5,00" style="width:90px" aria-label="Giới hạn USD"><span class="xs muted">USD / tháng · đã dùng 0,28</span></div>`)}
            ${setRow("Hỏi trước khi gửi dữ liệu", "Liệt kê dữ liệu sẽ gửi và chi phí ước tính cho từng tác vụ", `<label class="switch"><input type="checkbox" checked><span class="sr-only">Hỏi trước khi gửi</span></label>`)}
            <div class="set-row"><span class="spacer"></span><button class="btn sm">${I("activity")}Kiểm tra kết nối</button></div>
          </section></div>`;
      const social = `<section class="panel set-group"><div class="panel-head"><h2>Social Harness</h2>${st === "blocked" ? UI.badge("NOT_CONNECTED") : UI.chipBadge("success", "check", "Đã kết nối · 42 ms")}</div>
          ${setRow("Địa chỉ", "Social Harness chạy trên máy này", `<input class="input sm mono" value="http://127.0.0.1:3099" style="width:240px">`)}
          ${setRow("Workspace Social tương ứng", "Liên kết tường minh; Studio không tự đoán", `<select class="select sm" style="width:240px"><option>Local Studio Marketing</option></select>`)}
          ${setRow("Page có thể nhận gói", "Đọc từ Social lúc 23:12", `<div class="col xs" style="align-items:flex-end"><span>Fanpage Local Studio · Facebook</span><span>@localstudio · Instagram</span><span>Local Studio Company · LinkedIn</span></div>`)}
          <div class="set-row"><span class="xs muted">Studio chỉ gửi gói bất biến và đọc trạng thái. Duyệt, lên lịch và đăng thuộc quyền Social.</span><span class="spacer"></span><button class="btn sm">${I("activity")}Kiểm tra</button><button class="btn sm danger-outline">Ngắt kết nối</button></div>
        </section>`;
      const storage = `<div class="col-4"><section class="panel set-group">
          ${setRow("Thư mục workspace", "", `<div class="input-group" style="width:320px"><input class="input sm mono" readonly value="${D.workspace.path}"><button class="btn sm">${I("folder-open")}</button></div>`)}
          ${setRow("Catalog", "SQLite · WAL · schema v2", `<span class="row"><span class="dot success"></span>Toàn vẹn · kiểm lúc 08:00 <button class="btn sm">Kiểm tra ngay</button></span>`)}
          ${setRow("Tệp gốc và bản xuất", "Lưu theo SHA-256, không trùng lặp", `<span>1,2 GB / quota 5 GB</span>`)}
          ${setRow("Bộ đệm xem trước", "Có thể tạo lại bất cứ lúc nào", `<span class="row">1,2 GB <button class="btn sm">Dọn</button></span>`)}
          ${setRow("Sao lưu tự động", "", `<select class="select sm" style="width:200px"><option>Hằng ngày lúc 08:00</option><option>Mỗi 6 giờ</option><option>Tắt</option></select>`)}
        </section>
        <section class="panel"><div class="panel-head"><h2>Bản sao lưu</h2><button class="btn sm primary">${I("history")}Sao lưu ngay</button></div>
          <table class="table"><thead><tr><th>Thời điểm</th><th class="num">Dung lượng</th><th>Kiểm tra</th><th></th></tr></thead><tbody>
            ${[["Hôm nay 08:00", "1,18 GB", true], ["Hôm qua 08:00", "1,15 GB", true], ["12/10 08:00", "1,02 GB", true]].map(([t, s, ok]) => `<tr><td>${t}</td><td class="num">${s}</td><td><span class="badge success">${I("check")}214/214 hash khớp</span></td><td class="num"><div class="row-actions row" style="justify-content:flex-end"><button class="btn sm" data-action="restore">Khôi phục…</button></div></td></tr>`).join("")}
          </tbody></table></section></div>`;
      const caps = `${UI.callout("neutral", "info", "Năng lực dựa trên bằng chứng chạy thật", "Giao diện chỉ hiện thao tác khả dụng. Mục chưa kiểm chứng hoặc bị chặn luôn kèm lý do và cách xử lý.")}<div style="height:12px"></div>
        <section class="panel" style="overflow-x:auto"><table class="table"><thead><tr><th>Năng lực</th><th>Trạng thái</th><th>Bằng chứng</th><th></th></tr></thead><tbody>
          ${D.capabilities.map((c) => `<tr><td><div class="col" style="gap:0"><span class="strong">${c.name}</span><span class="mono xs muted">${c.key}</span></div></td><td>${UI.badge(c.state)}</td><td class="xs">${c.evidence}</td><td class="num"><button class="btn sm quiet" data-tip="Chạy lại kiểm tra năng lực này">${I("refresh")}Kiểm tra lại</button></td></tr>`).join("")}
        </tbody></table></section>`;
      const plugins = `${UI.callout("warning", "shield-alert", "Plugin bên thứ ba đang bị cách ly", "Studio chưa có sandbox tiến trình đủ an toàn. Plugin ngoài được nhập nhưng không chạy cho tới khi tính năng này sẵn sàng.")}<div style="height:12px"></div>
        <section class="panel"><table class="table"><thead><tr><th>Plugin</th><th>Quyền yêu cầu</th><th>Trạng thái</th><th></th></tr></thead><tbody>
          <tr><td><div class="col" style="gap:0"><span class="strong">Xuất PPTX</span><span class="xs muted">community.pptx-export 0.2.0</span></div></td><td class="xs">Đọc tài liệu, ghi tệp vào thư mục xuất</td><td>${UI.badge("QUARANTINED", { label: "Cách ly", tip: "Chưa chạy được: thiếu sandbox" })}</td><td class="num"><button class="btn sm danger-outline">Gỡ</button></td></tr>
          <tr><td><div class="col" style="gap:0"><span class="strong">Bộ xuất PNG/SVG/HTML</span><span class="xs muted">Có sẵn · studio 0.1.0</span></div></td><td class="xs">Nội bộ</td><td>${UI.badge("CAP_VERIFIED", { label: "Đang chạy" })}</td><td></td></tr>
        </tbody></table></section>`;
      const about = `<section class="panel set-group">
          ${setRow("Phiên bản", "Kênh ổn định", `<span class="mono">0.1.0 (2026-10-14)</span>`)}
          ${setRow("Cập nhật", "Dữ liệu, khóa và bản sao lưu được giữ qua mỗi lần cập nhật", `<button class="btn sm">Kiểm tra cập nhật</button>`)}
          ${setRow("Giấy phép mã nguồn mở", "", `<button class="btn sm">Xem danh sách</button>`)}
          ${setRow("Gói hỗ trợ", "Log và cấu hình đã lọc bỏ khóa, đường dẫn riêng", `<button class="btn sm">${I("package")}Tạo gói hỗ trợ</button>`)}
        </section>`;
      const body = st === "loading" ? UI.skeletonRows(6) : { general, ai, social, storage, caps, plugins, about }[t];
      return `<div class="page">${UI.pageHeader({ title, meta: `Cài đặt ${UI.metaSep} lưu tự động` })}<div class="page-body"><div style="max-width:880px" class="col-4">${body}</div></div></div>`;
    },
    mount(root) {
      root.querySelectorAll("[data-stab]").forEach((b) => b.addEventListener("click", () => { localStorage.setItem("cs.settingsTab", JSON.stringify(b.dataset.stab)); APP.rerender(); }));
    },
  });
  ACTIONS["theme-light"] = () => { if (APP.theme !== "light") ACTIONS["toggle-theme"](); };
  ACTIONS["theme-dark"] = () => { if (APP.theme !== "dark") ACTIONS["toggle-theme"](); };
  ACTIONS.restore = () => UI.confirm({
    title: "Khôi phục bản sao lưu?",
    body: `<p>Studio sẽ <strong>tạo bản sao lưu hiện trạng trước</strong>, rồi khôi phục catalog và tệp từ bản đã chọn. Thay đổi sau thời điểm đó sẽ nằm trong bản sao lưu hiện trạng, không bị mất.</p>`,
    confirmLabel: "Sao lưu hiện trạng rồi khôi phục",
    onConfirm: () => UI.toast({ type: "info", title: "Đang khôi phục…", desc: "Theo dõi trong Task Center." }),
  });
})();
