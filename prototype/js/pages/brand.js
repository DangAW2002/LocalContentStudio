PAGES.register({
  id: "brand",
  label: "Thương hiệu",
  sidebar({ st, D }) {
    if (st === "empty") return null;
    return {
      title: "Phiên bản",
      actions: `<button class="btn quiet sm icon" aria-label="Tạo bản nháp mới" data-tip="Tạo bản nháp từ v2">${I("plus")}</button>`,
      body: `<div style="padding:8px"><label class="label" for="bk">Bộ nhận diện</label><select class="select sm" id="bk"><option>Local Studio</option><option>Sản phẩm Automation</option></select></div>
        <div class="list" data-selectable>${D.brandVersions.map((v, i) => `
          <button class="list-item two-line ${i === 0 ? "is-selected" : ""}">
            <span style="color:var(--cs-${v.state === "DRAFT" ? "fg-secondary" : "success"})">${I(v.state === "DRAFT" ? "writing" : "check-circle")}</span>
            <span class="lines"><span><strong>${v.v}</strong> · ${UI.status[v.state][2]}${v.current ? ` <span class="badge info" style="height:18px">Mặc định</span>` : ""}</span><span class="xs muted">${v.date} · ${v.note}</span></span>
          </button>`).join("")}</div>`,
      foot: `<button class="btn block" data-tip="Tạo bản nháp từ hồ sơ phong cách của Page trong Social Harness" data-tip-sub="Là bản sao, không đồng bộ hai chiều">${I("import")}Nhập từ Social…</button>`,
    };
  },
  main({ st, D }) {
    const header = UI.pageHeader({
      title: "Thương hiệu · Local Studio",
      meta: `Đang xem ${UI.badge("DRAFT", { label: "v3 · bản nháp" })} ${UI.metaSep} dựa trên v2 ${UI.metaSep} tự lưu 23:05`,
      actions: st === "empty" ? "" : `<button class="btn" data-tip="So sánh từng thay đổi với v2">${I("git-compare")}So sánh với v2</button>
        <button class="btn" data-tip="Chạy lại bộ kiểm tra tương phản, phông, từ cấm">${I("list-checks")}Kiểm tra</button>
        ${st === "blocked" ? `<div class="unavailable"><button class="btn primary" disabled>${I("check")}Phát hành v3</button><span class="reason">${I("lock")}Thiếu tệp phông · <a href="#">Khắc phục</a></span></div>` : `<button class="btn primary" data-action="brand-publish" data-tip="Phát hành thành phiên bản bất biến" data-tip-sub="Chỉ người dùng; dự án không tự đổi sang v3">${I("check")}Phát hành v3</button>`}`,
    });
    const swatches = `<div class="panel-body col-3">
        <div style="overflow-x:auto"><table class="table"><thead><tr><th>Màu</th><th>Tên</th><th class="hide-sm">Token</th><th>Mã</th><th data-tip="So với màu Nền (#FFFFFF). Văn bản thường cần ≥ 4.5:1" tabindex="0">Tương phản ${I("info")}</th><th><span class="sr-only">Hành động</span></th></tr></thead><tbody>
        ${D.brandColors.map((c) => `<tr><td><span class="swatch" style="background:${c.hex}"></span></td><td class="strong">${c.name}</td><td class="hide-sm mono xs muted">${c.token}</td>
          <td><input class="input sm mono" value="${c.hex}" style="width:100px" aria-label="Mã màu ${c.name}"></td>
          <td>${c.warn ? `<span class="badge warning" data-tip="Không đạt cho chữ thường" data-tip-sub="Chỉ dùng cho nền, khối hoặc chữ ≥ 24px" tabindex="0">${I("alert")}${c.contrast}</span>` : c.contrast === "—" ? `<span class="muted">—</span>` : `<span class="badge success">${I("check")}${c.contrast}</span>`}</td>
          <td class="num"><div class="row-actions"><button class="btn sm quiet icon" aria-label="Xóa màu ${c.name}" data-tip="Xóa màu">${I("trash")}</button></div></td></tr>`).join("")}
        </tbody></table></div>
        <div><button class="btn sm">${I("plus")}Thêm màu</button></div></div>`;
    const type = `<div class="panel-body col-3">
        <div class="grid-2">
          <div class="field"><label>Phông tiêu đề</label><select class="select"><option>Be Vietnam Pro · Bold</option><option>Inter · Bold</option></select>
            <span class="error-text">${I("alert-circle")}Chưa cài trên máy này · <a href="#">Cài từ tệp</a> · <a href="#">Dùng Inter thay thế</a></span></div>
          <div class="field"><label>Phông nội dung</label><select class="select"><option>Inter · Regular</option></select><span class="hint">${I("check")} Có sẵn · hỗ trợ đủ dấu tiếng Việt</span></div>
        </div>
        <div class="type-scale">${[["Tiêu đề lớn", 56, 700], ["Tiêu đề", 36, 700], ["Tiêu đề phụ", 24, 600], ["Nội dung", 18, 400], ["Chú thích", 14, 400]].map(([n, s, w]) => `<div class="row-3"><span class="xs muted" style="width:96px">${n} · ${s}px</span><span style="font-size:${Math.min(s, 40)}px;font-weight:${w};line-height:1.2" class="truncate">Sáng tạo nhất quán</span></div>`).join("")}</div>
      </div>`;
    const logo = `<div class="panel-body grid-3">${[["Logo ngang", "logo-ngang.svg", "Tối thiểu 120 px · khoảng trống 0,5×"], ["Biểu tượng", "logo-icon.svg", "Tối thiểu 32 px"], ["Logo trên nền tối", "", "Chưa có tệp"]].map(([t, f, r]) => `
        <div class="asset-card"><div class="thumb" style="aspect-ratio:16/9">${f ? `<span class="logo" style="font-size:22px;font-weight:800;color:#334BD1">LS</span>` : I("plus", "icon-xl")}</div>
        <div class="col" style="padding:8px 10px;gap:2px"><span class="strong">${t}</span><span class="xs muted">${f || "—"}</span><span class="xs muted">${r}</span></div>
        <div style="padding:0 10px 10px"><button class="btn sm block">${f ? "Thay tệp" : `${I("upload")}Chọn từ thư viện`}</button></div></div>`).join("")}</div>`;
    const voice = `<div class="panel-body col-4">
        <div class="field"><label for="tone">Giọng điệu</label><textarea class="textarea" id="tone" rows="2">Chuyên nghiệp, rõ ràng, khích lệ hành động. Xưng “chúng tôi”, gọi người đọc là “bạn”.</textarea></div>
        <div class="grid-2"><div class="field"><label>Nên</label><textarea class="textarea" rows="3">Dùng số liệu có nguồn&#10;Câu ngắn, động từ mạnh&#10;Một lời kêu gọi mỗi bài</textarea></div>
          <div class="field"><label>Tránh</label><textarea class="textarea" rows="3">Phóng đại không có căn cứ&#10;Viết hoa toàn bộ câu&#10;Quá 3 emoji mỗi bài</textarea></div></div>
        <div class="field"><label>Từ cấm <span class="muted">(kiểm tra tự động khi soạn thảo và trước khi bàn giao)</span></label>
          <div class="row wrap">${["cam kết 100%", "chữa dứt điểm", "rẻ nhất thị trường", "duy nhất"].map((w) => `<span class="chip" style="border-color:var(--cs-danger);color:var(--cs-danger)">${w}<button aria-label="Bỏ “${w}”" data-tip="Bỏ từ này">${I("x")}</button></span>`).join("")}
          <input class="input sm" placeholder="Thêm từ, nhấn Enter" style="width:180px" aria-label="Thêm từ cấm"></div></div>
      </div>`;
    const channels = `<div style="overflow-x:auto"><table class="table"><thead><tr><th>Kênh</th><th>Độ dài caption</th><th>Hashtag</th><th>Khổ ưu tiên</th><th>Vùng an toàn</th></tr></thead><tbody>
        ${[["Facebook", "≤ 300 ký tự (khuyến nghị)", "0–3", "1080 × 1350", "Không"], ["Instagram", "≤ 2.200 ký tự", "3–8 (tối đa 30)", "1080 × 1350", "Lề dưới 14%"], ["LinkedIn", "≤ 3.000 ký tự", "0–3", "1080 × 1080", "Không"], ["TikTok", "≤ 2.200 ký tự", "3–5", "1080 × 1920", "Phải 12%, dưới 20%"]].map((r) => `<tr>${r.map((c, i) => `<td class="${i ? "" : "strong"}">${c}</td>`).join("")}</tr>`).join("")}
      </tbody></table><div class="xs muted" style="padding:12px 16px">Giới hạn nền tảng có thể thay đổi; đây là ước lượng, không bảo đảm hiển thị thực tế.</div></div>`;
    const diff = `<div class="list">${[["+", "Thêm màu “Nhấn” #F2A93B"], ["~", "Giọng điệu CTA: “Đăng ký ngay” → “Dùng thử miễn phí”"], ["+", "Thêm từ cấm “duy nhất”"], ["~", "Phông tiêu đề: Inter → Be Vietnam Pro"]].map(([k, t]) => `
        <div class="list-item" style="cursor:default"><span class="mono strong" style="width:16px;color:var(--cs-${k === "+" ? "success" : "warning"})">${k}</span><span>${t}</span></div>`).join("")}</div>`;

    const body = UI.byState(st, {
      loading: () => `${UI.skeletonRows(1, 38)}<div style="height:12px"></div>${UI.skeletonRows(5)}`,
      empty: () => `<div class="panel">${UI.empty({ icon: "brand", title: "Chưa có bộ nhận diện thương hiệu", desc: "Thương hiệu giúp giữ màu, phông, giọng điệu và từ cần tránh nhất quán trên mọi dự án. Dự án vẫn tạo được khi chưa có.", actions: `<button class="btn primary">${I("plus")}Tạo bộ nhận diện</button><button class="btn">${I("import")}Nhập từ Social Harness</button><button class="btn">${I("file-plus")}Nhập tệp .brand.json</button>` })}</div>`,
      error: () => `${UI.callout("danger", "git-compare", "Bản nháp v3 vừa được sửa ở cửa sổ khác", "Thay đổi của bạn chưa bị ghi đè. Hãy so sánh rồi chọn giữ bản nào.", `<button class="btn sm primary">${I("git-compare")}So sánh hai bản</button><button class="btn sm">Tải lại bản mới nhất</button>`)}<div style="height:16px"></div>${tabs()}`,
      blocked: () => `${UI.callout("warning", "type", "Chưa thể phát hành v3", "Phông “Be Vietnam Pro” chưa có tệp trên máy. Phiên bản phát hành phải kèm tệp phông để bản xuất giống nhau trên mọi máy.", `<button class="btn sm primary">${I("upload")}Thêm tệp phông</button><button class="btn sm">Dùng Inter thay thế</button>`)}<div style="height:16px"></div>${tabs()}`,
      normal: () => tabs(),
    });
    function tabs() {
      return `<section class="panel">${UI.tabs("bt", [{ label: "Màu sắc", icon: "brand" }, { label: "Chữ", icon: "type" }, { label: "Logo", icon: "image" }, { label: "Giọng điệu", icon: "quote" }, { label: "Kênh", icon: "publishing" }, { label: "Thay đổi so với v2", icon: "git-compare", count: 4 }])}
        ${UI.tabPanel("bt", 0, swatches)}${UI.tabPanel("bt", 1, type)}${UI.tabPanel("bt", 2, logo)}${UI.tabPanel("bt", 3, voice)}${UI.tabPanel("bt", 4, channels)}${UI.tabPanel("bt", 5, diff)}</section>`;
    }
    return `<div class="page">${header}<div class="page-body"><div class="col-4">${body}</div></div></div>`;
  },
  inspector({ st }) {
    if (st === "empty" || st === "loading") return null;
    const row = (v, t, d) => `<div class="row" style="align-items:flex-start;gap:8px"><span style="color:var(--cs-${v})">${I(v === "success" ? "check-circle" : v === "warning" ? "alert" : "x-circle")}</span><div class="col" style="gap:0"><span>${t}</span><span class="xs muted">${d}</span></div></div>`;
    return {
      title: "Kiểm tra thương hiệu",
      body: `<div class="insp-content" style="padding-top:12px">
          ${row("warning", "Tương phản: 1 cảnh báo", "Màu Nhấn chỉ đạt 2.1:1 trên nền trắng.")}
          ${row(st === "blocked" ? "danger" : "warning", "Phông: thiếu 1 tệp", "Be Vietnam Pro chưa có trên máy.")}
          ${row("success", "Từ cấm: không vi phạm", "Đã quét 4 dự án, 11 biến thể.")}
          ${row("success", "Logo: đủ biến thể bắt buộc", "Thiếu logo nền tối (không bắt buộc).")}
        </div>
        <details class="insp-section" open><summary>${I("chev-right", "chev")}Dự án đang ghim</summary><div class="insp-content xs">
          <div class="row"><span class="truncate">Ra mắt gói tự động hóa Q4</span><span class="spacer"></span><span class="muted">v2</span></div>
          <div class="row"><span class="truncate">Poster khuyến mãi cuối tuần</span><span class="spacer"></span><span class="muted">v2</span></div>
          <div class="row"><span class="truncate">Video giới thiệu 30 giây</span><span class="spacer"></span><span class="muted">v2</span></div>
          <div class="row"><span class="truncate">Infographic xu hướng nội dung</span><span class="spacer"></span><span style="color:var(--cs-warning)">v1</span></div>
          <span class="muted">Phát hành v3 không tự đổi dự án nào. Mỗi dự án tự chọn nâng cấp.</span></div></details>
        <details class="insp-section"><summary>${I("chev-right", "chev")}Xem trước trên thiết kế</summary><div class="insp-content">
          <div class="brand-preview"><div style="background:#334BD1;color:#fff;padding:16px;font-weight:700;font-size:18px">Sáng tạo nhất quán</div><div style="background:#fff;color:#182132;padding:12px;font-size:13px">Dùng thử miễn phí <span style="background:#F2A93B;color:#182132;padding:2px 6px;font-weight:600">Mới</span></div></div></div></details>`,
    };
  },
});

