use crate::error::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Application paths resolver.
#[derive(Debug, Clone)]
pub struct AppPaths {
    data_dir: PathBuf,
}

impl AppPaths {
    /// Resolve application data directory.
    /// Checks `STUDIO_APP_DIR` environment variable first,
    /// then falls back to `%APPDATA%/LocalContentStudio` (or equivalent).
    pub fn from_env() -> Self {
        if let Ok(val) = env::var("STUDIO_APP_DIR") {
            return Self {
                data_dir: PathBuf::from(val),
            };
        }

        let base = if let Ok(app_data) = env::var("APPDATA") {
            PathBuf::from(app_data)
        } else if let Ok(home) = env::var("USERPROFILE").or_else(|_| env::var("HOME")) {
            PathBuf::from(home).join(".localcontentstudio")
        } else {
            PathBuf::from(".studio-app-data")
        };

        Self {
            data_dir: base.join("LocalContentStudio"),
        }
    }

    /// Construct `AppPaths` with custom directory (useful for testing).
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
        }
    }

    /// Returns the application data directory.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Returns the path to `app.db`.
    pub fn app_db_path(&self) -> PathBuf {
        self.data_dir.join("app.db")
    }

    /// Returns the built-in/app-level shared directory (`%APPDATA%/LocalContentStudio/shared`).
    pub fn shared_dir(&self) -> PathBuf {
        self.data_dir.join("shared")
    }

    /// Open or initialize `AppDb`.
    pub fn open_app_db(&self) -> Result<AppDb> {
        fs::create_dir_all(&self.data_dir)?;
        AppDb::open(self.app_db_path())
    }
}

/// Recent workspace record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentWorkspace {
    /// Absolute or canonical path to workspace directory
    pub path: String,
    /// Last opened timestamp in RFC3339 format
    pub last_opened: String,
}

/// Root directory record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRoot {
    /// Path to root folder
    pub path: String,
    /// Name or label for the root
    pub name: Option<String>,
    /// Created timestamp in RFC3339 format
    pub created_at: String,
}

/// Org record in AppDb.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppOrgRecord {
    /// Path to org folder
    pub path: String,
    /// Org name
    pub name: String,
    /// Created timestamp in RFC3339 format
    pub created_at: String,
}

/// SQLite database storing app-level state (`app.db`).
pub struct AppDb {
    conn: Connection,
}

impl AppDb {
    /// Open database at given path with WAL mode enabled.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize tables in `app.db`.
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS roots (
                path TEXT PRIMARY KEY,
                name TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS orgs (
                path TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS recent_workspaces (
                path TEXT PRIMARY KEY,
                last_opened TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    /// Add or update a root directory.
    pub fn add_root(&self, path: &str, name: Option<&str>) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO roots (path, name, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET name = excluded.name",
            params![path, name, now],
        )?;
        Ok(())
    }

    /// List all roots.
    pub fn list_roots(&self) -> Result<Vec<AppRoot>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, name, created_at FROM roots ORDER BY created_at ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(AppRoot {
                path: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Register or update an org path.
    pub fn register_org(&self, path: &str, name: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO orgs (path, name, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET name = excluded.name",
            params![path, name, now],
        )?;
        Ok(())
    }

    /// List registered orgs.
    pub fn list_orgs(&self) -> Result<Vec<AppOrgRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, name, created_at FROM orgs ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(AppOrgRecord {
                path: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Record a workspace opening.
    pub fn record_workspace_open(&self, path: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO recent_workspaces (path, last_opened) VALUES (?1, ?2)
             ON CONFLICT(path) DO UPDATE SET last_opened = excluded.last_opened",
            params![path, now],
        )?;
        Ok(())
    }

    /// List recent workspaces, sorted descending by last_opened.
    pub fn list_recent_workspaces(&self) -> Result<Vec<RecentWorkspace>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, last_opened FROM recent_workspaces ORDER BY last_opened DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok(RecentWorkspace {
                path: row.get(0)?,
                last_opened: row.get(1)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Set a key-value setting.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get a key-value setting.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let val: String = row.get(0)?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }
}
