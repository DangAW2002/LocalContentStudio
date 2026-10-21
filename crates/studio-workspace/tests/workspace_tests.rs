use chrono::{Duration, Utc};
use std::fs;
use std::path::PathBuf;
use studio_workspace::{
    disk_usage, AppPaths, CheckpointService, Indexer, Org, Project, ResolutionLayer, ScopeChain,
    Workspace, WorkspaceDb,
};
use tempfile::tempdir;

#[test]
fn test_app_paths_and_app_db() {
    let temp = tempdir().unwrap();
    let app_paths = AppPaths::new(temp.path());

    let db = app_paths.open_app_db().unwrap();
    db.add_root("D:/Studio/ca-nhan", Some("Cá nhân")).unwrap();
    let roots = db.list_roots().unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].path, "D:/Studio/ca-nhan");
    assert_eq!(roots[0].name.as_deref(), Some("Cá nhân"));

    db.register_org("D:/Studio/ca-nhan", "Cá nhân").unwrap();
    let orgs = db.list_orgs().unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].name, "Cá nhân");

    db.record_workspace_open("D:/Studio/ca-nhan/kenh-review")
        .unwrap();
    let recents = db.list_recent_workspaces().unwrap();
    assert_eq!(recents.len(), 1);
    assert_eq!(recents[0].path, "D:/Studio/ca-nhan/kenh-review");

    db.set_setting("theme", "dark").unwrap();
    assert_eq!(db.get_setting("theme").unwrap().as_deref(), Some("dark"));
}

#[test]
fn test_org_and_workspace_creation_and_open() {
    let temp = tempdir().unwrap();
    let org_dir = temp.path().join("ca-nhan");
    let org = Org::create(&org_dir, "Cá nhân").unwrap();
    assert_eq!(org.name(), "Cá nhân");
    assert!(!org.is_implicit());
    assert!(org_dir.join("org.yaml").exists());
    assert!(org.shared_dir().join("brands").exists());
    assert!(org.shared_dir().join("channels").exists());
    assert!(org.shared_dir().join("playbooks").exists());

    let ws_dir = org_dir.join("kenh-review");
    let ws = Workspace::create(&ws_dir, "Kênh Review").unwrap();
    assert_eq!(ws.name(), "Kênh Review");
    assert_eq!(ws.org().name(), "Cá nhân");
    assert!(ws_dir.join("studio.yaml").exists());
    assert!(ws_dir.join(".studio/blobs").exists());
    assert!(ws_dir.join("brands").exists());
    assert!(ws_dir.join("projects").exists());

    // Project creation
    let proj = Project::create(&ws, "video-01", "Video Đánh Giá 01").unwrap();
    assert_eq!(proj.slug(), "video-01");
    assert_eq!(proj.title(), "Video Đánh Giá 01");
    assert!(proj.root().join("project.yaml").exists());
    assert!(proj.artifacts_dir().exists());
    assert!(proj.renders_dir().exists());

    // Re-open workspace and project
    let reopened_ws = Workspace::open(&ws_dir).unwrap();
    assert_eq!(reopened_ws.name(), "Kênh Review");
    let reopened_proj = Project::open(&reopened_ws, "video-01").unwrap();
    assert_eq!(reopened_proj.title(), "Video Đánh Giá 01");

    let projects = reopened_ws.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].slug(), "video-01");
}

#[test]
fn test_implicit_org_when_no_org_yaml() {
    let temp = tempdir().unwrap();
    let ws_dir = temp.path().join("standalone-ws");
    let ws = Workspace::create(&ws_dir, "Độc Lập").unwrap();
    assert!(ws.org().is_implicit());
    assert_eq!(ws.org().name(), "Cá nhân (Mặc định)");
}

