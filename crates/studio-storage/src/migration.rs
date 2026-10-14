use rusqlite::Connection;

pub struct MigrationEngine;

impl MigrationEngine {
    pub fn apply(conn: &mut Connection) -> rusqlite::Result<()> {
        // Enforce WAL mode and foreign keys
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

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
            Self::apply_v1(conn)?;
        }
        if current < 2 {
            Self::apply_v2(conn)?;
        }

        Ok(())
    }

    fn apply_v1(conn: &mut Connection) -> rusqlite::Result<()> {
        let tx = conn.transaction()?;

        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                title TEXT NOT NULL,
                project_type TEXT NOT NULL,
                status TEXT NOT NULL,
                manifest_version TEXT NOT NULL,
                brand_ref TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_projects_ws ON projects(workspace_id);

            CREATE TABLE IF NOT EXISTS project_revisions (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                parent_id TEXT,
                source_hash TEXT NOT NULL,
                author_actor_id TEXT NOT NULL,
                document_id TEXT NOT NULL,
                summary TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_rev_proj ON project_revisions(project_id);

            CREATE TABLE IF NOT EXISTS creative_documents (
                id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                revision_id TEXT PRIMARY KEY,
                document_json TEXT NOT NULL,
                source_hash TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_docs_id ON creative_documents(id);

            CREATE TABLE IF NOT EXISTS assets (
                id TEXT PRIMARY KEY,
                scope TEXT NOT NULL,
                owner_workspace_id TEXT,
                kind TEXT NOT NULL,
                original_name TEXT NOT NULL,
                declared_mime TEXT NOT NULL,
                verified_mime TEXT NOT NULL,
                blob_sha256 TEXT NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                provenance TEXT NOT NULL,
                rights_state TEXT NOT NULL,
                rights_attestation TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_assets_ws ON assets(owner_workspace_id);
            CREATE INDEX IF NOT EXISTS idx_assets_blob ON assets(blob_sha256);

            CREATE TABLE IF NOT EXISTS brand_kits (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                name TEXT NOT NULL,
                active_version_id TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS brand_versions (
                id TEXT PRIMARY KEY,
                brand_kit_id TEXT NOT NULL,
                version_number INTEGER NOT NULL,
                state TEXT NOT NULL,
                visual_tokens TEXT NOT NULL,
                voice_guidelines TEXT NOT NULL,
                logos TEXT NOT NULL,
                provenance TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (brand_kit_id) REFERENCES brand_kits(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS research_sources (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                canonical_url TEXT NOT NULL,
                domain TEXT NOT NULL,
                fetch_status TEXT NOT NULL,
                snapshot_json TEXT,
                fetched_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_res_ws ON research_sources(workspace_id);

            CREATE TABLE IF NOT EXISTS render_jobs (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                revision_id TEXT NOT NULL,
                state TEXT NOT NULL,
                progress_percent INTEGER NOT NULL,
                input_snapshot_hash TEXT NOT NULL,
                output_asset_ref TEXT,
                output_hash TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_jobs_ws ON render_jobs(workspace_id);

            CREATE TABLE IF NOT EXISTS delivery_packages (
                id TEXT PRIMARY KEY,
                studio_workspace_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                source_revision_id TEXT NOT NULL,
                source_hash TEXT NOT NULL,
                variant_json TEXT NOT NULL,
                media_json TEXT NOT NULL,
                target_selectors_json TEXT NOT NULL,
                idempotency_key TEXT UNIQUE NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit_logs (
                id TEXT PRIMARY KEY,
                actor_id TEXT NOT NULL,
                actor_role TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                operation TEXT NOT NULL,
                side_effect_class TEXT NOT NULL,
                aggregate_id TEXT,
                details TEXT,
                timestamp TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_audit_ws ON audit_logs(workspace_id);

            CREATE TABLE IF NOT EXISTS outbox (
                id TEXT PRIMARY KEY,
                destination TEXT NOT NULL,
                payload_hash TEXT NOT NULL,
                idempotency_key TEXT NOT NULL,
                state TEXT NOT NULL,
                remote_ref TEXT,
                created_at TEXT NOT NULL
            );
            ",
        )?;

        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO schema_migrations (version, checksum, applied_at) VALUES (1, 'v1-initial-schema', ?1)",
            [now],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn apply_v2(conn: &mut Connection) -> rusqlite::Result<()> {
        let tx = conn.transaction()?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_outbox_dest_idem ON outbox(destination, idempotency_key);",
            [],
        )?;
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO schema_migrations (version, checksum, applied_at) VALUES (2, 'v2-outbox-unique-index', ?1)",
            [now],
        )?;
        tx.commit()?;
        Ok(())
    }
}
