(function () {
  const SC = [
    { n: "Mở đầu", t0: 0, t1: 5, c: "#334BD1" },
    { n: "Vấn đề", t0: 5, t1: 11, c: "#5568d8" },
    { n: "Giải pháp", t0: 11, t1: 18, c: "#7f93e8", warn: "Phụ đề dùng phông chưa cài" },
    { n: "Số liệu", t0: 18, t1: 25, c: "#5568d8" },
    { n: "Kêu gọi", t0: 25, t1: 30, c: "#334BD1" },
  ];
  const sel = 2;
  PAGES.register({
    id: "motion",
    label: "Chuyển động",
    sidebar({ st }) {
      if (st === "empty") return null;
      return {
        title: "Storyboard",
        actions: `<button class="btn quiet sm icon" aria-label="Thêm cảnh" data-tip="Thêm cảnh">${I("plus")}</button>`,
        body: `<div class="list" data-selectable>${SC.map((s, i) => `
          <button class="list-item two-line scene-item ${i === sel ? "is-selected" : ""}">
            <span class="scene-thumb" style="background:${s.c}">${i + 1}</span>
            <span class="lines"><span>${s.n}</span><span class="xs muted mono">00:${String(s.t0).padStart(2, "0")} – 00:${String(s.t1).padStart(2, "0")}</span></span>
            ${s.warn ? `<span class="meta" style="color:var(--cs-warning)" data-tip="${s.warn}">${I("alert")}</span>` : ""}
          </button>`).join("")}</div>`,
        foot: `<div class="row xs"><span class="muted">Tổng</span><span class="spacer"></span><span class="mono">00:30.0</span></div>`,
      };
    },
    main({ st }) {
      const header = UI.pageHeader({
        title: "Video giới thiệu 30 giây",
        meta: `1080 × 1920 ${UI.metaSep} 30 fps ${UI.metaSep} rev-1 ${UI.metaSep} Brand v2`,
        actions: st === "empty" ? "" : `<button class="btn" data-tip="Xuất từng cảnh thành PNG" data-tip-sub="Luôn khả dụng, không cần FFmpeg">${I("image")}Xuất storyboard</button>
          ${st === "blocked" ? `<div class="unavailable"><button class="btn primary" disabled>${I("video")}Render MP4</button><span class="reason">${I("alert")}Chưa tìm thấy FFmpeg · <a href="#" data-go="settings" data-settings="caps">Cài đặt</a></span></div>`
            : `<button class="btn primary" data-action="render-mp4" data-tip="Render từ revision đã lưu" data-tip-sub="H.264 + AAC · kiểm tra codec và hash sau khi xong">${I("video")}Render MP4</button>`}`,
      });
      const ruler = Array.from({ length: 31 }, (_, i) => `<span style="left:${(i / 30) * 100}%">${i % 5 === 0 ? `<b>${i}s</b>` : ""}</span>`).join("");
      const track = (name, icon, clips, extra = "") => `<div class="track"><div class="track-head">${I(icon)}<span class="truncate">${name}</span><span class="spacer"></span>${extra}</div><div class="track-lane">${clips}</div></div>`;
      const clip = (t0, t1, label, cls = "", tip = "") => `<button class="clip ${cls}" style="left:${(t0 / 30) * 100}%;width:${((t1 - t0) / 30) * 100}%" ${tip ? `data-tip="${tip}"` : ""}>${label}</button>`;
      const timeline = `<section class="timeline panel" aria-label="Dòng thời gian">
          <div class="timeline-bar">
            <button class="btn quiet sm icon" aria-label="Về đầu" data-tip="Về đầu" data-kbd="Home">${I("skip-back")}</button>
            <button class="btn quiet sm icon" aria-label="Phát" data-tip="Phát / dừng" data-kbd="Space">${I("play")}</button>
            <button class="btn quiet sm icon" aria-label="Tới cuối" data-tip="Tới cuối" data-kbd="End">${I("skip-fwd")}</button>
            <span class="mono xs">00:12.4 / 00:30.0</span>
            <span class="vdivider"></span>
            <button class="btn quiet sm icon" aria-label="Cắt tại đầu phát" data-tip="Cắt tại đầu phát" data-kbd="S">${I("scissors")}</button>
            <span class="spacer"></span>
            <label class="xs muted hide-sm" for="tl-zoom">Thu phóng</label><input id="tl-zoom" type="range" min="1" max="5" value="2" style="width:100px" aria-label="Thu phóng dòng thời gian">
          </div>
          <div class="tracks">
            <div class="ruler"><div class="track-head"></div><div class="track-lane">${ruler}</div></div>
            ${track("Cảnh", "video", SC.map((s, i) => clip(s.t0, s.t1, s.n, i === sel ? "is-selected" : "", `${s.n} · ${s.t1 - s.t0} giây`)).join(""))}
            ${track("Chữ", "type", clip(0.5, 4.5, "Tiêu đề") + clip(6, 10.5, "Câu hỏi") + clip(11.5, 17.5, "Giảm 42%") + clip(18.5, 24.5, "3 lợi ích") + clip(25.5, 29.5, "Dùng thử"))}
            ${track("Phụ đề", "captions", clip(0, 30, "Phụ đề tiếng Việt · 14 dòng", "warn", "Phông Be Vietnam Pro chưa cài"))}
            ${track("Âm thanh", "audio", clip(0, 30, "intro-nhac-nen.mp3 · −18 dB", "audio"), `<button class="btn quiet sm icon" aria-label="Tắt tiếng" data-tip="Tắt tiếng track">${I("audio")}</button>`)}
            <div class="playhead" style="left:calc(140px + (100% - 140px) * ${12.4 / 30})" aria-hidden="true"></div>
          </div>
        </section>`;
      const player = `<div class="player panel">
          <div class="player-stage"><div class="phone" style="background:${SC[sel].c}">
            <div style="font-weight:800;font-size:28px;line-height:1.1;color:#fff">Giảm 42% thời gian làm nội dung</div>
            <div class="sub-line">Tự động hóa từ nghiên cứu đến bản xuất</div>
          </div></div>
          <div class="row xs muted" style="padding:8px 12px;border-top:1px solid var(--cs-border-subtle)"><span>Bản proxy 540p · không phải bản render cuối</span><span class="spacer"></span><span>Cảnh 3/5</span></div>
        </div>`;
      const body = UI.byState(st, {
        loading: () => `${UI.skeletonRows(1, 300)}<div style="height:12px"></div>${UI.skeletonRows(4, 36)}`,
        empty: () => `<div class="panel">${UI.empty({ icon: "motion", title: "Chưa có storyboard", desc: "Tạo video mẫu từ kịch bản, từ carousel có sẵn, hoặc thêm cảnh thủ công. Không phải trình dựng phim đầy đủ: tập trung vào slideshow, chữ động và phụ đề.", actions: `<button class="btn primary">${I("plus")}Thêm cảnh</button><button class="btn">${I("design")}Tạo từ carousel Q4</button><button class="btn">${I("file-text")}Tạo từ kịch bản</button>` })}</div>`,
        error: () => `${UI.callout("danger", "x-circle", "Lần render gần nhất thất bại ở cảnh 3", "Thiếu phông “Be Vietnam Pro” cho phụ đề. Không có tệp đầu ra lỗi nào được lưu.", `<button class="btn sm primary" data-action="job-error">Xem chi tiết</button><button class="btn sm">Đổi phụ đề sang Inter</button>`)}<div class="motion-grid">${player}</div>${timeline}`,
        blocked: () => `${UI.callout("warning", "cpu", "Không tìm thấy FFmpeg trên máy", "Vẫn sửa được storyboard, xem trước từng cảnh và xuất PNG. Render MP4 cần FFmpeg 6 trở lên.", `<button class="btn sm primary">Chỉ định đường dẫn FFmpeg…</button><button class="btn sm">Hướng dẫn cài đặt</button>`)}<div class="motion-grid">${player}</div>${timeline}`,
        normal: () => `${UI.callout("neutral", "info", "Chuyển động nâng cao chưa khả dụng", "Mẫu dùng FFmpeg (đã kiểm chứng). Motion Canvas chưa cài nên hiệu ứng theo mã không hiện trong danh sách.")}<div class="motion-grid">${player}</div>${timeline}`,
      });
      return `<div class="page">${header}<div class="page-body"><div class="col-3">${body}</div></div></div>`;
    },
    inspector({ st }) {
      if (st === "empty" || st === "loading") return null;
      return {
        title: "Cảnh 3 · Giải pháp",
        body: `
          <details class="insp-section" open><summary>${I("chev-right", "chev")}Thời lượng</summary><div class="insp-content">
            <div class="grid-2"><label class="num-field" data-tip="Bắt đầu (giây)"><span>⇤</span><input value="11.0"></label><label class="num-field" data-tip="Thời lượng (giây)"><span>↔</span><input value="7.0"></label></div>
            <div class="field"><label>Chuyển cảnh vào</label><select class="select sm"><option>Mờ dần · 0,4 giây</option><option>Cắt thẳng</option><option>Trượt trái</option><option disabled>Theo mã (cần Motion Canvas)</option></select></div></div></details>
          <details class="insp-section" open><summary>${I("chev-right", "chev")}Nội dung cảnh</summary><div class="insp-content">
            <div class="field"><label>Bố cục</label><select class="select sm"><option>Tiêu đề lớn + nền màu</option><option>Ảnh toàn màn + chú thích</option><option>Số liệu nổi bật</option></select></div>
            <div class="field"><label>Chữ</label><input class="input sm" value="Giảm 42% thời gian làm nội dung"></div>
            <div class="field"><label>Hiệu ứng chữ</label><select class="select sm"><option>Trượt lên · 0,6 giây</option><option>Hiện dần</option><option>Không</option></select></div>
            <div class="field"><label>Nền</label><div class="token-select"><span class="swatch" style="background:#7f93e8;width:20px;height:20px"></span><select class="select sm"><option>brand.primary 60%</option></select></div></div></div></details>
          <details class="insp-section" open><summary>${I("chev-right", "chev")}Phụ đề<span class="spacer"></span><span style="color:var(--cs-warning)">${I("alert")}</span></summary><div class="insp-content">
            <textarea class="textarea" rows="2" style="font-size:13px;min-height:56px">Tự động hóa từ nghiên cứu đến bản xuất</textarea>
            <span class="error-text">${I("alert-circle")}Phông “Be Vietnam Pro” chưa cài</span>
            <div class="row"><button class="btn sm">Dùng Inter</button><button class="btn sm quiet">Cài phông…</button></div></div></details>
          <details class="insp-section"><summary>${I("chev-right", "chev")}Cấu hình render</summary><div class="insp-content">
            <dl class="kv xs"><dt>Định dạng</dt><dd>MP4 · H.264 + AAC</dd><dt>Khổ</dt><dd>1080 × 1920 · 30 fps</dd><dt>Bitrate</dt><dd>8 Mbps (mặc định)</dd><dt>Engine</dt><dd>FFmpeg 8.1.1 · đã kiểm chứng</dd></dl>
            <select class="select sm" aria-label="Hồ sơ tài nguyên"><option>Cân bằng · 2 luồng</option><option>Nhanh · dùng toàn bộ CPU</option><option>Nhẹ · 1 luồng</option></select></div></details>`,
      };
    },
  });
  ACTIONS["render-mp4"] = () => UI.confirm({
    title: "Render MP4 từ rev-1?",
    body: `${UI.callout("warning", "alert", "Cảnh 3 có cảnh báo", "Phụ đề dùng phông chưa cài. Nếu tiếp tục, render sẽ thất bại ở cảnh 3.")}<p class="xs muted">Gợi ý: đổi phụ đề sang Inter trước khi render.</p>`,
    confirmLabel: "Vẫn render",
    onConfirm: () => UI.toast({ type: "info", title: "Đã xếp hàng render MP4", actionLabel: "Mở Task Center", action: () => ACTIONS.tasks() }),
  });
})();
