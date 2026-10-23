use std::fs;
use studio_artifacts::{
    check_pinned_modifications_after_turn, graph_create_variant, graph_link, graph_set_notes,
    graph_set_pinned, graph_unlink, graph_update_downstream_plan,
    restore_artifact_files_from_checkpoint, Artifact, ArtifactInput, InputRole,
};
use studio_workspace::{CheckpointService, Project, Workspace};
use tempfile::tempdir;

#[tokio::test]
async fn test_graph_link_and_cycle_rejection() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path(), "Test WS").unwrap();
    let proj = Project::create(&ws, "test-proj", "Test Project").unwrap();

    let _a = Artifact::create(&proj, "art-a", "text", "Art A", None, vec![], None).unwrap();
    let _b = Artifact::create(&proj, "art-b", "text", "Art B", None, vec![], None).unwrap();
    let _c = Artifact::create(&proj, "art-c", "text", "Art C", None, vec![], None).unwrap();

    // Link A -> B (A uses B)
    graph_link(&ws, &proj, "art-a", "art-b", InputRole::Uses).unwrap();
    let a_loaded = Artifact::load(&proj, "art-a").unwrap();
    assert_eq!(a_loaded.inputs.len(), 1);
    assert_eq!(a_loaded.inputs[0].reference, "art-b");
    assert_eq!(a_loaded.inputs[0].role, InputRole::Uses);

    // Link B -> C (B uses C)
    graph_link(&ws, &proj, "art-b", "art-c", InputRole::Uses).unwrap();

    // Link C -> A would form a cycle A -> B -> C -> A
    let res = graph_link(&ws, &proj, "art-c", "art-a", InputRole::Uses);
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(err_msg.contains("chu trình"));

    // Self loop rejection
    let res_self = graph_link(&ws, &proj, "art-c", "art-c", InputRole::Uses);
    assert!(res_self.is_err());
    assert!(res_self.unwrap_err().to_string().contains("vòng lặp"));

    // Checkpoint count verified
    let cps = CheckpointService::list(&ws).unwrap();
    assert!(cps.len() >= 2);
}

#[tokio::test]
async fn test_graph_unlink() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path(), "Test WS").unwrap();
    let proj = Project::create(&ws, "test-proj", "Test Project").unwrap();

    let mut a = Artifact::create(&proj, "art-a", "text", "Art A", None, vec![], None).unwrap();
    let _b = Artifact::create(&proj, "art-b", "text", "Art B", None, vec![], None).unwrap();

    a.inputs
        .push(ArtifactInput::new("artifact:art-b", InputRole::Uses));
    a.save(&proj).unwrap();

    graph_unlink(&ws, &proj, "art-a", "art-b").unwrap();
    let a_loaded = Artifact::load(&proj, "art-a").unwrap();
    assert_eq!(a_loaded.inputs.len(), 0);
}

#[tokio::test]
async fn test_graph_pinned_and_notes() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path(), "Test WS").unwrap();
    let proj = Project::create(&ws, "test-proj", "Test Project").unwrap();

    let _a = Artifact::create(&proj, "art-a", "text", "Art A", None, vec![], None).unwrap();

    // Pin
    graph_set_pinned(&ws, &proj, "art-a", true).unwrap();
    let a_pinned = Artifact::load(&proj, "art-a").unwrap();
    assert!(a_pinned.pinned);

    // Notes
    graph_set_notes(
        &ws,
        &proj,
        "art-a",
        vec!["Lưu ý 1".into(), "Lưu ý 2".into()],
    )
    .unwrap();
    let a_notes = Artifact::load(&proj, "art-a").unwrap();
    assert_eq!(a_notes.notes, vec!["Lưu ý 1", "Lưu ý 2"]);
}

