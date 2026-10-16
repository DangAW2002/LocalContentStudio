/* Overlay dùng chung: tạo dự án, xuất, nhập tệp, duyệt quyền, xác nhận người dùng, lưu revision, phím tắt. */
(function () {
  const A = window.ACTIONS;
  const D = window.DATA;

  /* ---------- Tạo dự án mới: wizard 4 bước ---------- */
  A["new-project"] = () => {
    let step = 0;
    const steps = ["Loại & tên", "Brief", "Nguồn & thương hiệu", "Khổ & mẫu"];
    const types = [
      ["design", "Carousel", "Nhiều trang, vuốt ngang", "1080 × 1350 · 2–10 trang"],
      ["image", "Poster", "Một trang tĩnh", "1080 × 1350 / 1080 × 1920"],
      ["layout-grid", "Infographic", "Dọc, nhiều khối dữ liệu", "1080 × 1920+"],
      ["file-text", "Bài viết / caption", "Chỉ văn bản và biến thể kênh", "Không có canvas"],
      ["video", "Video mẫu", "Slideshow, chữ động, phụ đề", "Cần FFmpeg (đã có)"],
      ["file", "Báo cáo / deck", "Nhiều trang dạng tài liệu", "Xuất HTML; PDF chưa có", true],
    ];
    const body = () => {
      const s = `<ol class="steps" aria-label="Các bước">${steps.map((t, i) => `<li class="step ${i < step ? "done" : i === step ? "current" : ""}" ${i === step ? 'aria-current="step"' : ""}><span class="n">${i < step ? "✓" : i + 1}</span>${t}</li>`).join("")}</ol>`;
      const pages = [
        `<div class="col-3">
          <span class="label" id="np-type">Loại nội dung</span>
          <div class="grid-3" role="radiogroup" aria-labelledby="np-type" data-exclusive>
            ${types.map((t, i) => `<button type="button" role="radio" class="np-type" aria-pressed="${i === 0}" aria-checked="${i === 0}" ${t[4] ? 'data-tip="PDF chưa có renderer; vẫn xuất được HTML"' : ""}>
              ${I(t[0], "icon-lg")}<span class="strong">${t[1]}</span><span class="xs muted">${t[2]}</span><span class="xs muted">${t[3]}</span></button>`).join("")}
          </div>
          <div class="grid-2">
            <div class="field"><label for="np-name">Tên dự án</label><input class="input" id="np-name" value="Chiến dịch Tết 2027" autofocus></div>
            <div class="field"><label for="np-ws">Workspace</label><select class="select" id="np-ws">${D.workspaces.filter((w) => w.id !== "global").map((w) => `<option>${w.name}</option>`).join("")}</select></div>
          </div>
          <div class="field"><label>Nơi lưu</label><div class="input-group"><input class="input mono" readonly value="D:\\Studio\\Marketing Q4\\chien-dich-tet-2027"><button class="btn">${I("folder-open")}Đổi…</button></div><span class="hint">Còn trống 182 GB · bản sao lưu tự động hằng ngày</span></div>
        </div>`,
        `<div class="col-3">
          <div class="field"><label for="np-goal">Mục tiêu chính</label><input class="input" id="np-goal" placeholder="Ví dụ: Tăng đăng ký dùng thử gói tự động hóa"></div>
          <div class="field"><label for="np-aud">Đối tượng</label><input class="input" id="np-aud" placeholder="Ví dụ: Chủ doanh nghiệp nhỏ, quản lý marketing"></div>
          <div class="field"><label for="np-msg">Thông điệp chính</label><textarea class="textarea" id="np-msg" rows="3" placeholder="Một hai câu người xem cần nhớ"></textarea></div>
          <div class="field"><label>Kênh dự kiến</label><div class="row wrap">${["Facebook", "Instagram", "LinkedIn", "TikTok"].map((k, i) => `<label class="check"><input type="checkbox" ${i < 2 ? "checked" : ""}>${k}</label>`).join("")}</div></div>
          <span class="xs muted">Có thể bỏ qua bước này và viết brief sau trong Soạn thảo.</span>
        </div>`,
        `<div class="col-3">
          <div class="field"><label for="np-brand">Phiên bản thương hiệu</label><select class="select" id="np-brand"><option>Brand v2 · đã phát hành (khuyến nghị)</option><option>Brand v1 · đã phát hành</option><option disabled>Brand v3 · bản nháp (chưa ghim được)</option></select><span class="hint">Dự án ghim đúng phiên bản này. Đổi phiên bản sau sẽ tạo revision mới.</span></div>
          <span class="label">Gói nghiên cứu đính kèm</span>
          <div class="list panel">${D.sources.filter((s) => s.state === "FETCHED").map((s, i) => `<label class="list-item"><input type="checkbox" ${i === 0 ? "checked" : ""}><span class="lines"><span>${s.title}</span><span class="xs muted">${s.host} · ${s.evidence} trích dẫn</span></span></label>`).join("")}</div>
          <button class="btn" data-go="research" data-close>${I("globe")}Thu thập nguồn mới…</button>
        </div>`,
        `<div class="col-3">
          <div class="grid-2">
            <div class="field"><label>Khổ</label><select class="select"><option>1080 × 1350 (4:5)</option><option>1080 × 1080 (1:1)</option><option>1080 × 1920 (9:16)</option><option>Tùy chỉnh…</option></select></div>
            <div class="field"><label>Số trang</label><input class="input" type="number" min="1" max="10" value="6"></div>
          </div>
          <span class="label">Mẫu bắt đầu</span>
          <div class="grid-3" data-exclusive>
            ${["Trống", "Danh sách 5 ý", "Trước / sau", "Số liệu nổi bật", "Câu chuyện khách hàng", "Hỏi đáp"].map((t, i) => `<button type="button" class="np-type" aria-pressed="${i === 1}"><span class="thumb" style="height:72px;width:100%">${I("layout-grid")}</span><span class="xs">${t}</span></button>`).join("")}
          </div>
        </div>`,
      ];
      return s + pages[step];
    };
    const foot = () => `<span class="foot-note">Bản nháp wizard được giữ nếu bạn đóng hộp thoại.</span>
      ${step > 0 ? `<button class="btn" data-np="back">${I("chev-left")}Quay lại</button>` : `<button class="btn" data-close>Hủy</button>`}
      ${step === 1 ? `<button class="btn quiet" data-np="next">Bỏ qua</button>` : ""}
      <button class="btn primary" data-np="${step === 3 ? "create" : "next"}">${step === 3 ? `${I("check")}Tạo dự án` : `Tiếp tục${I("chev-right")}`}</button>`;
    const d = UI.dialog({ title: "Tạo dự án mới", size: "lg", body: body(), footer: foot() });
    d.el.addEventListener("click", (e) => {
      const b = e.target.closest("[data-np]");
      if (!b) return;
      if (b.dataset.np === "create") { d.close(); UI.toast({ type: "success", title: "Đã tạo dự án “Chiến dịch Tết 2027”", desc: "rev-1 đã lưu · Brand v2 đã ghim", actionLabel: "Mở trong Thiết kế", action: () => APP.go("design") }); return; }
      step += b.dataset.np === "back" ? -1 : 1;
      d.el.querySelector(".dialog-body").innerHTML = body();
      d.el.querySelector(".dialog-foot").innerHTML = foot();
      d.el.querySelector(".dialog-body input, .dialog-body button")?.focus();
    });
  };

  /* ---------- Xuất ---------- */
  A.export = () => {
    UI.dialog({
      title: "Xuất bản thiết kế",
      size: "lg",
      body: `
        <div class="grid-2">
          <div class="field"><label for="ex-rev">Revision</label><select class="select" id="ex-rev"><option>rev-4 · đã lưu 23:08 (mới nhất)</option><option>rev-3 · 22:40</option></select><span class="hint">Chỉ xuất từ revision đã lưu. Thay đổi chưa lưu không được đưa vào.</span></div>
          <div class="field"><label>Thư mục đích</label><div class="input-group"><input class="input mono" readonly value="exports\\q4-rev4"><button class="btn">${I("folder-open")}</button></div></div>
        </div>
        <div class="col"><span class="label">Định dạng</span>
          <div class="grid-3" data-exclusive>
            <button class="np-type" aria-pressed="true">${I("image", "icon-lg")}<span class="strong">PNG</span><span class="xs muted">Từng trang · 1080 × 1350</span>${UI.badge("CAP_VERIFIED")}</button>
            <button class="np-type" aria-pressed="false">${I("file", "icon-lg")}<span class="strong">SVG</span><span class="xs muted">Vector, giữ chữ</span>${UI.badge("CAP_VERIFIED")}</button>
            <button class="np-type" aria-pressed="false">${I("globe", "icon-lg")}<span class="strong">HTML tĩnh</span><span class="xs muted">Một tệp, không script</span>${UI.badge("CAP_VERIFIED")}</button>
          </div>
          <div class="unavailable"><button class="btn" disabled>${I("file-text")}PDF</button><span class="reason">${I("info")}Chưa có renderer PDF. <a href="#" data-go="settings" data-settings="caps" data-close>Xem năng lực hệ thống</a></span></div>
        </div>
        <div class="col"><span class="label">Trang</span>
          <div class="row wrap">${Array.from({ length: 6 }, (_, i) => `<label class="check"><input type="checkbox" checked>Trang ${i + 1}</label>`).join("")}</div>
        </div>
        ${UI.callout("success", "shield-check", "Kiểm tra trước khi xuất: đạt", "6/6 tài nguyên đủ quyền · phông chữ đã nhúng · không có chữ tràn khung.")}
        <label class="check"><input type="checkbox" checked>Ghi kèm manifest (revision, renderer, SHA-256 từng tệp)</label>`,
      footer: `<span class="foot-note">Đầu ra được đọc lại và kiểm hash trước khi báo hoàn tất.</span><button class="btn" data-close>Hủy</button><button class="btn primary" data-close data-action="export-run">${I("download")}Xuất 6 tệp PNG</button>`,
    });
  };
  A["export-run"] = () => UI.toast({ type: "info", title: "Đã bắt đầu xuất 6 tệp PNG", desc: "Theo dõi tiến độ thật trong Task Center.", actionLabel: "Mở Task Center", action: () => A.tasks() });

  /* ---------- Nhập tệp ---------- */
  A.upload = () => {
    const d = UI.dialog({
      title: "Nhập tệp vào thư viện",
      size: "lg",
      body: `
        <div class="dropzone" id="dz" tabindex="0" role="button" aria-label="Chọn hoặc kéo tệp vào đây">
          ${I("upload", "icon-xl")}<strong>Kéo tệp vào đây hoặc chọn từ máy</strong>
          <span class="xs">PNG, JPG, WEBP, GIF, SVG, MP4, MP3, WAV, PDF · tối đa 50 MB/tệp · quota còn 3,8 GB</span>
          <button class="btn" type="button">${I("folder-open")}Chọn tệp…</button>
        </div>
        <div class="grid-2">
          <div class="field"><label>Phạm vi</label><select class="select"><option>Workspace: Marketing Q4</option><option>Thư viện toàn cục (cần quyền ghi toàn cục)</option></select></div>
          <div class="field"><label>Thẻ</label><input class="input" placeholder="Ví dụ: q4, sản-phẩm"></div>
        </div>
        <div class="col"><span class="label">Đang xử lý</span>
          <div class="panel">
            ${[["anh-san-pham-moi.jpg", "Đang ghi bytes · 612/850 KB", 72, ""], ["doi-ngu-hop.jpg", "Kiểm chữ ký tệp · băm SHA-256", null, ""], ["bao-gia.exe", "Bị từ chối: định dạng không được hỗ trợ", 0, "danger"], ["poster-cu.png", "Trùng nội dung với banner-chinh.png (cùng SHA-256)", 100, "warning"]].map(([n, s, p, v]) => `
              <div class="row-3" style="padding:10px 16px;border-bottom:1px solid var(--cs-border-subtle)">
                ${I(v === "danger" ? "x-circle" : v === "warning" ? "copy" : "file")}
                <div class="col" style="gap:4px;flex:1;min-width:0"><span class="row"><span class="truncate strong">${n}</span></span>
                  ${v ? `<span class="xs" style="color:var(--cs-${v})">${s}</span>` : `${UI.progress(p, n)}<span class="xs muted">${s}</span>`}</div>
                ${v === "warning" ? `<button class="btn sm">Dùng bản có sẵn</button>` : v === "danger" ? `<button class="btn sm quiet" aria-label="Bỏ khỏi danh sách" data-tip="Bỏ khỏi danh sách">${I("x")}</button>` : `<button class="btn sm quiet" aria-label="Hủy tải" data-tip="Hủy tải tệp này">${I("x")}</button>`}
              </div>`).join("")}
          </div>
        </div>
        ${UI.callout("warning", "shield-alert", "Tệp mới luôn vào trạng thái cách ly", "Bạn cần xác nhận nguồn và giấy phép trước khi dùng tệp để xuất hoặc bàn giao. AI không thể làm thay bước này.")}`,
      footer: `<span class="foot-note">Đóng hộp thoại không hủy tải; tiến trình tiếp tục trong Task Center.</span><button class="btn" data-close>Đóng</button><button class="btn primary" data-close data-go="library">Mở thư viện</button>`,
      onOpen(dd) {
        const dz = dd.el.querySelector("#dz");
        ["dragenter", "dragover"].forEach((t) => dz.addEventListener(t, (e) => { e.preventDefault(); dz.classList.add("is-over"); }));
        ["dragleave", "drop"].forEach((t) => dz.addEventListener(t, (e) => { e.preventDefault(); dz.classList.remove("is-over"); }));
      },
    });
  };

  /* ---------- Duyệt bản quyền (chỉ người dùng) ---------- */
  A["rights-review"] = (el) => {
    const a = D.assets.find((x) => x.id === el?.dataset.asset) || D.assets[1];
    const d = UI.dialog({
      title: `Xác nhận quyền sử dụng · ${a.name}`,
      size: "lg",
      guard: true,
      body: `
        <div class="row-3" style="align-items:flex-start">
          <div class="thumb" style="width:160px;height:120px;flex:none;background:hsl(${a.tone} 45% 70%)">${I("image", "icon-xl")}</div>
          <dl class="kv" style="flex:1"><dt>Định dạng</dt><dd>${a.ext} · ${a.dims} · ${a.size}</dd><dt>SHA-256</dt><dd class="hash">${a.hash}…e91c</dd><dt>Nhập lúc</dt><dd>Hôm nay 22:31 · bởi Bạn</dd><dt>Phạm vi</dt><dd>${a.scope}: Marketing Q4</dd></dl>
        </div>
        <div class="grid-2">
          <div class="field"><label for="rr-src">Nguồn gốc <span style="color:var(--cs-danger)">*</span></label><select class="select" id="rr-src"><option value="">Chọn nguồn…</option><option>Tự chụp / tự tạo</option><option>Đối tác cung cấp</option><option>Thư viện stock</option><option>Trang web bên ngoài</option></select></div>
          <div class="field"><label for="rr-lic">Cơ sở giấy phép <span style="color:var(--cs-danger)">*</span></label><select class="select" id="rr-lic"><option value="">Chọn giấy phép…</option><option>Sở hữu doanh nghiệp</option><option>Giấy phép thương mại đã mua</option><option>CC0 / Public domain</option><option>CC BY (cần ghi công)</option><option>Đồng ý bằng văn bản</option></select></div>
        </div>
        <div class="field"><label for="rr-attr">Ghi công (nếu giấy phép yêu cầu)</label><input class="input" id="rr-attr" placeholder="Ví dụ: Ảnh: Nguyễn Văn A / Unsplash"></div>
        <div class="field"><label for="rr-ev">Bằng chứng</label><div class="input-group"><input class="input" id="rr-ev" placeholder="Liên kết hợp đồng, hóa đơn hoặc email đồng ý"><button class="btn">${I("file-plus")}Đính kèm</button></div></div>
        <div class="col"><span class="label">Được phép dùng cho</span><div class="row wrap">${["Xuất tệp", "Bàn giao sang Social", "Quảng cáo trả phí", "In ấn"].map((k, i) => `<label class="check"><input type="checkbox" ${i < 2 ? "checked" : ""}>${k}</label>`).join("")}</div></div>
        <label class="check"><input type="checkbox" id="rr-ok"><span>Tôi xác nhận thông tin trên là đúng và tôi có thẩm quyền quyết định quyền sử dụng tệp này. Xác nhận được ghi vào nhật ký kiểm toán.</span></label>
        ${UI.callout("neutral", "fingerprint", "Cần xác minh danh tính Windows", "Thao tác chỉ dành cho người dùng. Agent và CLI không thể xác nhận thay.")}`,
      footer: `<span class="foot-note" id="rr-note">Điền các trường bắt buộc và đánh dấu xác nhận.</span><button class="btn" data-close>Hủy</button><button class="btn primary" id="rr-go" disabled>${I("fingerprint")}Xác minh và cấp quyền</button>`,
      onOpen(dd) {
        const go = dd.el.querySelector("#rr-go");
        const check = () => {
          const ok = dd.el.querySelector("#rr-src").value && dd.el.querySelector("#rr-lic").value && dd.el.querySelector("#rr-ok").checked;
          go.disabled = !ok;
          dd.el.querySelector("#rr-note").textContent = ok ? "Sẵn sàng xác minh." : "Điền các trường bắt buộc và đánh dấu xác nhận.";
        };
        dd.el.addEventListener("change", check);
        go.addEventListener("click", () => A["human-proof"]({ what: "Cấp quyền sử dụng cho " + a.name, onOk: () => { dd.guard = false; dd.close(); UI.toast({ type: "success", title: "Đã cấp quyền sử dụng", desc: `${a.name} chuyển sang “Đủ quyền dùng”. Đã ghi nhật ký kiểm toán.` }); } }));
      },
    });
  };

  /* ---------- Xác minh danh tính (giả lập hộp thoại hệ điều hành) ---------- */
  A["human-proof"] = ({ what, onOk }) => {
    const d = UI.dialog({
      title: "Bảo mật Windows",
      body: `<div class="col-3" style="align-items:center;text-align:center">
          <div class="state-icon" style="width:64px;height:64px;display:flex;align-items:center;justify-content:center;background:var(--cs-accent-subtle);color:var(--cs-accent)">${I("fingerprint", "icon-xl")}</div>
          <strong>Local Content Studio cần xác minh bạn</strong>
          <span class="muted">${what}</span>
          <span class="xs muted">Mã xác nhận chỉ dùng một lần, hết hạn sau 5 phút, gắn với đúng nội dung này.</span>
        </div>`,
      footer: `<span class="foot-note">Prototype: mô phỏng Windows Hello.</span><button class="btn" data-close>Hủy</button><button class="btn primary" id="hp-ok" autofocus>Xác minh</button>`,
      onOpen(dd) { dd.el.querySelector("#hp-ok").addEventListener("click", () => { dd.close(); onOk && onOk(); }); },
    });
  };

  /* ---------- Lưu revision ---------- */
  A["save-revision"] = () => {
    UI.dialog({
      title: "Lưu revision mới",
      body: `<div class="field"><label for="sr-sum">Mô tả thay đổi</label><input class="input" id="sr-sum" autofocus value="Đổi CTA trang 6, căn lại tiêu đề trang 2"></div>
        <dl class="kv"><dt>Revision kế tiếp</dt><dd><strong>rev-5</strong> (cha: rev-4)</dd><dt>Thay đổi</dt><dd>3 phần tử trên 2 trang</dd><dt>Thương hiệu</dt><dd>Brand v2 (đã ghim)</dd></dl>
        ${UI.callout("info", "info", "Revision là bất biến", "Sau khi lưu, revision không sửa được. Bản xem trước và gói bàn giao cũ vẫn giữ revision cũ.")}`,
      footer: `<span class="foot-note">${UI.kbd("Ctrl", "Enter")} để lưu</span><button class="btn" data-close>Hủy</button><button class="btn primary" data-close data-action="save-ok">${I("commit")}Lưu rev-5</button>`,
      onOpen(dd) { dd.el.addEventListener("keydown", (e) => { if (e.ctrlKey && e.key === "Enter") { dd.close(); A["save-ok"](); } }); },
    });
  };
  A["save-ok"] = () => UI.toast({ type: "success", title: "Đã lưu rev-5", desc: "SHA-256 7c41…a09e · 23:14" });

  /* ---------- Phím tắt ---------- */
  A.shortcuts = () => {
    const g = (title, rows) => `<div class="col"><span class="section-label">${title}</span><table class="table compact"><tbody>${rows.map(([k, t]) => `<tr><td>${t}</td><td class="num">${UI.kbd(...k.split(" "))}</td></tr>`).join("")}</tbody></table></div>`;
    UI.dialog({
      title: "Phím tắt",
      size: "lg",
      body: `<div class="grid-2" style="align-items:start">
        ${g("Chung", [["Ctrl K", "Tìm hoặc chạy lệnh"], ["Ctrl N", "Tạo dự án"], ["Ctrl S", "Lưu revision"], ["Ctrl E", "Xuất"], ["Ctrl J", "Task Center"], ["Ctrl B", "Ẩn/hiện thanh bên"], ["Ctrl .", "Ẩn/hiện bảng thuộc tính"], ["Ctrl ,", "Cài đặt"], ["?", "Phím tắt"]])}
        ${g("Điều hướng", [["Ctrl 1", "Tổng quan"], ["Ctrl 2", "Nghiên cứu"], ["Ctrl 3", "Thư viện"], ["Ctrl 4", "Thương hiệu"], ["Ctrl 5", "Soạn thảo"], ["Ctrl 6", "Thiết kế"], ["Ctrl 7", "Chuyển động"], ["Ctrl 8", "Tự động hóa & AI"], ["Ctrl 9", "Bàn giao"]])}
        ${g("Thiết kế", [["V", "Chọn"], ["T", "Chữ"], ["R", "Hình khối"], ["H", "Kéo khung nhìn"], ["Ctrl Z", "Hoàn tác"], ["Ctrl Y", "Làm lại"], ["Ctrl G", "Nhóm"], ["Ctrl 0", "Vừa khung"], ["Mũi tên", "Dịch 1 px (Shift: 10 px)"], ["Tab", "Chọn phần tử kế tiếp"]])}
        ${g("Chuyển động", [["Space", "Phát / dừng"], ["J L", "Lùi / tới 1 giây"], ["S", "Cắt tại đầu phát"]])}
      </div>
      <span class="xs muted">Không dùng tổ hợp Ctrl+Shift để tránh xung đột với bộ gõ tiếng Việt (Unikey, EVKey).</span>`,
      footer: `<button class="btn" data-close>Đóng</button>`,
    });
  };

  /* ---------- Công bố trước khi gửi dữ liệu lên AI đám mây ---------- */
  A["ai-disclosure"] = (el) => {
    UI.dialog({
      title: "Gửi dữ liệu tới nhà cung cấp AI",
      body: `<dl class="kv"><dt>Nhà cung cấp</dt><dd>OpenAI-compatible · gpt-4.1-mini</dd><dt>Dữ liệu gửi đi</dt><dd>Đoạn đã chọn (212 ký tự), brief (mục tiêu, đối tượng), quy tắc giọng điệu Brand v2</dd><dt>Không gửi</dt><dd>Tài nguyên, khóa API, đường dẫn tệp, các dự án khác</dd><dt>Ước tính</dt><dd>~640 token · ~0,01 USD · giới hạn còn 4,72 USD</dd></dl>
        ${UI.callout("info", "info", "AI chỉ tạo đề xuất", "Bạn sẽ thấy so sánh trước/sau và quyết định áp dụng hay bỏ.")}
        <label class="check"><input type="checkbox">Không hỏi lại cho loại tác vụ này trong phiên làm việc</label>`,
      footer: `<span class="foot-note">Có thể chuyển sang mô hình cục bộ trong Cài đặt.</span><button class="btn" data-close>Hủy</button><button class="btn primary" data-close data-action="ai-run">${I("sparkles")}Gửi và tạo đề xuất</button>`,
    });
  };
  A["ai-run"] = () => UI.toast({ type: "info", title: "Đang tạo đề xuất…", desc: "Có thể hủy trong bảng AI. Bản gốc không bị thay đổi." });
})();
