PAGES.register({
  id: "research",
  label: "Nghiên cứu",
  sidebar({ st, D }) {
    if (st === "loading") return { title: "Nguồn", body: `<div style="padding:12px">${UI.skeletonRows(4, 40)}</div>` };
    if (st === "empty") return { title: "Nguồn", body: `<div class="xs muted" style="padding:16px">Chưa có nguồn nào.</div>` };
    return {
      title: "Nguồn",
      actions: `<button class="btn quiet sm icon" aria-label="Lọc nguồn" data-tip="Lọc theo trạng thái, miền">${I("filter")}</button>`,
      body: `<div style="padding:8px"><div class="input-affix">${I("search")}<input class="input sm" placeholder="Tìm trong nguồn và trích dẫn" aria-label="Tìm nguồn"></div></div>
        <div class="list" role="listbox" aria-label="Danh sách nguồn" data-selectable>
          ${D.sources.map((s, i) => `
            <button class="list-item two-line ${i === (st === "blocked" ? 2 : 0) ? "is-selected" : ""}" role="option" aria-selected="${i === 0}">
              <span style="color:var(--cs-${s.state === "FETCHED" ? "success" : "danger"})">${I(s.state === "FETCHED" ? "globe" : "shield-alert")}</span>
              <span class="lines"><span class="truncate">${s.title}</span><span class="xs muted truncate">${s.host} · ${s.fetched}</span></span>
              ${s.evidence ? `<span class="meta" data-tip="${s.evidence} trích dẫn">${s.evidence}</span>` : ""}
            </button>`).join("")}
        </div>`,
      foot: `<button class="btn block" data-tip="HTML, PDF, Markdown, TXT" data-tip-sub="Đi qua cùng bộ kiểm tra an toàn">${I("file-plus")}Nhập tệp cục bộ…</button>`,
    };
  },
  main({ st, D }) {
    const s = D.sources[0];
    const form = `
      <form class="page-toolbar research-form" onsubmit="event.preventDefault();UI.toast({type:'info',title:'Đã bắt đầu thu thập',desc:'Theo dõi số trang và dung lượng thật ở bên dưới.'})">
        <div class="input-affix" style="flex:1 1 320px">${I("globe")}<input class="input" type="url" placeholder="Dán URL cần thu thập, ví dụ https://example.com/bao-cao" aria-label="URL nguồn" ${st === "error" ? 'value="https://slow.example.net/report" aria-invalid="true"' : ""}></div>
        <div class="segmented" aria-label="Chế độ thu thập">
          <button type="button" aria-pressed="true" data-tip="Chỉ lấy đúng trang này">Một trang</button>
          <button type="button" aria-pressed="false" data-tip="Theo liên kết cùng miền" data-tip-sub="Tối đa 10 trang, độ sâu 2">Crawl giới hạn</button>
        </div>
        <button type="button" class="btn icon" data-menu="research-opts" aria-haspopup="menu" aria-label="Tùy chọn thu thập" data-tip="Giới hạn: 10 MB · 20 giây · 5 chuyển hướng">${I("settings")}</button>
        <button class="btn primary" type="submit">${I("download")}Thu thập</button>
      </form>`;

    const header = UI.pageHeader({
      title: "Nghiên cứu",
      meta: `${D.sources.length} nguồn ${UI.metaSep} 11 trích dẫn ${UI.metaSep} chỉ mục toàn văn FTS5`,
      actions: st === "normal" ? `<button class="btn" data-tip="Thu thập lại với cùng giới hạn" data-tip-sub="Bản cũ được giữ để so sánh">${I("refresh")}Thu thập lại</button>
        <button class="btn primary" data-go="writing" data-tip="Đưa 2 trích dẫn đã chọn vào brief">${I("quote")}Gắn vào brief (2)</button>` : "",
    });

    const running = `<div class="job-strip" role="status" aria-live="polite">
        ${I("loader")}<span><strong>Đang thu thập research.example.org</strong> <span class="muted">· 3/10 trang · 412 KB / 10 MB · 6 giây</span></span>
        <div style="flex:1;max-width:240px">${UI.progress(30, "Tiến độ thu thập")}</div><button class="btn sm">Hủy</button></div>`;

    const body = UI.byState(st, {
      loading: () => `<div class="page-body">${UI.skeletonText(2)}<div style="height:16px"></div>${UI.skeletonRows(5)}</div>`,
      empty: () => `<div class="page-body">${UI.empty({
          icon: "globe", title: "Chưa có nguồn nghiên cứu",
          desc: "Dán URL ở ô phía trên để lấy văn bản, số liệu và hình ảnh. Studio giới hạn dung lượng, thời gian và chặn địa chỉ nội bộ.",
          actions: `<button class="btn">${I("file-plus")}Nhập tệp cục bộ</button><button class="btn quiet">${I("help")}Studio thu thập những gì?</button>`,
        })}</div>`,
      error: () => `<div class="page-body col-4">${UI.callout("danger", "clock", "Hết thời gian chờ sau 20 giây", "Máy chủ slow.example.net không trả lời trong giới hạn. Không có dữ liệu nào được lưu.", `<button class="btn sm primary">${I("refresh")}Thử lại</button><button class="btn sm">Tăng giới hạn lên 60 giây</button><button class="btn sm">${I("file-plus")}Lưu trang thủ công rồi nhập tệp</button>`)}</div>`,
      blocked: () => `<div class="page-body col-4">
          ${UI.callout("danger", "shield-alert", "Đã chặn http://192.168.1.20/admin", "URL trỏ tới mạng nội bộ (dải 192.168.0.0/16). Studio không truy cập địa chỉ nội bộ, localhost hoặc link-local để tránh lộ dữ liệu trong mạng của bạn.", `<button class="btn sm">Sửa URL</button><button class="btn sm">${I("file-plus")}Nhập tệp cục bộ thay thế</button>`)}
          <dl class="kv panel" style="padding:16px"><dt>URL nhập</dt><dd class="mono">http://192.168.1.20/admin</dd><dt>Phân giải DNS</dt><dd class="mono">192.168.1.20 (riêng tư)</dd><dt>Quy tắc</dt><dd>Chặn dải riêng tư · kiểm trước khi kết nối và sau mỗi chuyển hướng</dd><dt>Lúc</dt><dd>Hôm qua 09:51</dd></dl>
        </div>`,
      normal: () => `
        ${running}
        <div class="page-body">
          <div class="col-4">
            ${UI.callout("neutral", "alert", "Nội dung web là dữ liệu chưa tin cậy", "Văn bản thu thập không bao giờ được dùng làm chỉ dẫn cho AI. Quyền sử dụng hình ảnh phải được bạn xác nhận riêng.")}
            <section class="panel">
              <div class="panel-head"><h2 class="truncate">${s.title}</h2>${UI.badge("FETCHED")}<button class="btn quiet sm icon" data-tip="Mở trang gốc trong trình duyệt" aria-label="Mở trang gốc">${I("external")}</button></div>
              <div class="panel-body"><dl class="kv kv-2">
                <dt>URL</dt><dd><a href="#" class="mono xs">${s.url}</a></dd>
                <dt>Thu thập</dt><dd>${s.fetched} · ${s.bytes} · text/html · 1 chuyển hướng</dd>
                <dt>SHA-256</dt><dd><span class="hash copyable" data-copy="${s.hash}c9e0" data-tip="Nhấn để sao chép">${s.hash}…c9e0</span></dd>
                <dt>robots.txt</dt><dd>Cho phép · <span class="muted">Điều khoản sử dụng: chưa xác định, cần bạn kiểm tra</span></dd>
              </dl></div>
            </section>

            <section class="panel">
              ${UI.tabs("rs", [{ label: "Trích dẫn", icon: "quote", count: 4 }, { label: "Cấu trúc", icon: "list" }, { label: "Hình ảnh", icon: "image", count: 2 }, { label: "Văn bản gốc", icon: "file-text" }, { label: "Liên kết", icon: "link", count: 18 }])}
              ${UI.tabPanel("rs", 0, `<div class="list">${D.evidence.map((e, i) => `
                <div class="list-item two-line evidence" style="cursor:default">
                  <input type="checkbox" ${e.used ? "checked" : ""} aria-label="Chọn trích dẫn ${i + 1}" style="margin-top:4px">
                  <span class="lines"><span class="md">“${e.text}”</span><span class="xs muted">${e.where} · <a href="#">xem trong văn bản gốc</a></span></span>
                  <span class="item-actions"><button class="btn sm quiet icon" aria-label="Sao chép" data-tip="Sao chép kèm nguồn" data-copy="${e.text} (Nguồn: ${s.host})">${I("copy")}</button></span>
                </div>`).join("")}</div>`)}
              ${UI.tabPanel("rs", 1, `<div class="panel-body"><ol class="outline">
                  <li>Tổng quan thị trường nội dung Q4</li><li>Năng suất đội nội dung<ol><li>Tác động của tự động hóa</li><li>Thời gian sản xuất trung bình</li></ol></li><li>Nhất quán thương hiệu đa kênh</li><li>Định dạng hiệu quả</li><li>Bản quyền và tuân thủ</li></ol></div>`)}
              ${UI.tabPanel("rs", 2, `<div class="panel-body grid-3">${[["bieu-do-q4.png", "1200 × 800", 200], ["nhom-lam-viec.jpg", "1600 × 1067", 30]].map(([n, d, h]) => `
                  <div class="asset-card"><div class="thumb" style="aspect-ratio:4/3;background:hsl(${h} 40% 72%)">${I("image", "icon-xl")}</div>
                  <div class="col" style="padding:8px 10px;gap:2px"><span class="truncate strong xs">${n}</span><span class="xs muted">${d} · nguồn: ${s.host}</span></div>
                  <div style="padding:0 10px 10px"><button class="btn sm block" data-tip="Tệp vào trạng thái cách ly" data-tip-sub="Bạn cần xác nhận quyền trước khi dùng">${I("import")}Nhập vào thư viện</button></div></div>`).join("")}</div>`)}
              ${UI.tabPanel("rs", 3, `<div class="panel-body md source-text"><p><strong>Tổng quan thị trường nội dung Q4.</strong> Năm nay, các đội nội dung tiếp tục chịu áp lực sản xuất nhiều định dạng hơn với nguồn lực không đổi…</p><p><mark>Các đội áp dụng quy trình tự động hóa giảm 42% thời gian sản xuất nội dung.</mark> Tuy nhiên, phần lớn vẫn kiểm duyệt thủ công…</p></div>`)}
              ${UI.tabPanel("rs", 4, `<div class="panel-body xs muted">18 liên kết cùng miền · 6 liên kết ngoài. Chọn để thu thập tiếp trong chế độ crawl giới hạn.</div>`)}
            </section>
          </div>
        </div>`,
    });
    return `<div class="page">${header}${form}${body}</div>`;
  },
});

window.MENUS["research-opts"] = () => [
  { label: "Giới hạn thu thập", group: true },
  { icon: "hard-drive", text: "Dung lượng tối đa: 10 MB" },
  { icon: "clock", text: "Thời gian chờ: 20 giây" },
  { icon: "arrow-right", text: "Chuyển hướng tối đa: 5" },
  { icon: "layers", text: "Crawl: 10 trang, độ sâu 2" },
  "-",
  { icon: "shield", text: "Luôn chặn địa chỉ nội bộ", disabled: "Quy tắc an toàn bắt buộc, không tắt được" },
];