#[test]
fn test_scope_chain_override_resolution() {
    let temp = tempdir().unwrap();
    let app_dir = temp.path().join("app_data");
    let app_paths = AppPaths::new(&app_dir);

    let org_dir = temp.path().join("my-org");
    let _org = Org::create(&org_dir, "My Org").unwrap();

    let ws_dir = org_dir.join("my-ws");
    let ws = Workspace::create(&ws_dir, "My Workspace").unwrap();

    // 1. Setup channel item in App: app_data/shared/channels/youtube.yaml
    let app_channels = app_paths.shared_dir().join("channels");
    fs::create_dir_all(&app_channels).unwrap();
    fs::write(app_channels.join("youtube.yaml"), "name: App Youtube").unwrap();

    // 2. Setup channel item in Org: org/shared/channels/tiktok.yaml and youtube.yaml (overriding App)
    let org_channels = org_dir.join("shared/channels");
    fs::write(org_channels.join("tiktok.yaml"), "name: Org TikTok").unwrap();
    fs::write(org_channels.join("youtube.yaml"), "name: Org Youtube").unwrap();

    // 3. Setup channel item in Workspace: ws/channels/youtube.yaml (overriding Org)
    let ws_channels = ws_dir.join("channels");
    fs::write(ws_channels.join("youtube.yaml"), "name: Workspace Youtube").unwrap();
    fs::write(
        ws_channels.join("instagram.yaml"),
        "name: Workspace Instagram",
    )
    .unwrap();

    let chain = ScopeChain::with_app_paths(&ws, app_paths);
    let resolved = chain.resolve("channels");

    // We expect:
    // instagram -> Workspace
    // tiktok -> Org
    // youtube -> Workspace (overriding Org and App)
    assert_eq!(resolved.len(), 3);

    let ig = resolved.iter().find(|i| i.id == "instagram").unwrap();
    assert_eq!(ig.layer, ResolutionLayer::Workspace);

    let tt = resolved.iter().find(|i| i.id == "tiktok").unwrap();
    assert_eq!(tt.layer, ResolutionLayer::Org);

    let yt = resolved.iter().find(|i| i.id == "youtube").unwrap();
    assert_eq!(yt.layer, ResolutionLayer::Workspace);
    assert_eq!(yt.path, ws_channels.join("youtube.yaml"));
}

#[test]
fn test_path_scoping_and_escape_rejection() {
    let temp = tempdir().unwrap();
    let org_dir = temp.path().join("org");
    let _org = Org::create(&org_dir, "Org").unwrap();
    let ws_dir = org_dir.join("ws");
    let ws = Workspace::create(&ws_dir, "WS").unwrap();

    // Normal valid relative path
    let valid = ws.scoped_path("brands/main/logo.png").unwrap();
    assert_eq!(valid, ws_dir.join("brands/main/logo.png"));

    // Root-relative and drive-relative forms are not `is_absolute()` on Windows but must be rejected
    assert!(ws.scoped_path("\\Windows\\win.ini").is_err());
    assert!(ws.scoped_path("/etc/passwd").is_err());
    #[cfg(windows)]
    assert!(ws.scoped_path("C:Windows").is_err());

    // A junction in an intermediate directory must not let a new (non-existing) path escape
    #[cfg(windows)]
    {
        let outside = temp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        let link = ws_dir.join("projects").join("evil");
        let made = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&outside)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if made {
            assert!(ws.scoped_path("projects/evil/new/deep/file.txt").is_err());
        }
    }

    // Absolute path rejected
    let abs_path = if cfg!(windows) {
        PathBuf::from("C:\\Windows\\System32")
    } else {
        PathBuf::from("/etc/passwd")
    };
    assert!(ws.scoped_path(abs_path).is_err());

    // Parent escape `..` rejected
    assert!(ws.scoped_path("../outside.txt").is_err());
    assert!(ws.scoped_path("brands/../../outside.txt").is_err());

    // Symlink / Junction test (skip gracefully if OS doesn't allow)
    let outside_dir = temp.path().join("outside_folder");
    fs::create_dir_all(&outside_dir).unwrap();
    fs::write(outside_dir.join("secret.txt"), "secret").unwrap();

    let link_path = ws_dir.join("symlink_test");
    #[cfg(windows)]
    let symlink_result = std::os::windows::fs::symlink_dir(&outside_dir, &link_path);
    #[cfg(unix)]
    let symlink_result = std::os::unix::fs::symlink(&outside_dir, &link_path);

    if symlink_result.is_ok() {
        let escape_res = ws.scoped_path("symlink_test/secret.txt");
        assert!(
            escape_res.is_err(),
            "Symlink pointing outside workspace must be rejected"
        );
    }

    // can_write assertions
    assert!(ws.can_write(ws_dir.join("brands/main")));
    assert!(!ws.can_write(org_dir.join("shared/brands")));
    assert!(!ws.can_write(temp.path().join("other")));
}

