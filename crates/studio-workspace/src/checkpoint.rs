use crate::db::WorkspaceDb;
use crate::error::{Result, WorkspaceError};
use crate::workspace::Workspace;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Checkpoint identifier (e.g. `cp-20261016-123456-abc12345`).
pub type CheckpointId = String;

/// Checkpoint record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRecord {
    /// Unique checkpoint ID
    pub id: CheckpointId,
    /// Label or message
    pub label: String,
    /// Associated turn ID if created as part of agent turn
    pub turn_id: Option<String>,
    /// Whether the checkpoint is pinned (preserved during GC)
    pub pinned: bool,
    /// Created timestamp in RFC3339 format
    pub created_at: String,
}

/// Diff between two checkpoints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointDiff {
    /// Paths added in checkpoint `b` relative to `a`
    pub added: Vec<String>,
    /// Paths modified in `b` compared to `a`
    pub modified: Vec<String>,
    /// Paths deleted in `b` that existed in `a`
    pub deleted: Vec<String>,
}

/// Checkpoint service managing snapshots, restore, diff, and GC.
pub struct CheckpointService;

impl CheckpointService {
    /// Create a new checkpoint:
    /// Snapshots source files into content-addressed blobs (`.studio/blobs/ab/cdef...`), atomic write.
    /// Skips `renders/`, `.studio/`, and for files in `library/` larger than 50 MB records hash only without storing blob.
    pub fn create(ws: &Workspace, label: &str, turn_id: Option<String>) -> Result<CheckpointId> {
        let mut db = WorkspaceDb::open(ws)?;
        let blobs_dir = ws.blobs_dir();
        fs::create_dir_all(&blobs_dir)?;

        let now = Utc::now();
        let timestamp_str = now.format("%Y%m%d-%H%M%S").to_string();
        let short_uuid = &uuid::Uuid::new_v4().to_string()[..8];
        let checkpoint_id = format!("cp-{}-{}", timestamp_str, short_uuid);

        // Collect all source files
        let files = Self::scan_source_files(ws)?;

        // Write blobs and prepare database records
        let tx = db.conn_mut().transaction()?;

        tx.execute(
            "INSERT INTO checkpoints (id, label, turn_id, pinned, created_at) VALUES (?1, ?2, ?3, 0, ?4)",
            params![checkpoint_id, label, turn_id, now.to_rfc3339()],
        )?;

        for (rel_path, abs_path) in files {
            let metadata = fs::metadata(&abs_path)?;
            let size_bytes = metadata.len();

            // Hash the file
            let (sha256, bytes) = hash_and_read_file(&abs_path)?;

            // Check if file is in library/ and > 50 MB
            let is_large_library =
                rel_path.starts_with("library/") && size_bytes > 50 * 1024 * 1024;

            if !is_large_library {
                // Store content in CAS blob: .studio/blobs/ab/cdef...
                store_blob(&blobs_dir, &sha256, &bytes)?;
            }

            tx.execute(
                "INSERT INTO checkpoint_files (checkpoint_id, rel_path, blob_sha256, size_bytes)
                 VALUES (?1, ?2, ?3, ?4)",
                params![checkpoint_id, rel_path, sha256, size_bytes],
            )?;
        }

        tx.commit()?;
        Ok(checkpoint_id)
    }