ACTIONS["brand-publish"] = () => UI.dialog({
  title: "Phát hành Brand v3?",
  body: `<p>Phiên bản v3 sẽ trở thành <strong>bất biến</strong>. Dự án hiện tại vẫn giữ phiên bản đã ghim; bạn có thể nâng cấp từng dự án sau.</p>
    ${UI.callout("warning", "alert", "Còn 1 cảnh báo không chặn", "Màu Nhấn không đạt tương phản cho chữ thường. Bộ kiểm tra thiết kế sẽ cảnh báo khi dùng sai.")}
    <div class="field"><label for="bp-note">Ghi chú phát hành</label><input class="input" id="bp-note" value="Thêm màu phụ, đổi CTA, đổi phông tiêu đề"></div>
    <label class="check"><input type="checkbox" checked>Đặt v3 làm mặc định cho dự án mới</label>`,
  footer: `<span class="foot-note">Cần xác minh danh tính Windows.</span><button class="btn" data-close>Hủy</button><button class="btn primary" id="bp-ok">${I("fingerprint")}Xác minh và phát hành</button>`,
  onOpen: (d) => d.el.querySelector("#bp-ok").addEventListener("click", () => ACTIONS["human-proof"]({ what: "Phát hành Brand v3 (Local Studio)", onOk: () => { d.close(); UI.toast({ type: "success", title: "Đã phát hành Brand v3", desc: "4 dự án vẫn dùng phiên bản cũ.", actionLabel: "Xem dự án", action: () => APP.go("dashboard") }); } })),
});