#[test]
fn test_migrations_idempotency_and_backup() {
    let temp = tempdir().unwrap();
    let ws_dir = temp.path().join("ws");
    let ws = Workspace::create(&ws_dir, "WS").unwrap();

    // First open runs migrations
    {
        let db = WorkspaceDb::open(&ws).unwrap();
        let version: i64 = db
            .conn()
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(version, 5);
    }

    // Second open must be idempotent
    {
        let db = WorkspaceDb::open(&ws).unwrap();
        let version: i64 = db
            .conn()
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(version, 5);
    }
}

#[test]
fn test_migrations_upgrade_from_v2() {
    let temp = tempdir().unwrap();
    let ws_dir = temp.path().join("ws");
    let ws = Workspace::create(&ws_dir, "WS_UPGRADE").unwrap();
    let db_path = ws.db_path();
    let backups_dir = ws.backups_dir();

    // Create a database stopped at v2
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                checksum TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO schema_migrations (version, checksum, applied_at) VALUES (1, 'v1', '2026-01-01T00:00:00Z'), (2, 'v2', '2026-01-01T00:00:00Z');",
            [],
        )
        .unwrap();
        // Create an existing table to trigger backup
        conn.execute("CREATE TABLE dummy_existing (id INTEGER);", [])
            .unwrap();
    }

    // Now open with WorkspaceDb::open which calls apply_migrations_with_backup
    {
        let db = WorkspaceDb::open(&ws).unwrap();
        let version: i64 = db
            .conn()
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(version, 5);

        // Verify artifact_checks table exists
        let table_exists: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='artifact_checks'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(table_exists, 1);

        // Verify assets_index table exists
        let assets_exists: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='assets_index'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(assets_exists, 1);
    }

    // Verify backup file for v3 or v4 was created
    let backups = std::fs::read_dir(&backups_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(
        backups
            .iter()
            .any(|name| name.contains("studio-pre-v3-") || name.contains("studio-pre-v4-")),
        "Backup before v3/v4 must be created: {:?}",
        backups
    );
}

