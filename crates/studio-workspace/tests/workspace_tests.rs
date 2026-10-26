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

    let ws_path = temp.path().join("ca-nhan/kenh-review");
    std::fs::create_dir_all(&ws_path).unwrap();
    let ws_path_str = ws_path.to_string_lossy().to_string();
    db.record_workspace_open(&ws_path_str).unwrap();
    let recents = db.list_recent_workspaces().unwrap();
    assert_eq!(recents.len(), 1);
    assert_eq!(
        recents[0].path,
        studio_workspace::normalize_workspace_path(&ws_path_str)
    );

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
fn test_list_recent_workspaces_skips_nonexistent_paths_without_deleting_records() {
    let temp = tempdir().unwrap();
    let app_paths = AppPaths::new(temp.path().join("app_data"));
    let app_db = app_paths.open_app_db().unwrap();

    let existing_dir = temp.path().join("real_ws");
    let _ws = Workspace::create(&existing_dir, "Real WS").unwrap();

    let deleted_dir = temp.path().join("deleted_temp_ws");
    // Do NOT create directory on disk for deleted_dir

    let existing_str = existing_dir.to_string_lossy().to_string();
    let deleted_str = deleted_dir.to_string_lossy().to_string();

    app_db.record_workspace_open(&deleted_str).unwrap();
    app_db.record_workspace_open(&existing_str).unwrap();

    // 1. list_recent_workspaces must skip the non-existent path
    let recents = app_db.list_recent_workspaces().unwrap();
    assert_eq!(
        recents.len(),
        1,
        "Must filter out deleted/non-existent workspaces"
    );
    assert_eq!(
        recents[0].path,
        studio_workspace::normalize_workspace_path(&existing_str)
    );

    // 2. Underlying database records must remain intact (no data deleted)
    let raw_conn = rusqlite::Connection::open(app_paths.app_db_path()).unwrap();
    let count: i64 = raw_conn
        .query_row("SELECT COUNT(*) FROM recent_workspaces", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count, 2,
        "Database rows must not be purged; only filtered on read"
    );
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

#[test]
fn restore_under_and_changed_under_touch_only_the_given_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = Workspace::create(tmp.path(), "WS").unwrap();
    let a = ws.root().join("projects/p/artifacts/a_1");
    let b = ws.root().join("projects/p/artifacts/aX1"); // `_` must not act as a wildcard
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(a.join("index.html"), "A v1").unwrap();
    std::fs::write(b.join("index.html"), "B v1").unwrap();
    let cp = CheckpointService::create(&ws, "trước", None).unwrap();
    let count_before = CheckpointService::list(&ws).unwrap().len();

    std::fs::write(a.join("index.html"), "A v2").unwrap();
    std::fs::write(a.join("extra.css"), "new").unwrap();
    std::fs::write(b.join("index.html"), "B v2").unwrap();

    let prefixes = vec![
        "projects/p/artifacts/a_1/".to_string(),
        "projects/p/artifacts/none/".to_string(),
    ];
    let changed = CheckpointService::changed_under(&ws, &cp, &prefixes).unwrap();
    assert_eq!(changed, vec!["projects/p/artifacts/a_1/".to_string()]);
    assert_eq!(
        CheckpointService::list(&ws).unwrap().len(),
        count_before,
        "no checkpoint created"
    );

    CheckpointService::restore_under(&ws, &cp, "projects/p/artifacts/a_1/").unwrap();
    assert_eq!(
        std::fs::read_to_string(a.join("index.html")).unwrap(),
        "A v1"
    );
    assert!(!a.join("extra.css").exists());
    assert_eq!(
        std::fs::read_to_string(b.join("index.html")).unwrap(),
        "B v2"
    );
    assert!(CheckpointService::changed_under(&ws, &cp, &prefixes)
        .unwrap()
        .is_empty());
}

// ===== G3 Series / Reference tests =====

