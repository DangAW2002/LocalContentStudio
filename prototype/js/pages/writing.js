PAGES.register({
  id: "writing",
  label: "Soạn thảo",
  sidebar({ st }) {
    if (st === "empty") return null;
    const v = (ic, t, sub, sel, depth, warn) => `<button class="list-item tree-item two-line ${sel ? "is-selected" : ""}" style="--depth:${depth}">${I(ic)}<span class="lines"><span class="truncate">${t}</span>${sub ? `<span class="xs muted truncate">${sub}</span>` : ""}</span>${warn ? `<span class="meta" style="color:var(--cs-warning)" data-tip="${warn}">${I("alert")}</span>` : ""}</button>`;
    return {
      title: "Nội dung dự án",
      actions: `<button class="btn quiet sm icon" aria-label="Thêm" data-menu="writing-add" aria-haspopup="menu" data-tip="Thêm brief hoặc biến thể">${I("plus")}</button>`,
      body: `<div class="list" data-selectable>
          ${v("file-text", "Brief: Ra mắt gói tự động hóa", "Mục tiêu · đối tượng · 2 trích dẫn", false, 0)}
          ${v("publishing", "Facebook", "212 ký tự · tự lưu 23:11", true, 1)}
          ${v("publishing", "Instagram", "Lỗi thời so với brief", false, 1, "Brief đổi sau khi tạo biến thể")}
          ${v("publishing", "LinkedIn", "Bản nháp trống", false, 1)}
          ${v("file-text", "Bài viết dài: 5 cách giảm thời gian sản xuất", "1.240 từ", false, 0)}
        </div>`,
      foot: `<button class="btn block" data-menu="writing-add" aria-haspopup="menu">${I("plus")}Thêm biến thể kênh</button>`,
    };
  },
  main({ st, D }) {
    const header = UI.pageHeader({
      title: "Biến thể Facebook",
      meta: `Brief: Ra mắt gói tự động hóa ${UI.metaSep} Brand v2 ${UI.metaSep} ${UI.badge("AUTOSAVED", { label: "Tự lưu nháp 23:11" })}`,
      actions: st === "empty" ? "" : `<button class="btn" data-tip="Tạo biến thể cho kênh khác từ brief này">${I("copy")}Nhân sang kênh khác</button>
        <button class="btn" data-go="design" data-tip="Đưa caption và CTA vào thiết kế">${I("design")}Mở thiết kế</button>
        <button class="btn primary" data-action="save-revision" data-tip="Lưu thành revision bất biến" data-kbd="Ctrl+S">${I("commit")}Lưu revision</button>`,
    });
    const text = st === "blocked"
      ? `🚀 Giảm <span class="flag" data-tip="Từ cấm theo Brand v2" data-tip-sub="Gợi ý: “giảm tới 42%”" tabindex="0">cam kết 100%</span> thời gian sản xuất nội dung với Local Content Studio.`
      : `🚀 Đội của bạn mất bao lâu cho một carousel? <span class="sel-text">Các đội dùng quy trình tự động hóa giảm 42% thời gian sản xuất nội dung</span><sup class="cite" data-tip="[1] Báo cáo xu hướng thị trường nội dung Q4" data-tip-sub="example.com · Mục 2 · đoạn 3" tabindex="0">[1]</sup>.`;
    const editor = `
      <section class="panel editor-panel">
        <div class="tabs" role="tablist" aria-label="Kênh">
          <button role="tab" aria-selected="true">${I("publishing")}Facebook</button>
          <button role="tab" aria-selected="false" data-tip="Lỗi thời: brief đổi sau khi tạo">${I("alert")}Instagram</button>
          <button role="tab" aria-selected="false">LinkedIn</button>
          <button class="btn quiet sm" style="margin:5px 6px" data-menu="writing-add" aria-haspopup="menu">${I("plus")}Kênh</button>
        </div>
        <div class="editor-toolbar" role="toolbar" aria-label="Định dạng">
          <button class="btn quiet sm icon" aria-label="Hoàn tác" data-tip="Hoàn tác" data-kbd="Ctrl+Z">${I("undo")}</button>
          <button class="btn quiet sm icon" aria-label="Làm lại" data-tip="Làm lại" data-kbd="Ctrl+Y">${I("redo")}</button>
          <span class="vdivider"></span>
          <button class="btn quiet sm icon" aria-label="Đậm" data-tip="Đậm" data-kbd="Ctrl+B"><strong>B</strong></button>
          <button class="btn quiet sm icon" aria-label="Nghiêng" data-tip="Nghiêng" data-kbd="Ctrl+I"><em>I</em></button>
          <button class="btn quiet sm icon" aria-label="Liên kết" data-tip="Chèn liên kết">${I("link")}</button>
          <span class="vdivider"></span>
          <button class="btn quiet sm" data-tip="Chèn trích dẫn từ brief" data-tip-sub="Chỉ dùng được trích dẫn đã gắn vào brief">${I("quote")}Trích dẫn</button>
          <button class="btn quiet sm" data-tip="Gợi ý hashtag theo hồ sơ kênh">${I("hash")}Hashtag</button>
          <span class="spacer"></span>
          <button class="btn quiet sm" data-action="ai-focus" data-tip="Nhờ AI sửa đoạn đang chọn" data-tip-sub="Tạo đề xuất; bạn quyết định áp dụng">${I("sparkles")}Sửa đoạn chọn</button>
        </div>
        <div class="editor-body md" contenteditable="true" role="textbox" aria-multiline="true" aria-label="Nội dung caption Facebook" spellcheck="false">
          <p>${text}</p>
          <p>Local Content Studio giữ mọi tài nguyên, thương hiệu và bản quyền trên máy bạn, từ nghiên cứu đến bản xuất cuối cùng.</p>
          <p>👉 Dùng thử miễn phí tại localstudio.vn</p>
        </div>
        <div class="editor-foot">
          <span class="row xs"><span class="${st === "blocked" ? "" : "muted"}">212 / 300 ký tự khuyến nghị</span></span>
          <div style="width:120px">${UI.progress(70, "Độ dài so với khuyến nghị")}</div>
          <span class="spacer"></span>
          <span class="chip">${I("hash")}tudonghoa</span><span class="chip">${I("hash")}noidung</span>
          <span class="xs muted">2/3 hashtag</span>
        </div>
      </section>
      <div class="grid-2">
        <div class="field"><label for="w-cta">Lời kêu gọi (CTA)</label><input class="input" id="w-cta" value="Dùng thử miễn phí"><span class="hint">Theo giọng điệu Brand v2</span></div>
        <div class="field"><label for="w-link">Liên kết</label><input class="input" id="w-link" value="https://localstudio.vn/dung-thu?utm_source=facebook"><span class="hint">Đã thêm UTM theo hồ sơ kênh</span></div>
      </div>`;
    const preview = `
      <section class="panel">
        <div class="panel-head"><h2>Xem trước</h2><span class="badge" data-tip="Chỉ là ước lượng bố cục; giao diện nền tảng có thể khác" tabindex="0">${I("info")}Mô phỏng</span></div>
        <div class="panel-body"><div class="fb-mock">
          <div class="row" style="padding:10px"><span class="avatar">LS</span><div class="col" style="gap:0"><strong class="xs">Local Studio</strong><span class="xs muted">Vừa xong · 🌐</span></div></div>
          <div style="padding:0 10px 10px;font-size:13px">🚀 Đội của bạn mất bao lâu cho một carousel? Các đội dùng quy trình tự động hóa giảm 42% thời gian sản xuất nội dung… <span class="muted">Xem thêm</span></div>
          <div class="mock-carousel"><div style="background:#334BD1"></div><div style="background:#EEF2F8"></div></div>
          <div class="xs muted" style="padding:8px 10px">Carousel 6 trang · rev-4</div>
        </div></div>
      </section>`;

    const body = UI.byState(st, {
      loading: () => `${UI.skeletonRows(1, 260)}<div style="height:12px"></div>${UI.skeletonText(3)}`,
      empty: () => `<div class="panel">${UI.empty({ icon: "writing", title: "Chưa có brief cho dự án này", desc: "Brief nêu mục tiêu, đối tượng và trích dẫn nguồn. Mỗi kênh là một biến thể liên kết với brief, không phải bản sao rời rạc.", actions: `<button class="btn primary">${I("plus")}Viết brief</button><button class="btn">${I("file-text")}Bắt đầu từ mẫu</button><button class="btn" data-go="research">${I("quote")}Chọn trích dẫn từ nghiên cứu</button>` })}</div>`,
      error: () => `${UI.callout("warning", "git-compare", "Biến thể vừa được Claude Code (CLI) cập nhật thành rev-5", "Bản bạn đang sửa dựa trên rev-4. Không có gì bị ghi đè; hãy so sánh trước khi lưu.", `<button class="btn sm primary">${I("git-compare")}So sánh và gộp</button><button class="btn sm">Giữ bản của tôi thành nhánh mới</button><button class="btn sm">Bỏ thay đổi của tôi</button>`)}<div style="height:16px"></div><div class="writing-grid">${`<div class="col-3">${editor}</div>`}${preview}</div>`,
      blocked: () => `${UI.callout("danger", "alert-circle", "Có từ cấm theo Brand v2: “cam kết 100%”", "Biến thể vẫn lưu được nhưng không thể đưa vào gói bàn giao cho tới khi sửa.", `<button class="btn sm primary">Thay bằng “giảm tới 42%”</button><button class="btn sm">Xem quy tắc thương hiệu</button>`)}<div style="height:16px"></div><div class="writing-grid"><div class="col-3">${editor}</div>${preview}</div>`,
      normal: () => `<div class="writing-grid"><div class="col-3">${editor}</div>${preview}</div>`,
    });
    return `<div class="page">${header}<div class="page-body">${body}</div></div>`;
  },
  inspector({ st, D }) {
    if (st === "empty" || st === "loading") return null;
    const sel = st === "blocked" ? 2 : 0;
    const brief = `<div class="insp-content" style="padding-top:12px">
        <div class="field"><label>Mục tiêu</label><span>Tăng đăng ký dùng thử gói tự động hóa</span></div>
        <div class="field"><label>Đối tượng</label><span>Chủ doanh nghiệp nhỏ, quản lý marketing</span></div>
        <div class="field"><label>Thông điệp chính</label><span>Làm nội dung nhanh hơn mà vẫn đúng thương hiệu và bản quyền.</span></div>
        <hr class="divider"><span class="label">Trích dẫn đã gắn</span>
        ${D.evidence.filter((e) => e.used).map((e, i) => `<div class="cite-card"><span class="mono xs strong">[${i + 1}]</span><span class="xs">“${e.text}”<br><span class="muted">example.com · ${e.where}</span></span></div>`).join("")}
        <button class="btn sm" data-go="research">${I("plus")}Thêm trích dẫn</button>
      </div>`;
    const qa = (v, t, d) => `<div class="row" style="align-items:flex-start;gap:8px;padding:6px 0"><span style="color:var(--cs-${v})">${I(v === "success" ? "check-circle" : v === "warning" ? "alert" : "x-circle")}</span><div class="col" style="gap:0"><span>${t}</span>${d ? `<span class="xs muted">${d}</span>` : ""}</div></div>`;
    const checks = `<div class="insp-content" style="padding-top:12px">
        ${qa("success", "Độ dài phù hợp", "212 / 300 ký tự khuyến nghị cho Facebook")}
        ${st === "blocked" ? qa("danger", "1 từ cấm", "“cam kết 100%” ở đoạn 1") : qa("success", "Không có từ cấm")}
        ${qa("success", "1 trích dẫn hợp lệ", "Nguồn đã thu thập, hash khớp")}
        ${qa("success", "Có lời kêu gọi hành động")}
        ${qa("warning", "Liên kết chưa kiểm tra được", "Đang offline; sẽ kiểm khi có mạng")}
        ${qa("success", "Giọng điệu", "Xưng “bạn”, câu ngắn")}
      </div>`;
    const ai = st === "blocked"
      ? `<div class="insp-content" style="padding-top:12px">${UI.callout("neutral", "sparkles", "Đang ở chế độ không dùng AI", "Mọi thao tác soạn thảo vẫn làm được thủ công.", `<button class="btn sm" data-go="settings" data-settings="ai">Cấu hình AI</button><button class="btn sm">Dùng mô hình cục bộ</button>`)}</div>`
      : `<div class="insp-content" style="padding-top:12px">
        <div class="field"><label>Phạm vi</label><div class="segmented" data-exclusive><button aria-pressed="true">Đoạn đã chọn</button><button aria-pressed="false">Cả biến thể</button></div></div>
        <div class="field"><label for="ai-q">Yêu cầu</label><textarea class="textarea" id="ai-q" rows="2" style="font-size:13px">Ngắn gọn hơn, giữ số liệu và trích dẫn</textarea></div>
        <div class="row wrap">${["Rút gọn", "Thân thiện hơn", "Thêm số liệu từ brief", "Viết 3 phương án"].map((t) => `<button class="chip" style="cursor:pointer">${t}</button>`).join("")}</div>
        <div class="xs muted row">${I("cloud")}gpt-4.1-mini · đám mây · ~640 token</div>
        <button class="btn primary block" data-action="ai-disclosure">${I("sparkles")}Tạo đề xuất</button>
        <hr class="divider">
        <div class="row"><span class="label">Đề xuất #2</span><span class="spacer"></span><span class="xs muted">23:09 · 1,8 giây</span></div>
        <div class="diff md"><span class="diff-del">Các đội dùng quy trình tự động hóa giảm 42% thời gian sản xuất nội dung</span> <span class="diff-add">Tự động hóa giúp đội bạn rút 42% thời gian làm nội dung</span><sup>[1]</sup>.</div>
        <div class="row"><button class="btn sm primary" data-action="ai-apply">${I("check")}Áp dụng</button><button class="btn sm" data-action="ai-reject">Bỏ</button><span class="spacer"></span><button class="btn sm quiet icon" aria-label="Tạo lại" data-tip="Tạo lại với cùng yêu cầu">${I("refresh")}</button></div>
        <span class="xs muted">Áp dụng sẽ sửa bản nháp; revision chỉ tạo khi bạn lưu.</span>
      </div>`;
    return {
      title: "Chi tiết",
      body: `${UI.tabs("wi", Object.assign([{ label: "Brief" }, { label: "Kiểm tra", count: st === "blocked" ? 1 : 0 }, { label: "AI", icon: "sparkles" }], { sel }), "sm")}
        ${UI.tabPanel("wi", 0, brief, sel)}${UI.tabPanel("wi", 1, checks, sel)}${UI.tabPanel("wi", 2, ai, sel)}`,
    };
  },
});

window.MENUS["writing-add"] = () => [
  { label: "Biến thể kênh", group: true },
  { icon: "publishing", text: "Instagram (đã có)", disabled: "Đã có biến thể Instagram" },
  { icon: "publishing", text: "LinkedIn (đã có)", disabled: "Đã có biến thể LinkedIn" },
  { icon: "publishing", text: "TikTok" },
  { icon: "publishing", text: "Threads" },
  "-",
  { icon: "file-text", text: "Brief mới" },
  { icon: "file-text", text: "Bài viết dài" },
];
ACTIONS["ai-focus"] = () => { const t = document.querySelector("#wi-t2"); if (t) { t.click(); document.querySelector("#ai-q")?.focus(); } };
ACTIONS["ai-apply"] = () => UI.toast({ type: "success", title: "Đã áp dụng đề xuất vào bản nháp", desc: "Nhấn Ctrl+Z để hoàn tác.", actionLabel: "Hoàn tác", action: () => {} });
ACTIONS["ai-reject"] = () => UI.toast({ title: "Đã bỏ đề xuất", desc: "Bản nháp không thay đổi." });
