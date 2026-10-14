use studio_app::{
    research_extractor::ResearchExtractor, research_validator::SafeUrlValidator, ApplicationHost,
    IngestUrlInput,
};
use studio_core::{
    actor::ActorContext, capabilities::CapabilityRegistry, operation::OperationEnvelope,
};
use studio_storage::{config::StorageConfig, initialize_storage};
use tempfile::tempdir;

#[test]
fn test_ssrf_validator_blocks_private_and_local_ips() {
    // 1. Loopback addresses
    assert!(SafeUrlValidator::validate_url("http://localhost:3000/api").is_err());
    assert!(SafeUrlValidator::validate_url("http://127.0.0.1/admin").is_err());
    assert!(SafeUrlValidator::validate_url("http://127.0.0.254:8080").is_err());

    // 2. Private RFC1918 ranges
    assert!(SafeUrlValidator::validate_url("http://10.0.0.1/internal").is_err());
    assert!(SafeUrlValidator::validate_url("http://192.168.1.1/router").is_err());
    assert!(SafeUrlValidator::validate_url("http://172.16.0.1/secret").is_err());

    // 3. Link-local and zero addresses
    assert!(SafeUrlValidator::validate_url("http://169.254.169.254/metadata").is_err());
    assert!(SafeUrlValidator::validate_url("http://0.0.0.0/").is_err());

    // 4. Bad scheme
    assert!(SafeUrlValidator::validate_url("file:///etc/passwd").is_err());
    assert!(SafeUrlValidator::validate_url("gopher://evil.com").is_err());

    // 5. Valid public URL
    let ok_url = SafeUrlValidator::validate_url("https://example.com/blog/article-1");
    assert!(ok_url.is_ok());
    assert_eq!(ok_url.unwrap().host_str(), Some("example.com"));
}

#[test]
fn test_research_extractor_evidence_spans_and_hash() {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Xu hướng Marketing AI 2026</title>
            <meta name="description" content="Khảo sát xu hướng ứng dụng AI trong sáng tạo nội dung">
        </head>
        <body>
            <header><nav>Home | About</nav></header>
            <main>
                <h1>Xu hướng Marketing AI 2026</h1>
                <h2>1. Nội dung đa định dạng</h2>
                <p>Năm 2026 chứng kiến sự bùng nổ của việc sản xuất nội dung đa định dạng trên các nền tảng xã hội.</p>
                <p>Người làm nội dung chuyển đổi từ viết tay sang mô hình phối hợp giữa con người và AI agents có kiểm soát.</p>
                <h2>2. Bảo vệ bản quyền</h2>
                <p>Quy trình xác minh bản quyền (rights attestation) trở thành bắt buộc trước khi bàn giao ấn phẩm xuất bản.</p>
            </main>
            <script>alert("malicious script should be stripped");</script>
            <style>body { color: red; }</style>
        </body>
        </html>
    "#;

    let snapshot = ResearchExtractor::extract_from_html(html, "https://example.com/ai-trends");

    assert_eq!(snapshot.title, "Xu hướng Marketing AI 2026");
    assert_eq!(
        snapshot.description,
        Some("Khảo sát xu hướng ứng dụng AI trong sáng tạo nội dung".to_string())
    );
    assert_eq!(snapshot.canonical_url, "https://example.com/ai-trends");
    assert_eq!(snapshot.headings.len(), 3);
    assert_eq!(snapshot.headings[0], "Xu hướng Marketing AI 2026");
    assert_eq!(snapshot.headings[1], "1. Nội dung đa định dạng");
    assert_eq!(snapshot.headings[2], "2. Bảo vệ bản quyền");

    // Script tag was stripped
    assert!(!snapshot.main_text.contains("malicious script"));
    assert!(!snapshot.main_text.contains("color: red"));

    // Evidence spans created
    assert_eq!(snapshot.evidence_spans.len(), 3);
    assert!(snapshot.evidence_spans[0]
        .text
        .contains("Năm 2026 chứng kiến sự bùng nổ"));
    assert_eq!(snapshot.source_hash.len(), 64);
}

#[test]
fn test_research_ingest_e2e_and_offline_catalog() {
    let temp = tempdir().unwrap();
    let config = StorageConfig::new(temp.path());
    let (catalog, blob_storage, _) = initialize_storage(&config).unwrap();
    let caps = CapabilityRegistry::default();
    let host = ApplicationHost::new(catalog, blob_storage, caps);

    let actor = ActorContext::new_human("user-creator", "ws-brand-1");

    let sample_html = r#"
        <html>
        <head><title>Nghiên cứu thị trường F&B</title></head>
        <body>
            <h1>Thị trường F&B Việt Nam</h1>
            <p>Thị trường F&B Việt Nam đạt mức tăng trưởng 12% hàng năm với xu hướng tiêu dùng xanh.</p>
        </body>
        </html>
    "#;

    let op = OperationEnvelope::new(
        "research.ingest_url",
        IngestUrlInput {
            url: "https://example.org/market-report".into(),
        },
    );

    let res = host.ingest_url_content(&actor, op, sample_html).unwrap();
    let source = res.result;

    assert_eq!(source.workspace_id, "ws-brand-1");
    assert_eq!(source.canonical_url, "https://example.org/market-report");
    assert_eq!(source.domain, "example.org");
    assert_eq!(source.fetch_status, "COMPLETED");

    let snap = source.snapshot.expect("Snapshot must exist");
    assert_eq!(snap.title, "Nghiên cứu thị trường F&B");
    assert_eq!(snap.evidence_spans.len(), 1);

    // Verify offline retrieval from catalog
    let retrieved = host.get_research_source(&actor, &source.id).unwrap();
    assert_eq!(retrieved.id, source.id);
    assert_eq!(retrieved.canonical_url, source.canonical_url);

    // Cross-workspace isolation check
    let other_actor = ActorContext::new_human("intruder", "ws-brand-2");
    let forbidden_err = host.get_research_source(&other_actor, &source.id);
    assert!(forbidden_err.is_err());
}