#[test]
fn test_series_create_list_get() {
    use studio_workspace::{SeriesRecipe, SeriesService};
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path(), "Test WS").unwrap();

    // Initially empty
    let list = SeriesService::list(&ws).unwrap();
    assert!(list.is_empty());

    // Create a series
    let recipe = SeriesRecipe {
        kit: Some("youtube-pipeline".into()),
        modules: vec!["studio.brand".into()],
        playbook: Some("youtube-video".into()),
        extra: Default::default(),
    };
    let meta = SeriesService::create(
        &ws,
        "lich-su-vn",
        "Lịch Sử Việt Nam",
        "Series khám phá lịch sử VN",
        recipe,
        vec!["nv-an".into()],
    )
    .unwrap();

    assert_eq!(meta.slug, "lich-su-vn");
    assert_eq!(meta.title, "Lịch Sử Việt Nam");
    assert_eq!(meta.references, vec!["nv-an".to_string()]);
    assert!(meta.recipe.kit.as_deref() == Some("youtube-pipeline"));

    // List now has 1
    let list = SeriesService::list(&ws).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].slug, "lich-su-vn");

    // Get by slug
    let fetched = SeriesService::get(&ws, "lich-su-vn").unwrap();
    assert_eq!(fetched.description, "Series khám phá lịch sử VN");

    // series.yaml round-trip: save updated metadata
    let mut updated = fetched.clone();
    updated.description = "Updated description".into();
    SeriesService::save(&ws, &updated).unwrap();
    let reloaded = SeriesService::get(&ws, "lich-su-vn").unwrap();
    assert_eq!(reloaded.description, "Updated description");
    assert_eq!(reloaded.references, vec!["nv-an".to_string()]); // preserved

    // Duplicate slug is rejected
    let err = SeriesService::create(
        &ws,
        "lich-su-vn",
        "Dup",
        "",
        SeriesRecipe::default(),
        vec![],
    )
    .unwrap_err();
    assert!(
        format!("{}", err).contains("tồn tại"),
        "error should mention 'tồn tại': {}",
        err
    );
}

#[test]
fn test_series_slug_validation() {
    use studio_workspace::{SeriesRecipe, SeriesService};
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path(), "Test WS").unwrap();

    // Invalid slugs
    for bad in &["", "a/b", "a\\b", "a..b", "a:b"] {
        let err = SeriesService::create(&ws, bad, "Title", "", SeriesRecipe::default(), vec![])
            .unwrap_err();
        assert!(
            format!("{}", err).contains("hợp lệ")
                || format!("{}", err).to_lowercase().contains("invalid"),
            "expected validation error for slug '{}', got: {}",
            bad,
            err
        );
    }
}

#[test]
fn test_series_create_episode() {
    use studio_workspace::{SeriesRecipe, SeriesService};
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path(), "Test WS").unwrap();

    let recipe = SeriesRecipe {
        kit: Some("youtube-pipeline".into()),
        modules: vec!["studio.brand".into()],
        playbook: Some("youtube-video".into()),
        extra: Default::default(),
    };
    SeriesService::create(
        &ws,
        "my-series",
        "My Series",
        "A series",
        recipe,
        vec!["nv-an".into()],
    )
    .unwrap();

    // Create an episode
    let episode =
        SeriesService::create_episode(&ws, "my-series", "tap-01", "Tập 1: Mở đầu").unwrap();
    assert_eq!(episode.slug(), "tap-01");

    // project.yaml should have series set
    let meta = episode.meta().unwrap();
    assert_eq!(meta.series.as_deref(), Some("my-series"));
    assert_eq!(meta.kit.as_deref(), Some("youtube-pipeline"));
    assert_eq!(meta.playbook.as_deref(), Some("youtube-video"));

    // starter artifact artifact.yaml should have reference edge to "nv-an"
    let art_yaml = episode
        .artifacts_dir()
        .join("script-01")
        .join("artifact.yaml");
    assert!(art_yaml.is_file(), "starter artifact.yaml must exist");
    let content = fs::read_to_string(&art_yaml).unwrap();
    assert!(
        content.contains("nv-an"),
        "starter must have reference edge to nv-an"
    );
    assert!(
        content.contains("reference"),
        "starter must declare reference role"
    );
    // The title has ':' — the file must still be valid YAML with the title intact.
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).expect("valid YAML");
    assert_eq!(parsed["title"].as_str(), Some("Kịch bản - Tập 1: Mở đầu"));
    assert_eq!(parsed["inputs"][0]["role"].as_str(), Some("reference"));
}

