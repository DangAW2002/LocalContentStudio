use studio_app::{ApplicationHost, CreateProjectInput, SaveRevisionInput};
use studio_core::{
    actor::ActorContext,
    capabilities::CapabilityRegistry,
    document::{
        Artboard, ArtboardBackground, BoundingBox, CreativeDocument, Element, ElementSemanticRole,
        ElementStyle, ElementType,
    },
    operation::OperationEnvelope,
    preview::PreviewFidelity,
    project::ProjectType,
};
use studio_storage::{config::StorageConfig, initialize_storage};
use tempfile::tempdir;

#[test]
fn test_unified_preview_receipt_and_static_export() {
    let temp = tempdir().unwrap();
    let config = StorageConfig::new(temp.path());
    let (catalog, blob_storage, _) = initialize_storage(&config).unwrap();
    let caps = CapabilityRegistry::default();
    let host = ApplicationHost::new(catalog, blob_storage, caps);

    let actor = ActorContext::new_human("creator-art", "ws-creative");

    // 1. Create Poster project
    let proj_op = OperationEnvelope::new(
        "project.create",
        CreateProjectInput {
            title: "Poster Hội Thảo Công Nghệ".into(),
            project_type: ProjectType::Poster,
        },
    );
    let project = host.create_project(&actor, proj_op).unwrap().result;

    // 2. Build Document with 1 Artboard
    let mut doc = CreativeDocument::new(&project.id);
    doc.artboards.push(Artboard {
        id: "artboard-main".into(),
        width: 1080,
        height: 1080,
        background: ArtboardBackground {
            token: "brand.surface".into(),
        },
        elements: vec![
            Element {
                id: "el-title".into(),
                element_type: ElementType::Text,
                semantic_role: ElementSemanticRole::Headline,
                text: Some("Hội Thảo Trí Tuệ Nhân Tạo 2026".into()),
                asset_ref: None,
                box_model: BoundingBox {
                    x: 60.0,
                    y: 80.0,
                    w: 960.0,
                    h: 120.0,
                },
                style: ElementStyle {
                    font_token: Some("brand.font.heading".into()),
                    color_token: Some("brand.color.primary".into()),
                    font_size: Some(44),
                    text_align: Some("center".into()),
                },
                locked: false,
                visible: true,
            },
            Element {
                id: "el-key-visual".into(),
                element_type: ElementType::Image,
                semantic_role: ElementSemanticRole::ProductImage,
                text: None,
                asset_ref: Some("asset-keyvisual-1".into()),
                box_model: BoundingBox {
                    x: 60.0,
                    y: 220.0,
                    w: 960.0,
                    h: 680.0,
                },
                style: ElementStyle {
                    font_token: None,
                    color_token: None,
                    font_size: None,
                    text_align: None,
                },
                locked: false,
                visible: true,
            },
        ],
    });

    let save_op = OperationEnvelope::new(
        "revision.save",
        SaveRevisionInput {
            project_id: project.id.clone(),
            document: doc,
            summary: "Lưu bản thảo poster hoàn chỉnh".into(),
        },
    );
    let rev = host.save_revision(&actor, save_op).unwrap().result;

    // 3. Test Preview Receipt fidelity
    // A. Saved revision gets EXACT fidelity
    let exact_receipt = host
        .get_preview_receipt(&actor, &project.id, &rev.id, false)
        .unwrap();
    assert_eq!(exact_receipt.fidelity, PreviewFidelity::Exact);
    assert!(!exact_receipt.is_draft);
    assert_eq!(exact_receipt.revision_id, rev.id);
    assert_eq!(exact_receipt.output_hash.len(), 64);
    assert!(exact_receipt.warnings.is_empty());

    // B. Unsaved draft preview gets ESTIMATE fidelity with warnings
    let draft_receipt = host
        .get_preview_receipt(&actor, &project.id, &rev.id, true)
        .unwrap();
    assert_eq!(draft_receipt.fidelity, PreviewFidelity::Estimate);
    assert!(draft_receipt.is_draft);
    assert!(!draft_receipt.warnings.is_empty());

    // 4. Test SVG Export
    let svg_export = host
        .export_project_svg(&actor, &project.id, &rev.id)
        .unwrap();
    assert_eq!(svg_export.export_format, "svg");
    assert_eq!(svg_export.files.len(), 1);
    assert_eq!(svg_export.files[0].file_name, "artboard-01.svg");
    assert_eq!(svg_export.files[0].mime_type, "image/svg+xml");
    assert!(svg_export.total_bytes > 0);
    assert_eq!(svg_export.files[0].sha256.len(), 64);

    // 5. Test Static Sandboxed HTML Export
    let html_export = host
        .export_project_html(&actor, &project.id, &rev.id)
        .unwrap();
    assert_eq!(html_export.export_format, "html");
    assert_eq!(html_export.files.len(), 1);
    assert_eq!(html_export.files[0].file_name, "index.html");
    assert_eq!(html_export.files[0].mime_type, "text/html; charset=utf-8");
    assert!(html_export.total_bytes > 0);
}