    /// List all checkpoints sorted by creation time descending.
    pub fn list(ws: &Workspace) -> Result<Vec<CheckpointRecord>> {
        let db = WorkspaceDb::open(ws)?;
        let mut stmt = db.conn().prepare(
            "SELECT id, label, turn_id, pinned, created_at FROM checkpoints ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let pinned_int: i32 = row.get(3)?;
            Ok(CheckpointRecord {
                id: row.get(0)?,
                label: row.get(1)?,
                turn_id: row.get(2)?,
                pinned: pinned_int != 0,
                created_at: row.get(4)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Restore workspace to checkpoint state.
    /// Restores content, deletes files created after the checkpoint that are not in it — except skipped dirs.
    pub fn restore(ws: &Workspace, id: &str) -> Result<()> {
        let db = WorkspaceDb::open(ws)?;
        let blobs_dir = ws.blobs_dir();

        // Check checkpoint exists
        let exists: bool = db
            .conn()
            .query_row(
                "SELECT 1 FROM checkpoints WHERE id = ?1",
                params![id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !exists {
            return Err(WorkspaceError::CheckpointNotFound(id.to_string()));
        }

        // Fetch files recorded in this checkpoint
        let mut stmt = db.conn().prepare(
            "SELECT rel_path, blob_sha256, size_bytes FROM checkpoint_files WHERE checkpoint_id = ?1",
        )?;
        let file_rows = stmt.query_map(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u64>(2)?,
            ))
        })?;

        let mut expected_files: HashMap<String, (String, u64)> = HashMap::new();
        for r in file_rows {
            let (rel, sha, size) = r?;
            expected_files.insert(rel, (sha, size));
        }

        // Current source files
        let current_files = Self::scan_source_files(ws)?;

        // 1. Delete files created after the checkpoint that are not in it (except skipped dirs)
        for (cur_rel, cur_abs) in &current_files {
            if !expected_files.contains_key(cur_rel) {
                let _ = fs::remove_file(cur_abs);
            }
        }

        // 2. Restore all expected files from blobs
        for (rel, (sha, size)) in expected_files {
            let is_large_library = rel.starts_with("library/") && size > 50 * 1024 * 1024;
            let target_path = ws.root().join(&rel);

            if is_large_library {
                // If large library file, we recorded hash only; do not overwrite if exists
                continue;
            }

            // If file already exists and has the exact same content, do not touch it
            if target_path.is_file() {
                if let Ok((cur_sha, _)) = hash_and_read_file(&target_path) {
                    if cur_sha == sha {
                        continue;
                    }
                }
            }

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let blob_data = read_blob(&blobs_dir, &sha)?;
            // Atomic write: write to temp and rename
            let temp_path = target_path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
            {
                let mut temp_file = fs::File::create(&temp_path)?;
                temp_file.write_all(&blob_data)?;
                temp_file.sync_all()?;
            }
            fs::rename(&temp_path, &target_path)?;
        }

        Ok(())
    }

    /// Undo a turn at the workspace level:
    /// - Checks turn status: if already 'undone', returns WorkspaceError::TurnAlreadyUndone
    /// - Restores the checkpoint created before the turn
    /// - Deletes events emitted during the turn
    /// - Marks turn status as 'undone'
    /// - Sets thread note if turn was part of a thread
    pub fn undo_turn(ws: &Workspace, turn_id: &str) -> Result<()> {
        let db = WorkspaceDb::open(ws)?;

        // Check if turn is already undone
        let turn_info: Option<(String, Option<String>)> = db
            .conn()
            .query_row(
                "SELECT status, thread_id FROM turns WHERE id = ?1",
                params![turn_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;

        if let Some((status, _)) = &turn_info {
            if status == "undone" {
                return Err(WorkspaceError::TurnAlreadyUndone);
            }
        }

        let checkpoint = Self::list(ws)?
            .into_iter()
            .find(|c| c.turn_id.as_deref() == Some(turn_id))
            .ok_or_else(|| {
                WorkspaceError::CheckpointNotFound(format!(
                    "Không tìm thấy checkpoint của lượt '{turn_id}'."
                ))
            })?;

        Self::restore(ws, &checkpoint.id)?;

        db.conn()
            .execute("DELETE FROM events WHERE turn_id = ?1", params![turn_id])?;
        db.conn().execute(
            "UPDATE turns SET status = 'undone' WHERE id = ?1",
            params![turn_id],
        )?;

        if let Some((_, Some(thread_id))) = turn_info {
            let note_key = format!("thread.note.{thread_id}");
            let _ = db.conn().execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![
                    note_key,
                    "Lưu ý: người dùng đã hoàn tác lượt trước; các thay đổi tệp của lượt đó không còn nữa."
                ],
            );
        }

        Ok(())
    }

    /// Compare two checkpoints and return added, modified, deleted paths.
    pub fn diff(ws: &Workspace, a: &str, b: &str) -> Result<CheckpointDiff> {
        let db = WorkspaceDb::open(ws)?;

        let mut stmt = db.conn().prepare(
            "SELECT rel_path, blob_sha256 FROM checkpoint_files WHERE checkpoint_id = ?1",
        )?;

        let mut get_files = |cp_id: &str| -> Result<HashMap<String, String>> {
            let rows = stmt.query_map(params![cp_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut map = HashMap::new();
            for r in rows {
                let (p, s) = r?;
                map.insert(p, s);
            }
            Ok(map)
        };

        let files_a = get_files(a)?;
        let files_b = get_files(b)?;

        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        for (path_b, sha_b) in &files_b {
            match files_a.get(path_b) {
                None => added.push(path_b.clone()),
                Some(sha_a) => {
                    if sha_a != sha_b {
                        modified.push(path_b.clone());
                    }
                }
            }
        }

        for path_a in files_a.keys() {
            if !files_b.contains_key(path_a) {
                deleted.push(path_a.clone());
            }
        }

        added.sort();
        modified.sort();
        deleted.sort();

        Ok(CheckpointDiff {
            added,
            modified,
            deleted,
        })
    }

    /// Pin or unpin a checkpoint.
    pub fn pin(ws: &Workspace, id: &str, pinned: bool) -> Result<()> {
        let db = WorkspaceDb::open(ws)?;
        let count = db.conn().execute(
            "UPDATE checkpoints SET pinned = ?1 WHERE id = ?2",
            params![if pinned { 1 } else { 0 }, id],
        )?;
        if count == 0 {
            return Err(WorkspaceError::CheckpointNotFound(id.to_string()));
        }
        Ok(())
    }

    /// Garbage-collect checkpoints according to ADR-009 §5:
    /// - Keep all checkpoints < 7 days old
    /// - Daily (1 per day) for 7..30 days
    /// - Weekly (1 per calendar week) after 30 days
    /// - Pinned checkpoints are ALWAYS kept
    ///
    /// Also deletes unreferenced blobs.
    pub fn gc(ws: &Workspace, now: DateTime<Utc>) -> Result<usize> {
        let mut db = WorkspaceDb::open(ws)?;
        let all_cps = Self::list(ws)?;

        let mut kept_ids = HashSet::new();
        // Maps: date key ("YYYY-MM-DD") -> first CP id
        let mut daily_buckets: HashMap<String, String> = HashMap::new();
        // Maps: week key ("YYYY-Www") -> first CP id
        let mut weekly_buckets: HashMap<String, String> = HashMap::new();

        for cp in &all_cps {
            if cp.pinned {
                kept_ids.insert(cp.id.clone());
                continue;
            }

            let created_at = DateTime::parse_from_rfc3339(&cp.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(now);

            let age_days = (now - created_at).num_seconds() as f64 / 86400.0;
            // println!("DEBUG: id={}, age_days={}, created_at={}", cp.id, age_days, cp.created_at);

            if age_days < 7.0 {
                // Keep all < 7 days
                kept_ids.insert(cp.id.clone());
            } else if age_days < 30.0 {
                // Daily for 7..30 days: keep 1 per day (keep newest or first encountered)
                let day_key = created_at.format("%Y-%m-%d").to_string();
                if let std::collections::hash_map::Entry::Vacant(e) = daily_buckets.entry(day_key) {
                    e.insert(cp.id.clone());
                    kept_ids.insert(cp.id.clone());
                }
            } else {
                // Weekly after 30 days: keep 1 per ISO week
                let week_key = created_at.format("%G-W%V").to_string();
                if let std::collections::hash_map::Entry::Vacant(e) = weekly_buckets.entry(week_key)
                {
                    e.insert(cp.id.clone());
                    kept_ids.insert(cp.id.clone());
                }
            }
        }

        // Delete checkpoints that are not kept
        let tx = db.conn_mut().transaction()?;
        let mut deleted_count = 0;
        for cp in &all_cps {
            if !kept_ids.contains(&cp.id) {
                tx.execute("DELETE FROM checkpoints WHERE id = ?1", params![cp.id])?;
                deleted_count += 1;
            }
        }

        // Remove unreferenced blobs
        let mut active_shas = HashSet::new();
        {
            let mut stmt = tx.prepare("SELECT DISTINCT blob_sha256 FROM checkpoint_files")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            for r in rows {
                active_shas.insert(r?);
            }
        }
        tx.commit()?;

        // Clean unreferenced blob files from disk
        let blobs_dir = ws.blobs_dir();
        if blobs_dir.exists() {
            for entry in WalkDir::new(&blobs_dir).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_file() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if let Some(parent) = p.parent() {
                        let parent_name = parent.file_name().unwrap_or_default().to_string_lossy();
                        let full_sha = format!("{}{}", parent_name, file_name);
                        if full_sha.len() == 64 && !active_shas.contains(&full_sha) {
                            let _ = fs::remove_file(p);
                        }
                    }
                }
            }
        }

        Ok(deleted_count)
    }

    /// Scan source files in workspace, skipping renders/ and .studio/.
    fn scan_source_files(ws: &Workspace) -> Result<Vec<(String, PathBuf)>> {
        let ws_root = ws.root();
        let mut files = Vec::new();

        for entry in WalkDir::new(ws_root).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }

            let rel = match p.strip_prefix(ws_root) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };

            // Skip renders/ anywhere and .studio/
            if rel.starts_with(".studio/")
                || rel.contains("/renders/")
                || rel.starts_with("renders/")
                || rel.starts_with(".git/")
            {
                continue;
            }

            files.push((rel, p.to_path_buf()));
        }

        files.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(files)
    }
}

/// Store blob in `.studio/blobs/ab/cdef...` using atomic staging write.
fn store_blob(blobs_dir: &Path, sha256: &str, data: &[u8]) -> Result<()> {
    if sha256.len() < 2 {
        return Ok(());
    }

    let prefix = &sha256[..2];
    let rest = &sha256[2..];

    let target_dir = blobs_dir.join(prefix);
    fs::create_dir_all(&target_dir)?;

    let target_path = target_dir.join(rest);
    if target_path.exists() {
        return Ok(());
    }

    let stage_dir = blobs_dir.join(".staging");
    fs::create_dir_all(&stage_dir)?;
    let stage_path = stage_dir.join(format!("stage-{}", uuid::Uuid::new_v4()));

    {
        let mut f = fs::File::create(&stage_path)?;
        f.write_all(data)?;
        f.sync_all()?;
    }

    fs::rename(&stage_path, &target_path)?;
    Ok(())
}

/// Read blob data from `.studio/blobs/ab/cdef...`
fn read_blob(blobs_dir: &Path, sha256: &str) -> Result<Vec<u8>> {
    if sha256.len() < 2 {
        return Err(WorkspaceError::Internal(format!(
            "Invalid sha256: {}",
            sha256
        )));
    }

    let prefix = &sha256[..2];
    let rest = &sha256[2..];
    let target_path = blobs_dir.join(prefix).join(rest);

    if !target_path.exists() {
        return Err(WorkspaceError::Internal(format!(
            "Blob {} not found",
            sha256
        )));
    }

    let mut buf = Vec::new();
    let mut file = fs::File::open(target_path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn hash_and_read_file(path: &Path) -> Result<(String, Vec<u8>)> {
    let mut file = fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let mut hasher = Sha256::new();
    hasher.update(&buf);
    let hash = hex::encode(hasher.finalize());
    Ok((hash, buf))
}
