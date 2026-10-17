//! Process-wide state shared by all IPC commands.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use studio_engine::{AgyEngine, ClaudeEngine};
use studio_jobs::{EstimateStore, MemoryEstimateStore, Scheduler, SchedulerConfig, SysinfoProbe};
use studio_mcp::TokenRegistry;
use studio_tools::{CredentialManagerStore, SecretStore};
use studio_workspace::{AppPaths, Workspace};
use tokio::sync::Notify;

use crate::broker::UiBroker;
use crate::error::{CmdError, CmdResult};

/// A turn that is currently running.
pub struct RunningTurn {
    /// Thread the turn belongs to.
    pub thread_id: String,
    /// Signals the turn task to cancel the engine process tree.
    pub cancel: Arc<Notify>,
}

/// Shared application state.
pub struct AppState {
    /// App data paths (`%APPDATA%/LocalContentStudio`).
    pub app_paths: AppPaths,
    /// Machine-wide job scheduler.
    pub scheduler: Arc<Scheduler>,
    /// Per-turn MCP tokens.
    pub tokens: Arc<TokenRegistry>,
    /// Secrets in Windows Credential Manager.
    pub secrets: Arc<dyn SecretStore>,
    /// Permission broker backed by UI dialogs.
    pub broker: Arc<UiBroker>,
    /// Studio MCP port (set once the server is up).
    pub mcp_port: OnceLock<u16>,
    /// Host-set API endpoint overrides by tool id.
    pub endpoints: Arc<tokio::sync::RwLock<HashMap<String, String>>>,
    /// Host-set pricing configuration per provider.
    pub pricing: Arc<tokio::sync::RwLock<HashMap<String, studio_tools::ProviderPricing>>>,
    /// Currently open workspace root.
    pub current: Mutex<Option<PathBuf>>,
    /// Running turns by id.
    pub turns: Mutex<HashMap<String, RunningTurn>>,
    /// Filesystem watcher of the open workspace.
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
    /// App-layer catalog file.
    pub catalog_path: PathBuf,
    /// Cloned catalog sources (`%LOCALAPPDATA%/LocalContentStudio/catalog/sources`).
    pub sources_dir: PathBuf,
    /// Pending tool installation previews waiting for user confirmation.
    pub tool_previews: Mutex<HashMap<String, studio_tools::ToolInstallPreview>>,
    /// Claude Code engine adapter.
    pub claude: ClaudeEngine,
    /// Antigravity engine adapter.
    pub agy: AgyEngine,
}

impl AppState {
    /// Build the state with real system probes and the Windows credential store.
    pub fn new() -> Self {
        let app_paths = AppPaths::from_env();
        let estimates: Arc<dyn EstimateStore> = Arc::new(MemoryEstimateStore::new());
        let local = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| app_paths.data_dir().to_path_buf());
        Self {
            catalog_path: app_paths.data_dir().join("catalog").join("catalog.yaml"),
            sources_dir: local
                .join("LocalContentStudio")
                .join("catalog")
                .join("sources"),
            app_paths: app_paths.clone(),
            scheduler: Scheduler::new(
                SchedulerConfig::default(),
                Arc::new(SysinfoProbe::new()),
                estimates,
            ),
            tokens: Arc::new(TokenRegistry::new()),
            secrets: Arc::new(CredentialManagerStore::new()),
            broker: Arc::new(UiBroker::default()),
            mcp_port: OnceLock::new(),
            endpoints: {
                let mut map = HashMap::new();
                if let Ok(db) = app_paths.open_app_db() {
                    if let Ok(list) = db.list_settings_by_prefix("endpoint:") {
                        for (key, val) in list {
                            if let Some(tool_id) = key.strip_prefix("endpoint:") {
                                map.insert(tool_id.to_string(), val);
                            }
                        }
                    }
                }
                Arc::new(tokio::sync::RwLock::new(map))
            },
            pricing: {
                let mut map = HashMap::new();
                if let Ok(db) = app_paths.open_app_db() {
                    if let Ok(list) = db.list_settings_by_prefix("pricing:") {
                        for (key, val) in list {
                            if let Some(tool_id) = key.strip_prefix("pricing:") {
                                if let Ok(parsed) =
                                    serde_json::from_str::<studio_tools::ProviderPricing>(&val)
                                {
                                    map.insert(tool_id.to_string(), parsed);
                                }
                            }
                        }
                    }
                }
                Arc::new(tokio::sync::RwLock::new(map))
            },
            current: Mutex::new(None),
            turns: Mutex::new(HashMap::new()),
            tool_previews: Mutex::new(HashMap::new()),
            watcher: Mutex::new(None),
            claude: ClaudeEngine::new(),
            agy: AgyEngine::new(),
        }
    }

    /// The open workspace, or a Vietnamese error asking the user to open one.
    pub fn workspace(&self) -> CmdResult<Workspace> {
        let path = self
            .current
            .lock()
            .map_err(|_| CmdError::msg("Trạng thái ứng dụng bị khóa."))?
            .clone()
            .ok_or_else(|| {
                CmdError::msg("Chưa mở workspace nào. Hãy tạo hoặc mở một workspace.")
            })?;
        Workspace::open(&path).map_err(CmdError::from)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
