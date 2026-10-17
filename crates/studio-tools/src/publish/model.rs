//! Publish data model and SQLite database persistence.

use crate::error::{Result, ToolError};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Target platform for publishing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PublishTarget {
    Youtube,
    Facebook,
    Tiktok,
    Instagram,
    Export,
}

impl PublishTarget {
    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Youtube => "youtube",
            Self::Facebook => "facebook",
            Self::Tiktok => "tiktok",
            Self::Instagram => "instagram",
            Self::Export => "export",
        }
    }

    /// User-facing Vietnamese name for target platform.
    pub fn to_vietnamese(&self) -> &'static str {
        match self {
            Self::Youtube => "YouTube",
            Self::Facebook => "Facebook",
            Self::Tiktok => "TikTok",
            Self::Instagram => "Instagram",
            Self::Export => "Xuất bản gói tệp (Export)",
        }
    }
}

impl std::str::FromStr for PublishTarget {
    type Err = ToolError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "youtube" => Ok(Self::Youtube),
            "facebook" => Ok(Self::Facebook),
            "tiktok" => Ok(Self::Tiktok),
            "instagram" => Ok(Self::Instagram),
            "export" => Ok(Self::Export),
            other => Err(ToolError::ExecutionError(format!(
                "Nền tảng đăng không hợp lệ: \"{other}\""
            ))),
        }
    }
}

/// Content privacy setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PublishPrivacy {
    #[default]
    Private,
    Unlisted,
    Public,
}

impl PublishPrivacy {
    /// Convert to lowercase string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Unlisted => "unlisted",
            Self::Public => "public",
        }
    }

    /// User-facing Vietnamese name.
    pub fn to_vietnamese(&self) -> &'static str {
        match self {
            Self::Private => "Riêng tư",
            Self::Unlisted => "Không công khai",
            Self::Public => "Công khai",
        }
    }
}

impl std::str::FromStr for PublishPrivacy {
    type Err = ToolError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "private" => Ok(Self::Private),
            "unlisted" => Ok(Self::Unlisted),
            "public" => Ok(Self::Public),
            other => Err(ToolError::ExecutionError(format!(
                "Chế độ hiển thị không hợp lệ: \"{other}\""
            ))),
        }
    }
}

/// Status of a publish request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishStatus {
    #[default]
    Draft,
    AwaitingConfirmation,
    Publishing,
    Published,
    Failed,
}

impl PublishStatus {
    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::AwaitingConfirmation => "awaiting_confirmation",
            Self::Publishing => "publishing",
            Self::Published => "published",
            Self::Failed => "failed",
        }
    }

    /// User-facing Vietnamese description.
    pub fn to_vietnamese(&self) -> &'static str {
        match self {
            Self::Draft => "Bản nháp",
            Self::AwaitingConfirmation => "Chờ xác nhận",
            Self::Publishing => "Đang đăng...",
            Self::Published => "Đã đăng",
            Self::Failed => "Thất bại",
        }
    }
}

impl std::str::FromStr for PublishStatus {
    type Err = ToolError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "awaiting_confirmation" => Ok(Self::AwaitingConfirmation),
            "publishing" => Ok(Self::Publishing),
            "published" => Ok(Self::Published),
            "failed" => Ok(Self::Failed),
            other => Err(ToolError::ExecutionError(format!(
                "Trạng thái bản đăng không hợp lệ: \"{other}\""
            ))),
        }
    }
}

/// Metadata associated with a publish request.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PublishMetadata {
    /// Post or video title.
    pub title: String,
    /// Detailed description or caption body.
    #[serde(default)]
    pub description: String,
    /// Tags or hashtags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Privacy setting (defaults to private).
    #[serde(default)]
    pub privacy: PublishPrivacy,
    /// Optional scheduled publishing ISO-8601 timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    /// Optional relative or absolute path to a custom thumbnail image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