#[test]
fn test_project_meta_series_yaml_round_trip() {
    use studio_workspace::SeriesService;
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path(), "Test WS").unwrap();

    SeriesService::create(
        &ws,
        "my-series",
        "My Series",
        "",
        Default::default(),
        vec![],
    )
    .unwrap();

    let proj = Project::create(&ws, "my-project", "My Project").unwrap();
    // Initially no series
    let meta = proj.meta().unwrap();
    assert!(meta.series.is_none());

    // Write series directly to project.yaml
    let yaml_path = proj.root().join("project.yaml");
    let mut content = fs::read_to_string(&yaml_path).unwrap();
    content.push_str("series: my-series\n");
    fs::write(&yaml_path, &content).unwrap();

    // Reload
    let meta2 = proj.meta().unwrap();
    assert_eq!(meta2.series.as_deref(), Some("my-series"));

    // Round trip: serialize and reparse
    let yaml2 = serde_yaml::to_string(&meta2).unwrap();
    let meta3: studio_workspace::ProjectMeta = serde_yaml::from_str(&yaml2).unwrap();
    assert_eq!(meta3.series.as_deref(), Some("my-series"));
}

#[test]
fn test_checkpoint_changed_since() {
    let temp = tempdir().unwrap();
    let ws_dir = temp.path().join("ws");
    fs::create_dir_all(ws_dir.join("docs")).unwrap();

    let ws = Workspace::create(&ws_dir, "Test Workspace").unwrap();

    // 1. Create initial files
    let file_old = ws_dir.join("docs/old.txt");
    let file_mod = ws_dir.join("docs/mod.txt");
    let file_del = ws_dir.join("docs/del.txt");
    let file_same = ws_dir.join("docs/same.txt");

    fs::write(&file_old, "Old Content").unwrap();
    fs::write(&file_mod, "Initial Mod Content").unwrap();
    fs::write(&file_del, "To Be Deleted").unwrap();
    fs::write(&file_same, "Same Content").unwrap();

    // Create Checkpoint
    let cp =
        CheckpointService::create(&ws, "Pre-turn Checkpoint", Some("turn-1".to_string())).unwrap();

    // Sleep briefly to ensure separation between file creation and `since`
    std::thread::sleep(std::time::Duration::from_millis(60));
    let since = std::time::SystemTime::now();
    std::thread::sleep(std::time::Duration::from_millis(60));

    // 2. Perform changes:
    // - Add new file
    let file_new = ws_dir.join("docs/new.txt");
    fs::write(&file_new, "New File Content").unwrap();

    // - Modify mod.txt
    fs::write(&file_mod, "Updated Mod Content").unwrap();

    // - Delete del.txt
    fs::remove_file(&file_del).unwrap();

    // - Touch same.txt (write same content) -> mtime >= since, but hash identical!
    fs::write(&file_same, "Same Content").unwrap();

    // - old.txt is not touched at all -> mtime < since (skipped from hashing)

    // 3. Call changed_since
    let diff = CheckpointService::changed_since(&ws, &cp, since).unwrap();

    assert_eq!(diff.added, vec!["docs/new.txt"]);
    assert_eq!(diff.modified, vec!["docs/mod.txt"]);
    assert_eq!(diff.deleted, vec!["docs/del.txt"]);

    // Check non-existent checkpoint returns error
    let err = CheckpointService::changed_since(&ws, "cp-nonexistent", since);
    assert!(err.is_err());
}
