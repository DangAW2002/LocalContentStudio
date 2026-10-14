use studio_app::{ApplicationHost, CreateProjectInput, SaveRevisionInput};
use studio_core::{
    actor::ActorContext,
    capabilities::CapabilityRegistry,
    document::{
        Artboard, ArtboardBackground, BoundingBox, BrandRef, CreativeDocument, Element,
        ElementSemanticRole, ElementStyle, ElementType,
    },
    operation::OperationEnvelope,
    project::ProjectType,
};
use studio_storage::{config::StorageConfig, initialize_storage};
use tempfile::tempdir;

#[test]
fn test_creative_document_carousel_6_artboards_lifecycle() {
    let temp = tempdir().unwrap();
    let config = StorageConfig::new(temp.path());
    let (catalog, blob_storage, _) = initialize_storage(&config).unwrap();
    let caps = CapabilityRegistry::default();
    let host = ApplicationHost::new(catalog, blob_storage, caps);

    let actor = ActorContext::new_human("designer-1", "ws-studio");

    // 1. Create Carousel Project
    let proj_op = OperationEnvelope::new(
        "project.create",
        CreateProjectInput {
            title: "Chiến dịch Ra mắt Sản phẩm Mới (Carousel)".into(),
            project_type: ProjectType::Carousel,
        },
    );
    let project = host.create_project(&actor, proj_op).unwrap().result;

    // 2. Build 6 Artboards Carousel Document
    let mut doc = CreativeDocument::new(&project.id);
    doc.brand_ref = Some(BrandRef {
        id: "brand-core-1".into(),
        version: 2,
    });

    for idx in 1..=6 {
        let mut elements = Vec::new();

        // Element 1: Headline text
        elements.push(Element {
            id: format!("artboard-{}-headline", idx),
            element_type: ElementType::Text,
            semantic_role: ElementSemanticRole::Headline,
            text: Some(format!("Thông điệp trang số {}", idx)),
            asset_ref: None,
            box_model: BoundingBox {
                x: 80.0,
                y: 100.0,
                w: 920.0,
                h: 120.0,
            },
            style: ElementStyle {
                font_token: Some("brand.font.heading".into()),
                color_token: Some("brand.color.primary".into()),
                font_size: Some(48),
                text_align: Some("left".into()),
            },
            locked: false,
            visible: true,
        });

        // Element 2: Product Image slot
        elements.push(Element {
            id: format!("artboard-{}-image", idx),
            element_type: ElementType::Image,
            semantic_role: ElementSemanticRole::ProductImage,
            text: None,
            asset_ref: Some(format!("asset-img-{}", idx)),
            box_model: BoundingBox {
                x: 80.0,
                y: 260.0,
                w: 920.0,
                h: 600.0,
            },
            style: ElementStyle {
                font_token: None,
                color_token: None,
                font_size: None,
                text_align: None,
            },
            locked: false,
            visible: true,
        });

        // Element 3: CTA text
        elements.push(Element {
            id: format!("artboard-{}-cta", idx),
            element_type: ElementType::Text,
            semantic_role: ElementSemanticRole::Cta,
            text: Some(if idx == 6 {
                "Nhấp link bio để mua ngay!".into()
            } else {
                "Vuốt tiếp để khám phá tính năng →".into()
            }),
            asset_ref: None,
            box_model: BoundingBox {
                x: 80.0,
                y: 920.0,
                w: 920.0,
                h: 80.0,
            },
            style: ElementStyle {
                font_token: Some("brand.font.body".into()),
                color_token: Some("brand.color.secondary".into()),
                font_size: Some(28),
                text_align: Some("center".into()),
            },
            locked: false,
            visible: true,
        });

        doc.artboards.push(Artboard {
            id: format!("artboard-{}", idx),
            width: 1080,
            height: 1350, // Standard 4:5 Instagram Carousel aspect ratio
            background: ArtboardBackground {
                token: "brand.surface".into(),
            },
            elements,
        });
    }

    assert_eq!(doc.artboards.len(), 6);
    let hash_before = doc.compute_hash();
    assert_eq!(hash_before.len(), 64);

    // 3. Save Revision 1
    let save_op_1 = OperationEnvelope::new(
        "revision.save",
        SaveRevisionInput {
            project_id: project.id.clone(),
            document: doc.clone(),
            summary: "Khởi tạo 6 trang carousel hoàn chỉnh".into(),
        },
    );
    let rev1 = host.save_revision(&actor, save_op_1).unwrap().result;
    assert_eq!(rev1.source_hash, hash_before);

    // 4. Reload from offline SQLite catalog and verify exact hierarchy
    let loaded_doc = host
        .get_document_by_revision(&actor, &project.id, &rev1.id)
        .unwrap();
    assert_eq!(loaded_doc.artboards.len(), 6);
    assert_eq!(loaded_doc.compute_hash(), hash_before);

    for (idx, board) in loaded_doc.artboards.iter().enumerate() {
        assert_eq!(board.id, format!("artboard-{}", idx + 1));
        assert_eq!(board.width, 1080);
        assert_eq!(board.height, 1350);
        assert_eq!(board.elements.len(), 3);
        assert_eq!(
            board.elements[0].semantic_role,
            ElementSemanticRole::Headline
        );
        assert_eq!(
            board.elements[1].semantic_role,
            ElementSemanticRole::ProductImage
        );
        assert_eq!(board.elements[2].semantic_role, ElementSemanticRole::Cta);
    }

    // 5. Modify headline on artboard 1 and save Revision 2 with CAS
    let mut modified_doc = loaded_doc;
    modified_doc.artboards[0].elements[0].text = Some("Thông điệp mở màn đã được chỉnh sửa".into());
    let hash_after = modified_doc.compute_hash();
    assert_ne!(hash_before, hash_after);

    let mut save_op_2 = OperationEnvelope::new(
        "revision.save",
        SaveRevisionInput {
            project_id: project.id.clone(),
            document: modified_doc,
            summary: "Cập nhật tiêu đề trang 1".into(),
        },
    );
    save_op_2.expected_revision_id = Some(rev1.id.clone());

    let rev2 = host.save_revision(&actor, save_op_2).unwrap().result;
    assert_eq!(rev2.parent_id, Some(rev1.id.clone()));
    assert_eq!(rev2.source_hash, hash_after);
}