/// Complete publish request model stored in SQLite `publish_requests`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishRequest {
    /// Unique identifier for this publish request (UUID).
    pub id: String,
    /// Target artifact ID (e.g. `thumb-01` or `video-01`).
    pub artifact: String,
    /// Project slug.
    pub project: String,
    /// Target channel spec ID (e.g. `youtube` or `youtube-shorts`).
    pub channel: String,
    /// Publish target platform.
    pub target: PublishTarget,
    /// List of file paths to publish (e.g. render output, thumbnail).
    pub files: Vec<String>,
    /// Metadata payload.
    pub metadata: PublishMetadata,
    /// Current request status.
    pub status: PublishStatus,
    /// URL or destination string if successfully published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_url: Option<String>,
    /// Error message if publish failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// RFC3339 creation timestamp.
    pub created_at: String,
    /// RFC3339 update timestamp.
    pub updated_at: String,
}

impl PublishRequest {
    /// Atomically move a request into `publishing`.
    ///
    /// Succeeds only from `draft`, `awaiting_confirmation` or `failed` (explicit user retry), so two
    /// confirmations racing each other — or a confirmation of an already published request — can
    /// never start a second upload. Returns `false` when the claim was refused.
    pub fn claim_for_publishing(conn: &Connection, id: &str, now: &str) -> Result<bool> {
        let changed = conn.execute(
            "UPDATE publish_requests SET status = 'publishing', error = NULL, updated_at = ?2
             WHERE id = ?1 AND status IN ('draft', 'awaiting_confirmation', 'failed')",
            params![id, now],
        )?;
        Ok(changed == 1)
    }

    /// Save or update publish request in the database.
    pub fn save(&self, conn: &Connection) -> Result<()> {
        let files_json =
            serde_json::to_string(&self.files).map_err(|e| ToolError::ParseError(e.to_string()))?;
        let metadata_json = serde_json::to_string(&self.metadata)
            .map_err(|e| ToolError::ParseError(e.to_string()))?;

        conn.execute(
            "INSERT INTO publish_requests (
                id, artifact, project, channel, target, files_json, metadata_json,
                status, result_url, error, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(id) DO UPDATE SET
                artifact = excluded.artifact,
                project = excluded.project,
                channel = excluded.channel,
                target = excluded.target,
                files_json = excluded.files_json,
                metadata_json = excluded.metadata_json,
                status = excluded.status,
                result_url = excluded.result_url,
                error = excluded.error,
                updated_at = excluded.updated_at;",
            params![
                self.id,
                self.artifact,
                self.project,
                self.channel,
                self.target.as_str(),
                files_json,
                metadata_json,
                self.status.as_str(),
                self.result_url,
                self.error,
                self.created_at,
                self.updated_at,
            ],
        )?;

        Ok(())
    }

