use crate::error::Result;
use crate::workspace::Workspace;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

/// Workspace database manager at `.studio/studio.db`.
pub struct WorkspaceDb {
    conn: Connection,
}

impl WorkspaceDb {
    /// Open database at given path, enforcing WAL, foreign keys, and running migrations.
    pub fn open(ws: &Workspace) -> Result<Self> {
        let db_path = ws.db_path();
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut conn = Connection::open(&db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        Self::run_migrations(&mut conn, ws)?;

        Ok(Self { conn })
    }

    /// Open connection directly from db path and backup directory (e.g. for testing).
    pub fn open_path(db_path: impl AsRef<Path>, backup_dir: impl AsRef<Path>) -> Result<Self> {
        let p = db_path.as_ref();
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut conn = Connection::open(p)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        Self::apply_migrations_with_backup(&mut conn, p, backup_dir.as_ref())?;

        Ok(Self { conn })
    }

    /// Access inner SQLite connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Access inner mutable SQLite connection.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    fn run_migrations(conn: &mut Connection, ws: &Workspace) -> Result<()> {
        Self::apply_migrations_with_backup(conn, &ws.db_path(), &ws.backups_dir())
    }

    /// Run idempotent migrations with pre-migration SQLite backup.
    pub fn apply_migrations_with_backup(
        conn: &mut Connection,
        db_path: &Path,
        backup_dir: &Path,
    ) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                checksum TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );",
            [],
        )?;

        let current: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, Option<i64>>(0)
            })
            .unwrap_or(None)
            .unwrap_or(0);

        if current < 1 {
            // Backup before applying v1 if database file already exists and has tables
            if db_path.exists() {
                let table_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name != 'schema_migrations'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                if table_count > 0 {
                    Self::perform_backup(conn, backup_dir, "v1")?;
                }
            }

            Self::apply_v1(conn)?;
        }

        if current < 2 {
            // Backup before applying v2 if database file already exists and has tables
            if db_path.exists() {
                let table_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name != 'schema_migrations'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                if table_count > 0 {
                    Self::perform_backup(conn, backup_dir, "v2")?;
                }
            }

            Self::apply_v2(conn)?;
        }

        if current < 3 {
            // Backup before applying v3 if database file already exists and has tables
            if db_path.exists() {
                let table_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name != 'schema_migrations'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                if table_count > 0 {
                    Self::perform_backup(conn, backup_dir, "v3")?;
                }
            }

            Self::apply_v3(conn)?;
        }

        Ok(())
    }

    fn perform_backup(conn: &Connection, backup_dir: &Path, target_version: &str) -> Result<()> {
        fs::create_dir_all(backup_dir)?;
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let backup_file =
            backup_dir.join(format!("studio-pre-{}-{}.db", target_version, timestamp));

        let mut dst_conn = Connection::open(&backup_file)?;
        let backup = rusqlite::backup::Backup::new(conn, &mut dst_conn)?;
        backup.run_to_completion(10, std::time::Duration::from_millis(5), None)?;
        Ok(())
    }

    fn apply_v1(conn: &mut Connection) -> Result<()> {
        let tx = conn.transaction()?;

        tx.execute_batch(
            "
            -- 1. Threads
            CREATE TABLE IF NOT EXISTS threads (
                id TEXT PRIMARY KEY,
                project_slug TEXT,
                title TEXT NOT NULL,
                engine TEXT NOT NULL,
                model TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_threads_proj ON threads(project_slug);

            -- 2. Turns
            CREATE TABLE IF NOT EXISTS turns (
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                turn_index INTEGER NOT NULL,
                user_prompt TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_turns_thread ON turns(thread_id);

            -- 3. Events
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                turn_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (turn_id) REFERENCES turns(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_events_turn ON events(turn_id);

            -- 4. Artifacts Index
            CREATE TABLE IF NOT EXISTS artifacts_index (
                id TEXT PRIMARY KEY,
                project_slug TEXT NOT NULL,
                kind TEXT NOT NULL,
                title TEXT NOT NULL,
                path TEXT NOT NULL,
                brand TEXT,
                channels_json TEXT NOT NULL,
                source_file TEXT NOT NULL,
                renderer TEXT,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_artifacts_proj ON artifacts_index(project_slug);

            -- 5. Assets Index
            CREATE TABLE IF NOT EXISTS assets_index (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                name TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                sha256 TEXT NOT NULL,
                metadata_json TEXT,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_assets_sha ON assets_index(sha256);

            -- 6. Renders
            CREATE TABLE IF NOT EXISTS renders (
                id TEXT PRIMARY KEY,
                artifact_id TEXT NOT NULL,
                hash TEXT NOT NULL,
                output_path TEXT NOT NULL,
                format TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_renders_art ON renders(artifact_id);

            -- 7. Jobs
            CREATE TABLE IF NOT EXISTS jobs (
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

            -- 8. Checkpoints
            CREATE TABLE IF NOT EXISTS checkpoints (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                turn_id TEXT,
                pinned INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            -- 9. Checkpoint files
            CREATE TABLE IF NOT EXISTS checkpoint_files (
                checkpoint_id TEXT NOT NULL,
                rel_path TEXT NOT NULL,
                blob_sha256 TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                PRIMARY KEY (checkpoint_id, rel_path),
                FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_cpf_sha ON checkpoint_files(blob_sha256);

            -- 10. Cost Ledger
            CREATE TABLE IF NOT EXISTS cost_ledger (
                id TEXT PRIMARY KEY,
                turn_id TEXT,
                job_id TEXT,
                provider TEXT NOT NULL,
                model TEXT,
                cost_usd REAL NOT NULL,
                details_json TEXT,
                created_at TEXT NOT NULL
            );

            -- 11. Tools
            CREATE TABLE IF NOT EXISTS tools (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                description TEXT,
                manifest_json TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                updated_at TEXT NOT NULL
            );

            -- 12. Settings
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- 13. FTS5 Search Index over artifact and asset title + text
            CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
                target_type, -- 'artifact' or 'asset'
                target_id,
                title,
                content
            );
            ",
        )?;

        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO schema_migrations (version, checksum, applied_at) VALUES (1, 'v1-workspace-schema', ?1)
             ON CONFLICT(version) DO NOTHING",
            [&now],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn apply_v2(conn: &mut Connection) -> Result<()> {
        let tx = conn.transaction()?;

        tx.execute_batch(
            "
            -- 14. Publish Requests
            CREATE TABLE IF NOT EXISTS publish_requests (
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
            CREATE INDEX IF NOT EXISTS idx_publish_requests_proj ON publish_requests(project);
            CREATE INDEX IF NOT EXISTS idx_publish_requests_status ON publish_requests(status);
            ",
        )?;

        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO schema_migrations (version, checksum, applied_at) VALUES (2, 'v2-publish-requests', ?1)
             ON CONFLICT(version) DO NOTHING",
            [&now],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn apply_v3(conn: &mut Connection) -> Result<()> {
        let tx = conn.transaction()?;

        tx.execute_batch(
            "
            -- 15. Artifact Checks Matrix Cache
            CREATE TABLE IF NOT EXISTS artifact_checks (
                artifact_id TEXT NOT NULL,
                project TEXT NOT NULL,
                channel_id TEXT NOT NULL,
                render_hash TEXT NOT NULL,
                channel_version TEXT NOT NULL,
                status TEXT NOT NULL,
                errors_count INTEGER NOT NULL,
                warnings_count INTEGER NOT NULL,
                findings_json TEXT NOT NULL,
                checked_at TEXT NOT NULL,
                PRIMARY KEY (project, artifact_id, channel_id)
            );
            CREATE INDEX IF NOT EXISTS idx_art_checks_lookup
                ON artifact_checks(project, artifact_id, channel_id, render_hash, channel_version);
            ",
        )?;

        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO schema_migrations (version, checksum, applied_at) VALUES (3, 'v3-artifact-checks-matrix', ?1)
             ON CONFLICT(version) DO NOTHING",
            [&now],
        )?;

        tx.commit()?;
        Ok(())
    }
}
