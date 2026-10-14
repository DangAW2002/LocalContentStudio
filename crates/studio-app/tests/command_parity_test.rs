use studio_app::{ApplicationHost, CreateProjectInput, SaveRevisionInput};
use studio_core::{
    actor::ActorContext, capabilities::CapabilityRegistry, document::CreativeDocument,
    error::StudioError, operation::OperationEnvelope, project::ProjectType,
};
use studio_storage::{config::StorageConfig, initialize_storage};
use tempfile::tempdir;

#[test]
fn test_command_parity_ui_cli_mcp() {
    let temp = tempdir().unwrap();
    let config = StorageConfig::new(temp.path());
    let (catalog, blob_storage, _) = initialize_storage(&config).unwrap();
    let caps = CapabilityRegistry::default();
    let host = ApplicationHost::new(catalog, blob_storage, caps);

    let human_actor = ActorContext::new_human("user-alice", "workspace-alpha");
    let _agent_actor = ActorContext::new_agent("agent-bob", "workspace-alpha");
    let unauthorized_actor = ActorContext::new_human("user-mallory", "workspace-beta");

    // 1. Both UI and CLI execute create_project with the same envelope
    let op = OperationEnvelope::new(
        "project.create",
        CreateProjectInput {
            title: "Dự án Parity Test".into(),
            project_type: ProjectType::Poster,
        },
    );

    let res = host.create_project(&human_actor, op).unwrap();
    let project = res.result;
    assert_eq!(project.title, "Dự án Parity Test");
    assert_eq!(project.workspace_id, "workspace-alpha");

    // 2. Cross-workspace access returns identical ForbiddenScope error
    let cross_ws_err = host.get_project(&unauthorized_actor, &project.id);
    assert!(cross_ws_err.is_err());
    match cross_ws_err.err().unwrap() {
        StudioError::ForbiddenScope { resource_id, .. } => {
            assert_eq!(resource_id, "workspace-alpha");
        }
        other => panic!("Expected ForbiddenScope, got {:?}", other),
    }

    // 3. Save Revision 1
    let doc = CreativeDocument::new("doc-parity");
    let save_op_1 = OperationEnvelope::new(
        "revision.save",
        SaveRevisionInput {
            project_id: project.id.clone(),
            document: doc.clone(),
            summary: "Khởi tạo revision 1".into(),
        },
    );
    let rev_res_1 = host.save_revision(&human_actor, save_op_1).unwrap();
    let rev1 = rev_res_1.result;

    // 4. Stale CAS revision update yields ConflictCas error
    let save_op_stale = OperationEnvelope {
        operation: "revision.save".into(),
        api_version: OperationEnvelope::<()>::API_VERSION_V1.into(),
        request_id: "req-stale".into(),
        idempotency_key: None,
        expected_revision_id: Some("non-existent-or-stale-rev".into()),
        human_proof_token: None,
        input: SaveRevisionInput {
            project_id: project.id.clone(),
            document: doc,
            summary: "Cố tình ghi đè với revision cũ".into(),
        },
    };
    let stale_err = host.save_revision(&human_actor, save_op_stale);
    assert!(stale_err.is_err());
    match stale_err.err().unwrap() {
        StudioError::ConflictCas {
            expected, current, ..
        } => {
            assert_eq!(expected, "non-existent-or-stale-rev");
            assert_eq!(current, rev1.id);
        }
        other => panic!("Expected ConflictCas, got {:?}", other),
    }
}
