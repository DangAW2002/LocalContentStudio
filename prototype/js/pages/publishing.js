PAGES.register({
  id: "publishing",
  label: "Bàn giao",
  sidebar({ st, D }) {
    if (st === "empty") return null;
    const selIdx = st === "error" ? 1 : st === "blocked" ? 2 : 0;
    return {
      title: "Gói bàn giao",
      actions: `<button class="btn quiet sm icon" data-action="prepare-package" aria-label="Chuẩn bị gói mới" data-tip="Chuẩn bị gói mới">${I("plus")}</button>`,
      body: `<div class="list" data-selectable>${D.deliveries.map((d, i) => `
        <button class="list-item two-line ${i === selIdx ? "is-selected" : ""}">
          ${I("package")}<span class="lines"><span class="truncate">${d.title}</span><span class="xs muted">${d.platform} · ${d.rev} · ${d.when}</span><span style="margin-top:4px">${UI.badge(d.state)}</span></span>
        </button>`).join("")}</div>`,
      foot: `<button class="btn block" data-action="prepare-package">${I("package")}Chuẩn bị gói mới</button>`,
    };
  },
  main({ st, D }) {
    const d = st === "error" ? D.deliveries[1] : st === "blocked" ? D.deliveries[2] : D.deliveries[0];
    const header = UI.pageHeader({
      title: d.title,
      meta: `${d.platform} ${UI.metaSep} ${d.page} ${UI.metaSep} ${d.rev} ${UI.metaSep} gói ${d.id}`,
      actions: st === "empty" ? "" : `<button class="btn" data-tip="Mở bản nháp này trong Social Harness" data-tip-sub="Việc duyệt, lên lịch và đăng diễn ra ở Social">${I("external")}Mở trong Social</button>
        <button class="btn quiet icon" data-menu="delivery" aria-haspopup="menu" aria-label="Thêm hành động" data-tip="Thêm hành động">${I("more")}</button>`,
    });

    /* Mỗi mốc là một trạng thái độc lập, có nguồn xác nhận riêng */
    const steps = (states) => `<ol class="milestones" aria-label="Các mốc bàn giao và xuất bản">${[
      ["Đã lưu", "commit", "Studio"], ["Đã xuất", "download", "Studio"], ["Social đã nhận", "receipt", "Social Harness"], ["Đã duyệt", "check", "Người duyệt tại Social"],
      ["Đã lên lịch", "calendar", "Social Harness"], ["Đã đăng", "publishing", "Nền tảng"], ["Đã xác minh", "shield-check", "Đọc lại permalink"],
    ].map(([t, ic, src], i) => {
      const [s, when] = states[i] || ["todo", ""];
      return `<li class="ms ${s}" ${s === "current" ? 'aria-current="step"' : ""}>
        <span class="ms-icon">${I(s === "done" ? "check" : s === "warn" ? "alert" : s === "current" ? "clock" : ic)}</span>
        <span class="ms-text"><span class="strong">${t}</span><span class="xs muted">${when || (s === "todo" ? "Chưa có" : "")}</span><span class="xs muted">Xác nhận bởi: ${src}</span></span></li>`;
    }).join("")}</ol>`;

    const manifest = `<section class="panel"><div class="panel-head"><h2>Gói bàn giao (bất biến)</h2><span class="badge outline">studio.delivery/v1</span></div>
        <div class="panel-body col-3">
          <dl class="kv"><dt>Revision</dt><dd>${d.rev} · <span class="hash copyable" data-copy="169da2fd44a2b9c1" data-tip="SHA-256 nội dung, nhấn để sao chép">169da2fd…b9c1</span></dd>
            <dt>Page đích</dt><dd>${d.page} <span class="xs muted">(chọn từ danh sách của Social)</span></dd>
            <dt>Caption</dt><dd>Biến thể Facebook · 212 ký tự · <a href="#" data-go="writing">xem</a></dd>
            <dt>Thương hiệu</dt><dd>Brand v2 (đã ghim)</dd>
            <dt>Quyền tài nguyên</dt><dd>${st === "blocked" ? `<span style="color:var(--cs-danger)">5/6 đủ quyền · 1 bị thu hồi</span>` : `<span style="color:var(--cs-success)">6/6 đủ quyền</span>`}</dd>
            <dt>Khóa chống trùng</dt><dd><span class="mono xs copyable" data-copy="idem-rev4-fb-page01">idem-rev4-fb-page01</span></dd></dl>
          <table class="table compact"><thead><tr><th>Tệp</th><th class="num">Dung lượng</th><th>SHA-256</th></tr></thead><tbody>
            ${Array.from({ length: 6 }, (_, i) => `<tr><td>${I("image")} trang-${i + 1}.png</td><td class="num xs">${(1.2 + i * 0.13).toFixed(1)} MB</td><td class="hash">${["4be1", "91ac", "07d3", "e22f", "5c90", "a1b7"][i]}…${["c9", "02", "7e", "41", "dd", "18"][i]}</td></tr>`).join("")}
          </tbody></table>
        </div></section>`;

    const receipt = `<section class="panel"><div class="panel-head"><h2>Biên nhận từ Social Harness</h2>${UI.badge("ACCEPTED")}</div>
        <div class="panel-body col-3">
          <dl class="kv"><dt>Mã bản nháp</dt><dd><span class="mono copyable" data-copy="sh_draft_81f2c0">sh_draft_81f2c0</span></dd><dt>Nhận lúc</dt><dd>14/10/2026 23:08:12</dd>
            <dt>Hash nội dung</dt><dd class="row" style="color:var(--cs-success)">${I("check")}Khớp với gói đã gửi</dd><dt>Từ</dt><dd>Social Harness · localhost:3099</dd></dl>
          ${UI.callout("neutral", "info", "Social đã nhận gói, nhưng chưa đăng", "Việc duyệt, lên lịch và đăng do Social Harness quản lý. Studio chỉ hiển thị những gì Social xác nhận, không tự suy ra trạng thái.")}
          <span class="label">Diễn biến</span>
          <div class="events xs">
            <div><span class="mono">23:08:12</span> Social trả biên nhận sh_draft_81f2c0</div>
            <div><span class="mono">23:08:11</span> Gửi gói (8,4 MB) với khóa chống trùng</div>
            <div><span class="mono">23:07:55</span> Bạn xác minh qua Windows Hello</div>
            <div><span class="mono">23:07:40</span> Kiểm tra sẵn sàng: đạt 7/7</div>
          </div>
          <button class="btn sm" style="align-self:flex-start" data-tip="Hỏi lại Social trạng thái mới nhất" data-tip-sub="Lần cập nhật gần nhất: 23:12">${I("refresh")}Cập nhật trạng thái</button>
        </div></section>`;

    const check = (ok, t, d, fix = "") => `<div class="check-row ${ok ? "ok" : "fail"}"><span>${I(ok ? "check-circle" : "x-circle")}</span><div class="col" style="gap:0;flex:1"><span class="strong">${t}</span><span class="xs muted">${d}</span></div>${fix}</div>`;

    const body = UI.byState(st, {
      loading: () => `${UI.skeletonRows(1, 88)}<div style="height:12px"></div>${UI.skeletonRows(5)}`,
      empty: () => `<div class="panel">${UI.empty({
          icon: "package", title: "Chưa có gói bàn giao",
          desc: "Gói bàn giao ghim đúng revision, caption, tệp đã xuất và Page đích rồi gửi sang Social Harness. Việc duyệt và đăng diễn ra ở Social.",
          actions: `<button class="btn primary" data-action="prepare-package">${I("package")}Chuẩn bị gói</button><button class="btn" data-go="settings" data-settings="social">Kết nối Social Harness</button>`,
        })}</div>`,
      error: () => `
        ${UI.callout("warning", "alert", "Chưa rõ Social đã nhận gói hay chưa", "Kết nối bị ngắt lúc 17:42 trước khi có biên nhận. Gói có thể đã tới Social. <strong>Không gửi lại</strong>: hãy đối soát bằng khóa chống trùng để tránh đăng hai lần.", `<button class="btn sm primary" data-action="reconcile">${I("refresh")}Đối soát với Social</button><button class="btn sm" data-action="job-error">Xem chi tiết lỗi</button>`)}
        ${steps([["done", "Hôm qua 17:30"], ["done", "6 PNG · hash đã kiểm"], ["warn", "Chưa rõ: cần đối soát"]])}
        <div class="pub-grid">${manifest}
          <section class="panel"><div class="panel-head"><h2>Gửi lại</h2></div><div class="panel-body">
            <div class="unavailable"><button class="btn" disabled>${I("publishing")}Gửi lại sang Social</button><span class="reason">${I("lock")}Bị khóa cho tới khi đối soát xong, để tránh đăng trùng.</span></div>
          </div></section></div>`,
      blocked: () => `
        ${steps([["done", "12/10 09:15"], ["current", "Chưa xuất bản mới"]])}
        <div class="pub-grid">
          <section class="panel"><div class="panel-head"><h2>Kiểm tra sẵn sàng</h2><span class="badge danger">${I("x-circle")}3 mục chưa đạt</span></div>
            <div class="panel-body col">
              ${check(true, "Revision đã lưu", "rev-7 · hash 02bc…7e11")}
              ${check(false, "Kết nối Social Harness", "Không liên lạc được localhost:3099", `<button class="btn sm" data-go="settings" data-settings="social">Kết nối</button>`)}
              ${check(false, "Quyền tài nguyên", "bieu-do-tang-truong.png đã bị thu hồi quyền", `<button class="btn sm" data-go="library">Thay tệp</button>`)}
              ${check(false, "Từ cấm", "“cam kết 100%” trong caption LinkedIn", `<button class="btn sm" data-go="writing">Sửa caption</button>`)}
              ${check(true, "Độ dài caption", "1.240 / 3.000 ký tự")}
              ${check(true, "Thương hiệu", "Brand v1 · vẫn hợp lệ (có v2 mới hơn)")}
              ${check(true, "Khổ ảnh", "1080 × 1080 khớp hồ sơ LinkedIn")}
              <hr class="divider">
              <div class="unavailable"><button class="btn primary" disabled>${I("publishing")}Xác minh và gửi sang Social</button><span class="reason">${I("lock")}Cần xử lý 3 mục chưa đạt ở trên</span></div>
            </div></section>
          ${manifest}</div>`,
      normal: () => `
        ${steps([["done", "rev-4 · 23:08"], ["done", "6 PNG · hash đã kiểm"], ["done", "Biên nhận 23:08"], ["current", "Đang chờ người duyệt"]])}
        <div class="pub-grid">${manifest}${receipt}</div>`,
    });
    return `<div class="page">${header}<div class="page-body"><div class="col-4">${body}</div></div></div>`;
  },
});