#[tokio::test]
async fn test_graph_create_variant() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path(), "Test WS").unwrap();
    let proj = Project::create(&ws, "test-proj", "Test Project").unwrap();

    let _source =
        Artifact::create(&proj, "thumb", "design", "Thumbnail", None, vec![], None).unwrap();
    let variant = graph_create_variant(
        &ws,
        &proj,
        "thumb",
        "thumb-var",
        Some("Thumbnail 9:16".into()),
    )
    .unwrap();

    assert_eq!(variant.id, "thumb-var");
    assert_eq!(variant.title, "Thumbnail 9:16");
    assert_eq!(variant.inputs.len(), 1);
    assert_eq!(variant.inputs[0].reference, "artifact:thumb");
    assert_eq!(variant.inputs[0].role, InputRole::VariantOf);
}

#[tokio::test]
async fn test_downstream_plan_ordering_and_pinned_skips() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path(), "Test WS").unwrap();
    let proj = Project::create(&ws, "test-proj", "Test Project").unwrap();

    // script -> voice (generation/paid) -> timeline (free)
    let _script = Artifact::create(&proj, "script", "text", "Script", None, vec![], None).unwrap();

    let mut voice =
        Artifact::create(&proj, "voice", "generation", "Voice", None, vec![], None).unwrap();
    voice
        .inputs
        .push(ArtifactInput::new("artifact:script", InputRole::Uses));
    voice.save(&proj).unwrap();

    let mut timeline = Artifact::create(
        &proj,
        "timeline",
        "timeline",
        "Timeline",
        None,
        vec![],
        None,
    )
    .unwrap();
    timeline
        .inputs
        .push(ArtifactInput::new("artifact:voice", InputRole::Uses));
    timeline.pinned = true; // Pinned!
    timeline.save(&proj).unwrap();

    let plan = graph_update_downstream_plan(&ws, &proj, "script").unwrap();
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].id, "voice");
    assert!(plan.steps[0].paid);
    assert!(!plan.steps[0].pinned);
    assert!(!plan.steps[0].skipped);

    assert_eq!(plan.steps[1].id, "timeline");
    assert!(plan.steps[1].pinned);
    assert!(plan.steps[1].skipped);
    assert_eq!(
        plan.steps[1].skip_reason.as_deref(),
        Some("Đã khóa (pinned)")
    );

    // The UI reads these fields in camelCase (api/types.ts DownstreamPlan).
    let json = serde_json::to_value(&plan).unwrap();
    assert_eq!(json["targetRef"], "script");
    assert!(json["freeStepsCount"].is_u64());
    assert!(json["paidStepsCount"].is_u64());
    assert!(json["totalEstimatedCostUsd"].is_number());
    assert_eq!(json["steps"][1]["skipReason"], "Đã khóa (pinned)");
}

#[tokio::test]
async fn test_post_turn_pinned_diff_detection_and_restore() {
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path(), "Test WS").unwrap();
    let proj = Project::create(&ws, "test-proj", "Test Project").unwrap();

    let mut art = Artifact::create(
        &proj,
        "script-pinned",
        "text",
        "Pinned Script",
        None,
        vec![],
        None,
    )
    .unwrap();
    art.pinned = true;
    art.save(&proj).unwrap();

    // Initial content
    let script_file = art.dir(&proj).join("source.txt");
    fs::write(&script_file, "Gốc ban đầu").unwrap();

    // Create pre-turn checkpoint
    let pre_cp_id = CheckpointService::create(&ws, "Trước lượt", Some("turn-123".into())).unwrap();

    // Agent modifies the pinned artifact file
    fs::write(&script_file, "Agent đã sửa trái phép").unwrap();

    // 1. Check pinned modifications after turn
    let modified = check_pinned_modifications_after_turn(&ws, &pre_cp_id).unwrap();
    assert_eq!(modified, vec!["script-pinned"]);

    // 2. Restore pinned artifact files
    restore_artifact_files_from_checkpoint(&ws, "test-proj", "script-pinned", &pre_cp_id).unwrap();

    // Content must be reverted to original
    let restored_content = fs::read_to_string(&script_file).unwrap();
    assert_eq!(restored_content, "Gốc ban đầu");
}
