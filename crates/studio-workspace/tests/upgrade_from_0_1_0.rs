use rusqlite::Connection;
use std::fs;
use std::path::Path;
use studio_workspace::{Workspace, WorkspaceDb};
use tempfile::tempdir;

const ARTIFACT_YAML_CONTENT: &str = r#"version: 1
id: "art-legacy-01"
project: "demo-legacy"
kind: "design"
title: "Banner Khai Truong 0.1.0"
created_at: "2026-10-14T10:05:00Z"
updated_at: "2026-10-14T10:05:00Z"
"#;

const PROJECT_YAML_CONTENT: &str = r#"slug: "demo-legacy"
title: "Demo 0.1.0 Project"
created_at: "2026-10-14T10:00:00Z"
modules: []
"#;

const RENDER_PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\n\x00fake_0_1_0_render_bytes_payload";

/// Builds a workspace directory matching the exact state and SQLite schema left by Local Content Studio 0.1.0.
fn build_0_1_0_workspace(root: &Path) {
    // 1. Filesystem layout: studio.yaml, projects, artifacts, renders, blobs
    fs::create_dir_all(root.join(".studio/blobs")).unwrap();
    fs::create_dir_all(root.join(".studio/cache")).unwrap();
    fs::create_dir_all(root.join(".studio/backups")).unwrap();

    fs::write(
        root.join("studio.yaml"),
        "name: \"Legacy 0.1.0 Workspace\"\nversion: 1\ndefault_brand: null\n",
    )
    .unwrap();

    let proj_dir = root.join("projects/demo-legacy");
    fs::create_dir_all(proj_dir.join("artifacts/art-legacy-01")).unwrap();
    fs::create_dir_all(proj_dir.join("renders")).unwrap();

    fs::write(proj_dir.join("project.yaml"), PROJECT_YAML_CONTENT).unwrap();

    let art_yaml_path = proj_dir.join("artifacts/art-legacy-01/artifact.yaml");
    fs::write(&art_yaml_path, ARTIFACT_YAML_CONTENT).unwrap();

    let render_path = proj_dir.join("renders/art-legacy-01-render.png");
    fs::write(&render_path, RENDER_PNG_BYTES).unwrap();

    let blob_path = root.join(".studio/blobs/deadbeef0102030405060708090a0b0c0d0e0f");
    fs::write(&blob_path, b"checkpoint_blob_content_0_1_0").unwrap();

    // 2. Initialize .studio/studio.db with the exact 0.1.0 schema (v1 + v2 migrations from commit fca95ed)
    let db_path = root.join(".studio/studio.db");
    let conn = Connection::open(&db_path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();

    // schema_migrations table
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            checksum TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );
        INSERT INTO schema_migrations (version, checksum, applied_at)
        VALUES (1, 'v1-workspace-schema', '2026-10-14T10:00:00Z'),
               (2, 'v2-publish-requests', '2026-10-14T10:05:00Z');
        ",
    )
    .unwrap();

    // v1 tables
    conn.execute_batch(
        "
        CREATE TABLE threads (
            id TEXT PRIMARY KEY,
            project_slug TEXT,
            title TEXT NOT NULL,
            engine TEXT NOT NULL,
            model TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX idx_threads_proj ON threads(project_slug);

        CREATE TABLE turns (
            id TEXT PRIMARY KEY,
            thread_id TEXT NOT NULL,
            role TEXT NOT NULL,
            input_text TEXT NOT NULL,
            status TEXT NOT NULL,
            error_message TEXT,
            started_at TEXT NOT NULL,
            ended_at TEXT,
            FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE
        );
        CREATE INDEX idx_turns_thread ON turns(thread_id);

        CREATE TABLE events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            thread_id TEXT NOT NULL,
            turn_id TEXT NOT NULL,
            seq INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE,
            FOREIGN KEY (turn_id) REFERENCES turns(id) ON DELETE CASCADE
        );
        CREATE INDEX idx_events_turn ON events(turn_id);

        CREATE TABLE artifacts_index (
            id TEXT PRIMARY KEY,
            project_slug TEXT NOT NULL,
            kind TEXT NOT NULL,
            title TEXT NOT NULL,
            rel_path TEXT NOT NULL,
            status TEXT NOT NULL,
            input_hash TEXT NOT NULL,
            output_hash TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX idx_artifacts_proj ON artifacts_index(project_slug);

        CREATE TABLE assets_index (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL,
            name TEXT NOT NULL,
            mime_type TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            sha256 TEXT NOT NULL,
            metadata_json TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX idx_assets_sha ON assets_index(sha256);

        CREATE TABLE renders (
            id TEXT PRIMARY KEY,
            artifact_id TEXT NOT NULL,
            hash TEXT NOT NULL,
            output_path TEXT NOT NULL,
            format TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE INDEX idx_renders_art ON renders(artifact_id);

        CREATE TABLE jobs (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            status TEXT NOT NULL,
            priority INTEGER NOT NULL DEFAULT 0,
            input_json TEXT NOT NULL,
            output_json TEXT,
            error_message TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE checkpoints (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            turn_id TEXT,
            pinned INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );

        CREATE TABLE checkpoint_files (
            checkpoint_id TEXT NOT NULL,
            rel_path TEXT NOT NULL,
            blob_sha256 TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            PRIMARY KEY (checkpoint_id, rel_path),
            FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id) ON DELETE CASCADE
        );
        CREATE INDEX idx_cpf_sha ON checkpoint_files(blob_sha256);

        CREATE TABLE cost_ledger (
            id TEXT PRIMARY KEY,
            turn_id TEXT,
            job_id TEXT,
            provider TEXT NOT NULL,
            model TEXT,
            cost_usd REAL NOT NULL,
            details_json TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE tools (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            description TEXT,
            manifest_json TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE VIRTUAL TABLE search_fts USING fts5(
            target_type,
            target_id,
            title,
            content
        );

        -- v2 tables
        CREATE TABLE publish_requests (
            id TEXT PRIMARY KEY,
            artifact TEXT NOT NULL,
            project TEXT NOT NULL,
            channel TEXT NOT NULL,
            target TEXT NOT NULL,
            files_json TEXT NOT NULL,
            metadata_json TEXT NOT NULL,
            status TEXT NOT NULL,
            result_url TEXT,
            error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX idx_publish_requests_proj ON publish_requests(project);
        CREATE INDEX idx_publish_requests_status ON publish_requests(status);
        ",
    )
    .unwrap();

    // 3. Populate 0.1.0 data rows
    conn.execute(
        "INSERT INTO threads (id, project_slug, title, engine, model, created_at, updated_at)
         VALUES ('th-010', 'demo-legacy', 'Hội thoại ban đầu 0.1.0', 'claude', 'claude-3-5-sonnet', '2026-10-14T10:01:00Z', '2026-10-14T10:02:00Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO turns (id, thread_id, role, input_text, status, error_message, started_at, ended_at)
         VALUES ('turn-010', 'th-010', 'user', 'Tạo cho tôi một banner khai trương đẹp', 'done', NULL, '2026-10-14T10:01:05Z', '2026-10-14T10:01:30Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO events (id, thread_id, turn_id, seq, event_type, payload_json, timestamp)
         VALUES (1, 'th-010', 'turn-010', 1, 'text', '{\"text\":\"Đã tạo banner xong\"}', '2026-10-14T10:01:10Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO artifacts_index (id, project_slug, kind, title, rel_path, status, input_hash, output_hash, updated_at)
         VALUES ('art-legacy-01', 'demo-legacy', 'design', 'Banner Khai Truong 0.1.0', 'projects/demo-legacy/artifacts/art-legacy-01/artifact.yaml', 'done', 'hash-in-010', 'hash-out-010', '2026-10-14T10:05:00Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO renders (id, artifact_id, hash, output_path, format, size_bytes, created_at)
         VALUES ('rnd-010', 'art-legacy-01', 'hash-out-010', 'projects/demo-legacy/renders/art-legacy-01-render.png', 'png', 42, '2026-10-14T10:05:10Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO checkpoints (id, label, turn_id, pinned, created_at)
         VALUES ('cp-010', 'Checkpoint tự động trước lượt 1', 'turn-010', 1, '2026-10-14T10:01:00Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO checkpoint_files (checkpoint_id, rel_path, blob_sha256, size_bytes)
         VALUES ('cp-010', 'projects/demo-legacy/artifacts/art-legacy-01/artifact.yaml', 'deadbeef0102030405060708090a0b0c0d0e0f', 128)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO assets_index (id, path, name, mime_type, size_bytes, sha256, metadata_json, updated_at)
         VALUES ('asset-010', 'projects/demo-legacy/logo.png', 'logo.png', 'image/png', 1024, 'sha256logo010', '{}', '2026-10-14T10:00:00Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO publish_requests (id, artifact, project, channel, target, files_json, metadata_json, status, result_url, error, created_at, updated_at)
         VALUES ('pub-010', 'art-legacy-01', 'demo-legacy', 'youtube', 'draft', '[]', '{}', 'completed', 'https://youtu.be/legacy010', NULL, '2026-10-14T10:06:00Z', '2026-10-14T10:06:30Z')",
        [],
    )
    .unwrap();

    // Close connection cleanly so WAL checkpoints and release locks
    drop(conn);
}

#[test]
fn test_upgrade_from_0_1_0_workspace() {
    let temp = tempdir().unwrap();
    let ws_dir = temp.path().join("legacy-ws");

    // 1. Build workspace exactly as 0.1.0 left it
    build_0_1_0_workspace(&ws_dir);

    let art_yaml_path = ws_dir.join("projects/demo-legacy/artifacts/art-legacy-01/artifact.yaml");
    let render_path = ws_dir.join("projects/demo-legacy/renders/art-legacy-01-render.png");

    let art_yaml_before = fs::read_to_string(&art_yaml_path).unwrap();
    assert_eq!(art_yaml_before, ARTIFACT_YAML_CONTENT);
    let render_bytes_before = fs::read(&render_path).unwrap();
    assert_eq!(render_bytes_before, RENDER_PNG_BYTES);

    // 2. Open workspace with current code (triggers apply_migrations_with_backup)
    let ws = Workspace::open(&ws_dir).expect("Workspace::open should succeed on 0.1.0 layout");
    let ws_db = WorkspaceDb::open(&ws).expect("WorkspaceDb::open should run migrations cleanly");

    // 3. Assertion: Pre-migration backup file exists in .studio/backups/
    let backups: Vec<_> = fs::read_dir(ws.backups_dir())
        .expect("backups_dir must exist")
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("studio-pre-") && name.ends_with(".db")
        })
        .collect();

    assert!(
        !backups.is_empty(),
        "At least one pre-migration backup file matching studio-pre-*.db must exist"
    );

    // Verify the initial pre-v3 backup preserves the original 0.1.0 state (version 2)
    let pre_v3_backup = backups
        .iter()
        .find(|b| {
            b.file_name()
                .to_string_lossy()
                .starts_with("studio-pre-v3-")
        })
        .expect("studio-pre-v3 backup must exist before applying v3");

    let pre_v3_conn =
        Connection::open(pre_v3_backup.path()).expect("Backup file must be valid SQLite DB");
    let initial_backup_ver: i64 = pre_v3_conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
            r.get(0)
        })
        .expect("Backup must contain schema_migrations table");
    assert_eq!(
        initial_backup_ver, 2,
        "Pre-migration backup before v3 must be preserved at version 2 (0.1.0 state)"
    );

    for b in &backups {
        assert!(
            b.metadata().unwrap().len() > 0,
            "Backup database file must not be empty"
        );
        let backup_conn = Connection::open(b.path()).expect("Backup file must be valid SQLite DB");
        let v: i64 = backup_conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .expect("Backup must contain schema_migrations table");
        assert!(v >= 2, "Backup version must be at least version 2");
    }

    // 4. Assertion: All migrations ran to the latest version generically
    let max_version: i64 = ws_db
        .conn()
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
            r.get(0)
        })
        .expect("Query max version");

    let count_migrations: i64 = ws_db
        .conn()
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
        .expect("Query count migrations");

    assert!(
        max_version > 2,
        "Migrations must have advanced beyond 0.1.0's version (2), got {}",
        max_version
    );
    assert_eq!(
        max_version, count_migrations,
        "All migrations 1..MAX must have been applied without gaps"
    );

    // 5. Assertion: Rows from 0.1.0 remain intact
    let thread_title: String = ws_db
        .conn()
        .query_row("SELECT title FROM threads WHERE id = 'th-010'", [], |r| {
            r.get(0)
        })
        .expect("Thread row intact");
    assert_eq!(thread_title, "Hội thoại ban đầu 0.1.0");

    let turn_input: String = ws_db
        .conn()
        .query_row(
            "SELECT input_text FROM turns WHERE id = 'turn-010'",
            [],
            |r| r.get(0),
        )
        .expect("Turn row intact");
    assert_eq!(turn_input, "Tạo cho tôi một banner khai trương đẹp");

    let event_type: String = ws_db
        .conn()
        .query_row(
            "SELECT event_type FROM events WHERE thread_id = 'th-010' AND seq = 1",
            [],
            |r| r.get(0),
        )
        .expect("Event row intact");
    assert_eq!(event_type, "text");

    let art_title: String = ws_db
        .conn()
        .query_row(
            "SELECT title FROM artifacts_index WHERE id = 'art-legacy-01'",
            [],
            |r| r.get(0),
        )
        .expect("Artifacts index row intact");
    assert_eq!(art_title, "Banner Khai Truong 0.1.0");

    let render_format: String = ws_db
        .conn()
        .query_row("SELECT format FROM renders WHERE id = 'rnd-010'", [], |r| {
            r.get(0)
        })
        .expect("Render row intact");
    assert_eq!(render_format, "png");

    let cp_label: String = ws_db
        .conn()
        .query_row(
            "SELECT label FROM checkpoints WHERE id = 'cp-010'",
            [],
            |r| r.get(0),
        )
        .expect("Checkpoint row intact");
    assert_eq!(cp_label, "Checkpoint tự động trước lượt 1");

    let cp_file_rel: String = ws_db
        .conn()
        .query_row(
            "SELECT rel_path FROM checkpoint_files WHERE checkpoint_id = 'cp-010'",
            [],
            |r| r.get(0),
        )
        .expect("Checkpoint file intact");
    assert_eq!(
        cp_file_rel,
        "projects/demo-legacy/artifacts/art-legacy-01/artifact.yaml"
    );

    let pub_target: String = ws_db
        .conn()
        .query_row(
            "SELECT target FROM publish_requests WHERE id = 'pub-010'",
            [],
            |r| r.get(0),
        )
        .expect("Publish request intact");
    assert_eq!(pub_target, "draft");

    // Assets index row intact with new v4 column defaulted properly
    let (asset_name, asset_source): (String, String) = ws_db
        .conn()
        .query_row(
            "SELECT name, source FROM assets_index WHERE id = 'asset-010'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("Assets index intact with new v4 column");
    assert_eq!(asset_name, "logo.png");
    assert_eq!(asset_source, "uploaded");

    // 6. Assertion: Files on disk (artifact.yaml, render) are unchanged
    let art_yaml_after = fs::read_to_string(&art_yaml_path).unwrap();
    assert_eq!(
        art_yaml_after, art_yaml_before,
        "artifact.yaml must remain unchanged on disk"
    );

    let render_bytes_after = fs::read(&render_path).unwrap();
    assert_eq!(
        render_bytes_after, render_bytes_before,
        "Rendered image file must remain unchanged on disk"
    );

    // 7. Assertion: Re-opening is idempotent (no duplicate backups, schema stays intact)
    let initial_backup_count = backups.len();
    drop(ws_db);

    let ws_db_2 = WorkspaceDb::open(&ws).expect("Second WorkspaceDb::open must succeed");
    let backups_after: Vec<_> = fs::read_dir(ws.backups_dir())
        .expect("backups_dir must exist")
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("studio-pre-") && name.ends_with(".db")
        })
        .collect();

    assert_eq!(
        backups_after.len(),
        initial_backup_count,
        "Reopening database when already on latest version must not create additional backups"
    );

    let max_v2: i64 = ws_db_2
        .conn()
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(max_v2, max_version, "Version must remain identical");
}