#[test]
fn test_rebuild_index_after_deleting_db() {
    let temp = tempdir().unwrap();
    let ws_dir = temp.path().join("ws");
    let ws = Workspace::create(&ws_dir, "WS").unwrap();

    // Setup an artifact
    let proj = Project::create(&ws, "demo-proj", "Demo").unwrap();
    let art_dir = proj.artifacts_dir().join("thumb-01");
    fs::create_dir_all(&art_dir).unwrap();
    let art_yaml = "
id: thumb-01
kind: design
title: Thumbnail Video Demo
channels: [youtube, facebook]
source: index.html
inputs:
- artifact:script-01
- ref: artifact:nv-an
  role: reference
";
    fs::write(art_dir.join("artifact.yaml"), art_yaml).unwrap();
    fs::write(
        art_dir.join("index.html"),
        "<h1>Thumbnail title content</h1>",
    )
    .unwrap();

    // Setup an asset in library/
    let lib_dir = ws.root().join("library/images");
    fs::create_dir_all(&lib_dir).unwrap();
    let asset_file = lib_dir.join("logo.png");
    fs::write(&asset_file, b"fake-png-content-123").unwrap();
    fs::write(
        lib_dir.join("logo.png.meta.yaml"),
        "title: Logo Chính\ndescription: Logo thuong hieu chinh\n",
    )
    .unwrap();

    // 1. Initial rebuild
    Indexer::rebuild(&ws).unwrap();

    let check_rows = |ws: &Workspace| {
        let db = WorkspaceDb::open(ws).unwrap();
        let art_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM artifacts_index WHERE id='thumb-01'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(art_count, 1);

        let asset_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM assets_index WHERE name='Logo Chính'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(asset_count, 1);

        // Edges count
        let edge_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM artifact_edges WHERE project='demo-proj' AND from_id='thumb-01'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(edge_count, 2);

        // Verify edge roles
        let ref_role: String = db
            .conn()
            .query_row(
                "SELECT role FROM artifact_edges WHERE project='demo-proj' AND from_id='thumb-01' AND to_ref='artifact:nv-an'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ref_role, "reference");

        // FTS search
        let fts_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM search_fts WHERE search_fts MATCH 'Thumbnail'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(fts_count, 1);
    };

    check_rows(&ws);

    // 2. Delete database file
    let db_path = ws.db_path();
    fs::remove_file(&db_path).unwrap();
    assert!(!db_path.exists());

    // 3. Rebuild and verify exact same rows
    Indexer::rebuild(&ws).unwrap();
    check_rows(&ws);
}

#[test]
fn test_checkpoint_edit_restore_and_diff() {
    let temp = tempdir().unwrap();
    let ws_dir = temp.path().join("ws");
    let ws = Workspace::create(&ws_dir, "WS").unwrap();

    // Create initial files
    let file1 = ws_dir.join("brands/main.txt");
    fs::create_dir_all(file1.parent().unwrap()).unwrap();
    fs::write(&file1, "Original Content 1").unwrap();

    let file2 = ws_dir.join("playbooks/review.md");
    fs::create_dir_all(file2.parent().unwrap()).unwrap();
    fs::write(&file2, "Original Content 2").unwrap();

    // Create Checkpoint A
    let cp_a = CheckpointService::create(&ws, "Checkpoint A", None).unwrap();

    // Modify file1, delete file2, add new file3
    fs::write(&file1, "Modified Content 1").unwrap();
    fs::remove_file(&file2).unwrap();
    let file3 = ws_dir.join("brands/new_file.txt");
    fs::write(&file3, "New File 3").unwrap();

    // Create Checkpoint B
    let cp_b = CheckpointService::create(&ws, "Checkpoint B", None).unwrap();

    // Diff Checkpoint A vs Checkpoint B
    let diff = CheckpointService::diff(&ws, &cp_a, &cp_b).unwrap();
    assert_eq!(diff.added, vec!["brands/new_file.txt"]);
    assert_eq!(diff.modified, vec!["brands/main.txt"]);
    assert_eq!(diff.deleted, vec!["playbooks/review.md"]);

    // Restore to Checkpoint A:
    // - file1 should be restored to "Original Content 1"
    // - file2 should be restored to "Original Content 2"
    // - file3 (created after Checkpoint A) should be deleted!
    CheckpointService::restore(&ws, &cp_a).unwrap();

    assert_eq!(fs::read_to_string(&file1).unwrap(), "Original Content 1");
    assert_eq!(fs::read_to_string(&file2).unwrap(), "Original Content 2");
    assert!(
        !file3.exists(),
        "New file created after checkpoint must be deleted on restore"
    );
}

#[test]
fn test_checkpoint_gc_and_pin() {
    let temp = tempdir().unwrap();
    let ws_dir = temp.path().join("ws");
    let ws = Workspace::create(&ws_dir, "WS").unwrap();

    let test_file = ws_dir.join("brands/test.txt");
    fs::create_dir_all(test_file.parent().unwrap()).unwrap();
    fs::write(&test_file, "data").unwrap();

    let cp1 = CheckpointService::create(&ws, "CP 1 (recent)", None).unwrap();
    let cp2 = CheckpointService::create(&ws, "CP 2 (pinned old)", None).unwrap();
    let cp3 = CheckpointService::create(&ws, "CP 3 (old daily candidate)", None).unwrap();
    let cp4 = CheckpointService::create(&ws, "CP 4 (old daily duplicate)", None).unwrap();

    CheckpointService::pin(&ws, &cp2, true).unwrap();

    let now = Utc::now();
    // Artificially age checkpoints in DB
    {
        let db = WorkspaceDb::open(&ws).unwrap();
        // cp2 is 40 days old but pinned
        let t_cp2 = (now - Duration::days(40)).to_rfc3339();
        db.conn()
            .execute(
                "UPDATE checkpoints SET created_at = ?1 WHERE id = ?2",
                [&t_cp2, &cp2],
            )
            .unwrap();

        // cp3 and cp4 are same day (15 days old) -> only one should be kept
        let t_cp3 = (now - Duration::days(15)).to_rfc3339();
        let t_cp4 = (now - Duration::days(15) + Duration::hours(2)).to_rfc3339();
        db.conn()
            .execute(
                "UPDATE checkpoints SET created_at = ?1 WHERE id = ?2",
                [&t_cp3, &cp3],
            )
            .unwrap();
        db.conn()
            .execute(
                "UPDATE checkpoints SET created_at = ?1 WHERE id = ?2",
                [&t_cp4, &cp4],
            )
            .unwrap();
    }

    let deleted = CheckpointService::gc(&ws, now).unwrap();
    assert_eq!(
        deleted, 1,
        "Expected exactly 1 duplicate daily checkpoint deleted"
    );

    let list = CheckpointService::list(&ws).unwrap();
    let ids: Vec<String> = list.into_iter().map(|c| c.id).collect();
    println!("KEPT IDS: {:?}", ids);
    println!("cp1={}, cp2={}, cp3={}, cp4={}", cp1, cp2, cp3, cp4);

    assert!(
        ids.contains(&cp1),
        "Recent checkpoint (< 7 days) must be kept"
    );
    assert!(ids.contains(&cp2), "Pinned checkpoint must always be kept");
    assert!(
        (ids.contains(&cp3) && !ids.contains(&cp4)) || (!ids.contains(&cp3) && ids.contains(&cp4)),
        "Exactly one of the duplicate daily checkpoints must be kept and one pruned"
    );
}

#[test]
fn test_disk_usage() {
    let temp = tempdir().unwrap();
    let ws_dir = temp.path().join("ws");
    let ws = Workspace::create(&ws_dir, "WS").unwrap();

    fs::write(ws_dir.join("brands/logo.txt"), "12345").unwrap(); // 5 bytes
    fs::write(ws_dir.join("library/clip.mp4"), "1234567890").unwrap(); // 10 bytes

    let usage = disk_usage(&ws).unwrap();
    assert!(usage.sources_bytes >= 5);
    assert_eq!(usage.library_bytes, 10);
    assert!(usage.total_bytes >= 15);
}

#[test]
fn test_recent_workspaces_no_duplicate_verbatim_case_trailing_slash() {
    let temp = tempdir().unwrap();
    let app_paths = AppPaths::new(temp.path().join("app_data"));
    let app_db = app_paths.open_app_db().unwrap();

    let ws_dir = temp.path().join("my-cool-workspace");
    let _ws = Workspace::create(&ws_dir, "Cool WS").unwrap();

    // 1. Normal path
    let spelling1 = ws_dir.to_string_lossy().to_string();
    // 2. Verbatim with trailing slash
    let spelling2 = format!(r"\\?\{}\", spelling1);
    // 3. Case difference (on Windows) or with forward slashes
    let spelling3 = format!("{}/", spelling1.to_uppercase());

    app_db.record_workspace_open(&spelling1).unwrap();
    app_db.record_workspace_open(&spelling2).unwrap();
    app_db.record_workspace_open(&spelling3).unwrap();

    let recents = app_db.list_recent_workspaces().unwrap();
    assert_eq!(
        recents.len(),
        1,
        "Recent workspaces must list the folder exactly once regardless of verbatim prefix, case, or trailing slash"
    );

    // Path must be clean and normalized
    assert!(!recents[0].path.starts_with(r"\\?\"));
    assert!(!recents[0].path.ends_with('\\'));
    assert!(!recents[0].path.ends_with('/'));
}

#[test]
fn test_turn_undo_workspace_level_semantics() {
    let temp = tempdir().unwrap();
    let ws_dir = temp.path().join("undo-ws");
    let ws = Workspace::create(&ws_dir, "Undo Test Workspace").unwrap();

    // Pre-turn files
    let file1_path = ws_dir.join("file1.txt");
    let untouched_path = ws_dir.join("untouched.txt");
    fs::write(&file1_path, "version 1 content").unwrap();
    fs::write(&untouched_path, "untouched original content").unwrap();

    let turn_id = "turn-xyz-123";
    let thread_id = "thread-abc-456";

    // Set up turn record in WorkspaceDb
    {
        let db = WorkspaceDb::open(&ws).unwrap();
        db.conn()
            .execute(
                "INSERT INTO threads (id, project_slug, title, engine, created_at, updated_at) VALUES (?1, NULL, ?2, ?3, ?4, ?4)",
                rusqlite::params![thread_id, "Test Thread", "claude", chrono::Utc::now().to_rfc3339()],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO turns (id, thread_id, turn_index, user_prompt, status, created_at) VALUES (?1, ?2, 1, ?3, 'running', ?4)",
                rusqlite::params![turn_id, thread_id, "Make edits", chrono::Utc::now().to_rfc3339()],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO events (id, turn_id, event_type, payload_json, created_at) VALUES (?1, ?2, 'Text', '\"hello\"', ?3)",
                rusqlite::params!["event-1", turn_id, chrono::Utc::now().to_rfc3339()],
            )
            .unwrap();
    }

    // Checkpoint created before turn modifies files
    let _cp =
        CheckpointService::create(&ws, "Pre-turn checkpoint", Some(turn_id.to_string())).unwrap();

    // Turn modifies file1.txt and creates file2.txt
    let file2_path = ws_dir.join("file2.txt");
    fs::write(&file1_path, "version 2 modified by agent").unwrap();
    fs::write(&file2_path, "new file created during turn").unwrap();

    // Verify modified state
    assert_eq!(
        fs::read_to_string(&file1_path).unwrap(),
        "version 2 modified by agent"
    );
    assert!(file2_path.exists());
    assert_eq!(
        fs::read_to_string(&untouched_path).unwrap(),
        "untouched original content"
    );

    // Perform turn undo
    CheckpointService::undo_turn(&ws, turn_id).unwrap();

    // Verify post-undo state:
    // 1. file1.txt is restored to pre-turn content
    assert_eq!(
        fs::read_to_string(&file1_path).unwrap(),
        "version 1 content",
        "Modified file must be restored to pre-turn checkpoint"
    );
    // 2. file2.txt is deleted
    assert!(
        !file2_path.exists(),
        "File created during the turn must be deleted"
    );
    // 3. untouched.txt is not changed
    assert_eq!(
        fs::read_to_string(&untouched_path).unwrap(),
        "untouched original content",
        "Untouched file must remain unchanged"
    );

    // 4. DB state: events deleted, turn status is 'undone'
    {
        let db = WorkspaceDb::open(&ws).unwrap();
        let event_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM events WHERE turn_id = ?1",
                [turn_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 0, "Events of undone turn must be deleted");

        let status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = ?1", [turn_id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "undone", "Turn status must be 'undone'");
    }

    // 5. Undoing twice must be refused with a Vietnamese message
    let second_undo = CheckpointService::undo_turn(&ws, turn_id);
    assert!(second_undo.is_err(), "Undoing twice must fail");
    let err_msg = second_undo.unwrap_err().to_string();
    assert!(
        err_msg.contains("Lượt này đã được hoàn tác trước đó."),
        "Error message must be in Vietnamese explaining turn was already undone. Got: {}",
        err_msg
    );
}