window.MENUS.delivery = () => [
  { icon: "copy", text: "Sao chép mã gói" },
  { icon: "download", text: "Lưu manifest (.json)" },
  { icon: "folder-open", text: "Mở thư mục tệp đã xuất" },
  "-",
  { icon: "user", text: "Tôi đã tự đăng bài này…", disabled: "Không khả dụng khi Social đang xử lý gói, để tránh đăng trùng" },
  { icon: "x-circle", text: "Rút gói khỏi Social…", danger: true },
];
ACTIONS.reconcile = () => {
  UI.dialog({
    title: "Đối soát với Social Harness",
    body: `<div class="col-3"><div class="row">${I("loader")}<span>Đang hỏi Social theo khóa <span class="mono xs">idem-rev2-ig-page02</span>…</span></div>${UI.progress(null, "Đang đối soát")}
      <span class="xs muted">Studio chỉ hỏi trạng thái, không gửi lại gói.</span></div>`,
    footer: `<button class="btn" data-close>Đóng</button>`,
    onOpen: (d) => setTimeout(() => {
      if (!d.el.isConnected) return;
      d.el.querySelector(".dialog-body").innerHTML = `${UI.callout("success", "receipt", "Social đã nhận gói từ hôm qua 17:42", "Không cần gửi lại. Biên nhận sh_draft_77a019 đã được lưu và trạng thái chuyển sang “Social đã nhận”.")}
        <dl class="kv"><dt>Mã bản nháp</dt><dd class="mono">sh_draft_77a019</dd><dt>Hash nội dung</dt><dd style="color:var(--cs-success)">Khớp</dd><dt>Nguồn</dt><dd>Social Harness · đối soát lúc 23:14</dd></dl>`;
    }, 1400),
  });
};
ACTIONS["prepare-package"] = () => {
  const ok = (t, d) => `<div class="check-row ok"><span>${I("check-circle")}</span><div class="col" style="gap:0"><span class="strong">${t}</span><span class="xs muted">${d}</span></div></div>`;
  UI.dialog({
    title: "Chuẩn bị gói bàn giao",
    size: "lg",
    guard: true,
    body: `<div class="grid-2">
        <div class="field"><label>Dự án & revision</label><select class="select"><option>Ra mắt gói tự động hóa Q4 · rev-4</option></select><span class="hint">Chỉ revision đã lưu và đã xuất.</span></div>
        <div class="field"><label>Page đích</label><select class="select"><option>Fanpage Local Studio (Facebook)</option><option>@localstudio (Instagram)</option></select><span class="hint">Danh sách lấy từ Social Harness lúc 23:12.</span></div></div>
      <div class="grid-2"><div class="field"><label>Caption</label><select class="select"><option>Biến thể Facebook · 212 ký tự</option></select></div>
        <div class="field"><label>Tệp</label><select class="select"><option>Xuất PNG rev-4 · 6 tệp · 8,4 MB</option></select></div></div>
      <div class="col"><span class="label">Kiểm tra sẵn sàng</span>
        ${ok("Quyền tài nguyên", "6/6 đủ quyền dùng và bàn giao")}${ok("Thương hiệu", "Brand v2 · không có từ cấm")}${ok("Tệp đã kiểm hash", "6/6 khớp manifest")}${ok("Kết nối Social", "localhost:3099 · phản hồi 42 ms")}</div>
      <label class="check"><input type="checkbox" id="pp-ok">Tôi đã xem lại nội dung và đồng ý gửi gói này sang Social Harness để duyệt.</label>`,
    footer: `<span class="foot-note">Gửi gói không đăng bài. Việc đăng cần người duyệt tại Social.</span><button class="btn" data-close>Hủy</button><button class="btn primary" id="pp-go" disabled>${I("fingerprint")}Xác minh và gửi</button>`,
    onOpen: (d) => {
      d.el.querySelector("#pp-ok").addEventListener("change", (e) => (d.el.querySelector("#pp-go").disabled = !e.target.checked));
      d.el.querySelector("#pp-go").addEventListener("click", () => ACTIONS["human-proof"]({ what: "Gửi gói Carousel Q4 rev-4 tới Fanpage Local Studio", onOk: () => { d.guard = false; d.close(); UI.toast({ type: "info", title: "Đang truyền gói sang Social…", desc: "Trạng thái chỉ chuyển sang “Social đã nhận” khi có biên nhận." }); } }));
    },
  });
};
