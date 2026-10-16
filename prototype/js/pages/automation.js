(function () {
  const S = { tab: "runs" };
  const TABS = [["workflows", "automation", "Quy trình"], ["runs", "activity", "Lần chạy"], ["policy", "shield", "Quyền công cụ"], ["agents", "terminal", "Agent CLI & MCP"], ["audit", "history", "Nhật ký kiểm toán"]];

  PAGES.register({
    id: "automation",
    label: "Tự động hóa & AI",
    sidebar() {
      return {
        title: "Tự động hóa",
        body: `<div class="list">${TABS.map(([k, ic, t]) => `<button class="list-item" data-atab="${k}" ${S.tab === k ? 'aria-current="page"' : ""}>${I(ic)}<span>${t}</span>${k === "runs" ? `<span class="meta"><span class="badge warning" style="height:18px">1 chờ bạn</span></span>` : ""}</button>`).join("")}</div>`,
      };
    },
    main({ st, D }) {
      const titles = Object.fromEntries(TABS.map(([k, , t]) => [k, t]));
      const header = UI.pageHeader({
        title: titles[S.tab],
        meta: `AI: gpt-4.1-mini (đám mây, hỏi trước khi gửi) ${UI.metaSep} Agent dùng chung quyền với giao diện`,
        actions: S.tab === "workflows" ? `<button class="btn primary">${I("plus")}Tạo quy trình từ mẫu</button>` : S.tab === "audit" ? `<button class="btn">${I("download")}Xuất CSV</button>` : "",
      });
      const chain = (w) => `<div class="chain">${w.steps.map((s, i) => `<span class="chain-step ${w.gates.includes(i) ? "gate" : ""}" ${w.gates.includes(i) ? `data-tip="Cổng duyệt của người dùng" data-tip-sub="Quy trình dừng chờ bạn quyết định" tabindex="0"` : ""}>${w.gates.includes(i) ? I("fingerprint") : ""}${s}</span>${i < w.steps.length - 1 ? `<span class="chain-arrow">${I("chev-right")}</span>` : ""}`).join("")}</div>`;
      const workflows = `<div class="col-3">${D.workflows.map((w) => `
          <section class="panel"><div class="panel-head"><h2>${w.name}</h2><span class="xs muted">${w.runs} lần chạy</span>
            <button class="btn sm" data-tip="Xem và sửa các bước">Sửa</button><button class="btn sm primary" data-action="wf-run">${I("play")}Chạy…</button></div>
            <div class="panel-body">${chain(w)}</div></section>`).join("")}
          <span class="xs muted">Quy trình chạy tuần tự, có checkpoint; khởi động lại sau sự cố sẽ tiếp tục từ bước đã lưu. ${I("fingerprint")} là cổng cần người dùng duyệt.</span></div>`;
      const stepRow = (state, name, detail, act = "") => `<div class="run-step ${state}">
          <span class="run-dot">${I({ done: "check", wait: "fingerprint", run: "loader", todo: "circle", fail: "x" }[state])}</span>
          <div class="col" style="gap:0;flex:1;min-width:0"><span class="strong">${name}</span><span class="xs muted">${detail}</span></div>${act}</div>`;
      const runs = `<div class="grid-runs">
          <section class="panel"><div class="panel-head"><h2>Lần chạy</h2></div><div class="list" data-selectable>
            ${[["wait", "URL → Carousel nháp #3", "Chờ bạn duyệt brief · 23:02"], ["done", "Brief → Biến thể đa kênh #5", "Xong · 22:30 · 4 bước"], ["fail", "Kịch bản → Video mẫu #1", "Lỗi ở Render MP4 · 22:41"], ["done", "URL → Carousel nháp #2", "Xong · hôm qua"]].map(([s, t, d], i) => `
              <button class="list-item two-line ${i === 0 ? "is-selected" : ""}"><span style="color:var(--cs-${s === "wait" ? "warning" : s === "fail" ? "danger" : "success"})">${I(s === "wait" ? "fingerprint" : s === "fail" ? "x-circle" : "check-circle")}</span><span class="lines"><span>${t}</span><span class="xs muted">${d}</span></span></button>`).join("")}
          </div></section>
          <section class="panel"><div class="panel-head"><h2>URL → Carousel nháp #3</h2><button class="btn sm danger-outline" data-tip="Dừng quy trình" data-tip-sub="Các bước đã xong được giữ lại">Hủy lần chạy</button></div>
            <div class="panel-body col">
              ${st === "error" ? UI.callout("danger", "alert-circle", "Ứng dụng đóng đột ngột lúc 23:04", "Lần chạy dừng ở bước 3. Checkpoint gần nhất: bước 2 (Gói nghiên cứu, đã lưu).", `<button class="btn sm primary">${I("play")}Tiếp tục từ checkpoint</button><button class="btn sm">Chạy lại từ đầu</button>`) : ""}
              ${st === "blocked" ? UI.callout("neutral", "sparkles", "Bước 4 cần AI nhưng đang ở chế độ không dùng AI", "Bạn có thể tự viết 3 biến thể rồi đánh dấu bước hoàn tất, hoặc cấu hình AI.", `<button class="btn sm primary">Tự làm bước này</button><button class="btn sm" data-go="settings" data-settings="ai">Cấu hình AI</button>`) : ""}
              ${stepRow("done", "1. Thu thập URL", "example.com/trends-q4 · 184 KB · hash 8f4a21c9")}
              ${stepRow("done", "2. Gói nghiên cứu", "4 trích dẫn · 2 hình ảnh (cách ly) · checkpoint")}
              ${stepRow("wait", "3. Brief · cần bạn duyệt", "AI đề xuất mục tiêu, đối tượng, thông điệp", `<button class="btn sm primary" data-go="writing">Xem và duyệt</button>`)}
              ${stepRow("todo", "4. 3 biến thể caption", "gpt-4.1-mini · ước tính ~1.900 token")}
              ${stepRow("todo", "5. Carousel nháp · cần bạn duyệt", "Mẫu “Số liệu nổi bật” · Brand v2")}
              ${stepRow("todo", "6. Xem trước", "renderer studio-svg 0.3")}
              ${stepRow("todo", "7. Xuất PNG · cần bạn duyệt", "Chỉ xuất khi mọi tài nguyên đủ quyền")}
              <hr class="divider"><dl class="kv xs"><dt>Bắt đầu</dt><dd>23:01 bởi Bạn</dd><dt>Chi phí AI</dt><dd>0,004 USD / giới hạn 0,50 USD</dd><dt>Mã lần chạy</dt><dd><span class="copyable mono" data-copy="run-7d21">run-7d21</span></dd></dl>
            </div></section></div>`;
      const P = { y: ["success", "check", "Cho phép"], d: ["info", "git-compare", "Tạo đề xuất"], h: ["warning", "fingerprint", "Chỉ người dùng"], n: ["danger", "x", "Chặn"] };
      const cell = (k) => `<td><span class="badge ${P[k][0]}">${I(P[k][1])}${P[k][2]}</span></td>`;
      const policy = `${UI.callout("info", "shield", "Một chính sách cho mọi đường vào", "Giao diện, CLI, MCP và DSH đi qua cùng lệnh ứng dụng. Agent không thể xác nhận quyền tài nguyên, tự duyệt, tự đăng hay đọc khóa bí mật. Chính sách này cố định trong phiên bản hiện tại.")}
        <div style="height:12px"></div>
        <section class="panel" style="overflow-x:auto"><table class="table"><thead><tr><th>Loại thao tác</th><th>Bạn (giao diện)</th><th>Agent qua CLI</th><th>Agent qua MCP</th><th>DSH connector</th></tr></thead><tbody>
          ${[["Đọc dữ liệu trong workspace", "y", "y", "y", "y"], ["Tạo bản nháp", "y", "y", "y", "y"], ["Sửa tài liệu (có thể hoàn tác)", "y", "d", "d", "d"], ["Xóa, thu hồi, ghi đè", "h", "n", "n", "n"], ["Xác nhận quyền tài nguyên", "h", "n", "n", "n"], ["Phát hành phiên bản thương hiệu", "h", "n", "n", "n"], ["Bàn giao sang Social", "h", "n", "n", "n"], ["Chạy lệnh shell trên máy", "n", "n", "n", "n"]].map(([t, ...c]) => `<tr><td class="strong">${t}</td>${c.map(cell).join("")}</tr>`).join("")}
        </tbody></table></section>`;
      const agents = `<div class="col-4">
          <section class="panel"><div class="panel-head"><h2>Agent CLI trên máy</h2><button class="btn sm">${I("refresh")}Dò lại</button></div>
            <table class="table"><thead><tr><th>Công cụ</th><th>Phiên bản</th><th>Trạng thái</th><th><span class="sr-only">Hành động</span></th></tr></thead><tbody>
              <tr><td class="strong">Claude Code</td><td class="mono xs">2.4.1</td><td>${UI.badge("CAP_PROPOSED", { label: "Phát hiện · chưa smoke test" })}</td><td class="num"><button class="btn sm">Chạy smoke test</button></td></tr>
              <tr><td class="strong">Gemini CLI</td><td class="mono xs">0.9.0</td><td>${UI.badge("CAP_PROPOSED", { label: "Phát hiện · chưa smoke test" })}</td><td class="num"><button class="btn sm">Chạy smoke test</button></td></tr>
              <tr><td class="strong">Codex CLI</td><td class="muted">—</td><td>${UI.badge("CAP_MISSING")}</td><td class="num"><button class="btn sm quiet">Hướng dẫn</button></td></tr>
            </tbody></table></section>
          <section class="panel"><div class="panel-head"><h2>Máy chủ MCP của Studio</h2><label class="switch"><input type="checkbox" checked>Đang bật</label></div>
            <div class="panel-body col-3"><dl class="kv"><dt>Kênh</dt><dd>stdio · chỉ tiến trình cục bộ</dd><dt>Lệnh</dt><dd><span class="copyable mono xs" data-copy="studio-mcp --workspace ws-mkt">studio-mcp --workspace ws-mkt</span></dd></dl>
              <span class="label">Quyền đã cấp</span>
              <table class="table compact"><thead><tr><th>Ứng dụng</th><th>Phạm vi</th><th>Hết hạn</th><th></th></tr></thead><tbody>
                <tr><td>Claude Code</td><td>Marketing Q4 · đọc, tạo nháp, đề xuất sửa</td><td class="xs">Còn 6 giờ</td><td class="num"><button class="btn sm danger-outline">Thu hồi</button></td></tr>
              </tbody></table>
              <button class="btn sm" style="align-self:flex-start">${I("key")}Cấp quyền mới…</button></div></section></div>`;
      const audit = `<div class="page-toolbar" style="margin:-20px -20px 16px;"><div class="segmented" data-exclusive aria-label="Tác nhân"><button aria-pressed="true">Tất cả</button><button aria-pressed="false">${I("user")}Người dùng</button><button aria-pressed="false">${I("bot")}Agent</button><button aria-pressed="false">${I("cpu")}Hệ thống</button></div><span class="spacer"></span><div class="input-affix" style="width:240px">${I("search")}<input class="input sm" placeholder="Lọc theo thao tác, đối tượng" aria-label="Lọc nhật ký"></div></div>
        <section class="panel" style="overflow-x:auto"><table class="table compact"><thead><tr><th>Thời gian</th><th>Tác nhân</th><th>Thao tác</th><th>Đối tượng</th><th>Kết quả</th></tr></thead><tbody>
          ${D.audit.map((a) => `<tr><td class="mono xs">${a.t}</td><td><span class="row">${I(a.kind === "agent" ? "bot" : a.kind === "system" ? "cpu" : "user")}${a.actor}</span></td><td class="mono xs">${a.op}</td><td class="xs">${a.target}</td><td class="xs" style="${a.result.startsWith("Bị") || a.result.startsWith("Thất") ? "color:var(--cs-danger)" : ""}">${a.result}</td></tr>`).join("")}
        </tbody></table></section>`;
      const content = { workflows, runs, policy, agents, audit }[S.tab];
      const body = UI.byState(st, {
        loading: () => UI.skeletonRows(6),
        empty: () => `<div class="panel">${UI.empty({ icon: "automation", title: "Chưa có quy trình nào", desc: "Quy trình nối các bước nghiên cứu, viết, thiết kế, xuất. Bước cần quyết định của bạn luôn dừng lại chờ duyệt.", actions: `<button class="btn primary">${I("plus")}Tạo từ mẫu “URL → Carousel nháp”</button><button class="btn">${I("terminal")}Kết nối agent CLI</button>` })}</div>`,
        normal: () => content,
        error: () => content,
        blocked: () => content,
      });
      return `<div class="page">${header}<div class="page-body">${body}</div></div>`;
    },
    mount(root) {
      root.querySelectorAll("[data-atab]").forEach((b) => b.addEventListener("click", () => { S.tab = b.dataset.atab; APP.rerender(); }));
    },
  });
  ACTIONS["wf-run"] = () => UI.dialog({
    title: "Chạy “URL → Carousel nháp”",
    body: `<div class="field"><label for="wr-url">URL nguồn</label><input class="input" id="wr-url" placeholder="https://…" autofocus></div>
      <div class="grid-2"><div class="field"><label>Thương hiệu</label><select class="select"><option>Brand v2</option></select></div><div class="field"><label>Giới hạn chi phí AI</label><input class="input" value="0,50 USD"></div></div>
      ${UI.callout("info", "fingerprint", "3 cổng duyệt", "Quy trình dừng chờ bạn ở: brief, carousel nháp và xuất PNG.")}`,
    footer: `<span class="foot-note">Dữ liệu gửi tới AI được liệt kê trước mỗi bước.</span><button class="btn" data-close>Hủy</button><button class="btn primary" data-close data-action="wf-started">${I("play")}Bắt đầu</button>`,
  });
  ACTIONS["wf-started"] = () => UI.toast({ type: "info", title: "Đã bắt đầu lần chạy #4", desc: "Theo dõi ở Lần chạy và Task Center." });
})();