    /// Retrieve publish request by ID from database.
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Self>> {
        let mut stmt = conn.prepare(
            "SELECT id, artifact, project, channel, target, files_json, metadata_json,
                        status, result_url, error, created_at, updated_at
                 FROM publish_requests WHERE id = ?1",
        )?;

        let res = stmt
            .query_row(params![id], |row| {
                let target_str: String = row.get(4)?;
                let files_json: String = row.get(5)?;
                let meta_json: String = row.get(6)?;
                let status_str: String = row.get(7)?;

                let target = target_str
                    .parse::<PublishTarget>()
                    .unwrap_or(PublishTarget::Export);
                let files: Vec<String> = serde_json::from_str(&files_json).unwrap_or_default();
                let metadata: PublishMetadata =
                    serde_json::from_str(&meta_json).unwrap_or_default();
                let status = status_str
                    .parse::<PublishStatus>()
                    .unwrap_or(PublishStatus::Draft);

                Ok(Self {
                    id: row.get(0)?,
                    artifact: row.get(1)?,
                    project: row.get(2)?,
                    channel: row.get(3)?,
                    target,
                    files,
                    metadata,
                    status,
                    result_url: row.get(8)?,
                    error: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .optional()?;

        Ok(res)
    }

    /// List publish requests, optionally filtered by project slug.
    pub fn list(conn: &Connection, project: Option<&str>) -> Result<Vec<Self>> {
        let (sql, params_vec): (&str, Vec<String>) = match project {
            Some(p) => (
                "SELECT id, artifact, project, channel, target, files_json, metadata_json,
                        status, result_url, error, created_at, updated_at
                 FROM publish_requests WHERE project = ?1 ORDER BY created_at DESC",
                vec![p.to_string()],
            ),
            None => (
                "SELECT id, artifact, project, channel, target, files_json, metadata_json,
                        status, result_url, error, created_at, updated_at
                 FROM publish_requests ORDER BY created_at DESC",
                vec![],
            ),
        };

        let mut stmt = conn.prepare(sql)?;

        let rows = if params_vec.is_empty() {
            stmt.query_map([], Self::map_row)
        } else {
            stmt.query_map(params![params_vec[0]], Self::map_row)
        }?;

        let mut list = Vec::new();
        for item in rows.flatten() {
            list.push(item);
        }
        Ok(list)
    }

    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let target_str: String = row.get(4)?;
        let files_json: String = row.get(5)?;
        let meta_json: String = row.get(6)?;
        let status_str: String = row.get(7)?;

        let target = target_str
            .parse::<PublishTarget>()
            .unwrap_or(PublishTarget::Export);
        let files: Vec<String> = serde_json::from_str(&files_json).unwrap_or_default();
        let metadata: PublishMetadata = serde_json::from_str(&meta_json).unwrap_or_default();
        let status = status_str
            .parse::<PublishStatus>()
            .unwrap_or(PublishStatus::Draft);

        Ok(Self {
            id: row.get(0)?,
            artifact: row.get(1)?,
            project: row.get(2)?,
            channel: row.get(3)?,
            target,
            files,
            metadata,
            status,
            result_url: row.get(8)?,
            error: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE publish_requests (
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
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_publish_request_crud_and_status() {
        let conn = setup_test_db();
        let now = chrono::Utc::now().to_rfc3339();

        let req = PublishRequest {
            id: "req-1".into(),
            artifact: "thumb-01".into(),
            project: "proj-alpha".into(),
            channel: "youtube-shorts".into(),
            target: PublishTarget::Youtube,
            files: vec!["video.mp4".into()],
            metadata: PublishMetadata {
                title: "Tiêu đề test".into(),
                description: "Mô tả test".into(),
                tags: vec!["studio".into(), "test".into()],
                privacy: PublishPrivacy::Private,
                schedule: None,
                thumbnail: None,
            },
            status: PublishStatus::AwaitingConfirmation,
            result_url: None,
            error: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        // 1. Save
        req.save(&conn).unwrap();

        // 2. Get
        let loaded = PublishRequest::get(&conn, "req-1").unwrap().expect("found");
        assert_eq!(loaded.id, "req-1");
        assert_eq!(loaded.target, PublishTarget::Youtube);
        assert_eq!(loaded.status, PublishStatus::AwaitingConfirmation);
        assert_eq!(loaded.metadata.title, "Tiêu đề test");

        // 3. Update status to published
        let mut updated = loaded;
        updated.status = PublishStatus::Published;
        updated.result_url = Some("https://www.youtube.com/watch?v=abc123xyz".into());
        updated.save(&conn).unwrap();

        let loaded_again = PublishRequest::get(&conn, "req-1").unwrap().unwrap();
        assert_eq!(loaded_again.status, PublishStatus::Published);
        assert_eq!(
            loaded_again.result_url.as_deref(),
            Some("https://www.youtube.com/watch?v=abc123xyz")
        );

        // 4. List
        let list = PublishRequest::list(&conn, Some("proj-alpha")).unwrap();
        assert_eq!(list.len(), 1);
        let list_other = PublishRequest::list(&conn, Some("proj-beta")).unwrap();
        assert_eq!(list_other.len(), 0);
    }
}
