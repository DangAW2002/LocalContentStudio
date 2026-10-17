//! Engine execution trait, Turn handle with process management, and live probe implementations.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::builder::{agy_command, claude_command, CommandSpec};
use crate::parser_agy::AgyParser;
use crate::parser_claude::ClaudeParser;
use crate::types::{DoneStatus, EngineError, Event, Probe, TurnRequest};

/// Known verified versions of CLI engines.
pub const SUPPORTED_CLAUDE_VERSION: &str = "2.1.299";
pub const SUPPORTED_AGY_VERSION: &str = "1.3.5";

/// Trait implemented by agent engine adapters.
pub trait Engine: Send + Sync {
    /// Probes the system to detect installation, version, and compatibility status.
    fn probe(&self) -> impl std::future::Future<Output = Probe> + Send;

    /// Starts a turn execution for the specified request.
    fn start_turn(
        &self,
        req: TurnRequest,
    ) -> impl std::future::Future<Output = Result<Turn, EngineError>> + Send;
}

/// Active turn execution handle providing event streaming and cancellation.
pub struct Turn {
    receiver: mpsc::Receiver<Event>,
    child_pid: Option<u32>,
    is_cancelled: Arc<AtomicBool>,
    temp_files: Vec<PathBuf>,
}

impl Turn {
    /// Receives the next normalized event from the running engine.
    /// Returns `None` when the engine process has exited and all events have been yielded.
    pub async fn next_event(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }

    /// Cancels the running turn.
    ///
    /// Terminates the process tree (using `taskkill /T /F /PID` on Windows, or SIGKILL on Unix).
    /// Emits `Event::Done { status: DoneStatus::Cancelled, text: None }`.
    pub async fn cancel(&mut self) -> Result<(), EngineError> {
        if self.is_cancelled.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        if let Some(pid) = self.child_pid {
            kill_process_tree(pid).await?;
        }

        self.cleanup_temp_files();
        Ok(())
    }

    fn cleanup_temp_files(&mut self) {
        for path in self.temp_files.drain(..) {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        self.cleanup_temp_files();
    }
}

/// Kills a process and its child processes.
pub async fn kill_process_tree(pid: u32) -> Result<(), EngineError> {
    #[cfg(windows)]
    {
        let status = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match status {
            Ok(_) => Ok(()),
            Err(e) => Err(EngineError::Io(e)),
        }
    }

    #[cfg(not(windows))]
    {
        let status = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match status {
            Ok(_) => Ok(()),
            Err(e) => Err(EngineError::Io(e)),
        }
    }
}

/// Claude Code engine adapter.
#[derive(Debug, Clone)]
pub struct ClaudeEngine {
    program_path: PathBuf,
}

impl ClaudeEngine {
    /// Creates a new ClaudeEngine with the default executable name ("claude").
    pub fn new() -> Self {
        Self {
            program_path: PathBuf::from("claude"),
        }
    }

    /// Creates a ClaudeEngine with a custom path to the executable.
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            program_path: path.into(),
        }
    }
}

impl Default for ClaudeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for ClaudeEngine {
    async fn probe(&self) -> Probe {
        probe_cli(&self.program_path, SUPPORTED_CLAUDE_VERSION).await
    }

    async fn start_turn(&self, req: TurnRequest) -> Result<Turn, EngineError> {
        let tmp_dir = std::env::temp_dir();
        let spec = claude_command(&req, &tmp_dir, Some(&self.program_path))?;
        spawn_turn_process(spec, req.workspace_dir, true).await
    }
}

/// Antigravity (agy) engine adapter.
#[derive(Debug, Clone)]
pub struct AgyEngine {
    program_path: PathBuf,
}

impl AgyEngine {
    /// Creates a new AgyEngine with the default executable name ("agy").
    pub fn new() -> Self {
        Self {
            program_path: PathBuf::from("agy"),
        }
    }

    /// Creates an AgyEngine with a custom path to the executable.
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            program_path: path.into(),
        }
    }
}

impl Default for AgyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for AgyEngine {
    async fn probe(&self) -> Probe {
        probe_cli(&self.program_path, SUPPORTED_AGY_VERSION).await
    }

    async fn start_turn(&self, req: TurnRequest) -> Result<Turn, EngineError> {
        let tmp_dir = std::env::temp_dir();
        let spec = agy_command(&req, &tmp_dir, Some(&self.program_path))?;
        spawn_turn_process(spec, req.workspace_dir, false).await
    }
}

/// A model the engine CLI accepts for `--model`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ModelInfo {
    /// Value passed to `--model`.
    pub id: String,
    /// Human-readable label.
    pub label: String,
}

impl ClaudeEngine {
    /// Models for Claude Code. The CLI has no listing command; aliases always resolve to the latest model
    /// of each family, so this list does not go stale when new versions ship.
    pub fn list_models(&self) -> Vec<ModelInfo> {
        [
            ("opus", "Opus (mới nhất)"),
            ("sonnet", "Sonnet (mới nhất)"),
            ("haiku", "Haiku (mới nhất, nhanh)"),
            ("fable", "Fable (mới nhất)"),
        ]
        .into_iter()
        .map(|(id, label)| ModelInfo {
            id: id.into(),
            label: label.into(),
        })
        .collect()
    }
}

impl AgyEngine {
    /// Models available to this agy install and account, from `agy models` (live, needs login).
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, EngineError> {
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(45),
            Command::new(&self.program_path)
                .arg("models")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .map_err(|_| EngineError::Io(std::io::Error::other("agy models: quá thời gian chờ")))?
        .map_err(EngineError::Io)?;
        let models = parse_agy_models(&String::from_utf8_lossy(&output.stdout));
        if models.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EngineError::Io(std::io::Error::other(format!(
                "agy models không trả về model nào: {}",
                stderr.trim()
            ))));
        }
        Ok(models)
    }
}

/// Parses `agy models` output: one `id<TAB>label` per line; other lines (progress text) are ignored.
pub fn parse_agy_models(raw: &str) -> Vec<ModelInfo> {
    raw.lines()
        .filter_map(|line| {
            let (id, label) = line.split_once('\t')?;
            let id = id.trim();
            if id.is_empty() || id.contains(' ') {
                return None;
            }
            Some(ModelInfo {
                id: id.to_string(),
                label: label.trim().to_string(),
            })
        })
        .collect()
}

async fn probe_cli(program: &Path, expected_version: &str) -> Probe {
    let output = match Command::new(program)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
    {
        Ok(out) => out,
        Err(_) => {
            return Probe {
                installed: false,
                version: None,
                supported: false,
                note: Some("Không tìm thấy tệp thực thi của engine".to_string()),
            };
        }
    };

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let version = parse_version_string(&stdout_str);

    match version {
        Some(v) => {
            let is_supported = v == expected_version;
            let note = if is_supported {
                None
            } else {
                Some("phiên bản chưa được kiểm chứng".to_string())
            };

            Probe {
                installed: true,
                version: Some(v),
                supported: is_supported,
                note,
            }
        }
        None => Probe {
            installed: true,
            version: None,
            supported: false,
            note: Some("phiên bản chưa được kiểm chứng".to_string()),
        },
    }
}

fn parse_version_string(raw: &str) -> Option<String> {
    for part in raw.split_whitespace() {
        let cleaned = part.trim_matches(|c: char| !c.is_numeric() && c != '.');
        if !cleaned.is_empty() && cleaned.contains('.') {
            return Some(cleaned.to_string());
        }
    }
    None
}

async fn spawn_turn_process(
    spec: CommandSpec,
    cwd: PathBuf,
    is_claude: bool,
) -> Result<Turn, EngineError> {
    let mut cmd = Command::new(&spec.program);
    cmd.envs(spec.env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    cmd.args(&spec.args)
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let pid = child.id();

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| EngineError::Process("Failed to capture process stdin".to_string()))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| EngineError::Process("Failed to capture process stdout".to_string()))?;

    // Send the single JSON line to stdin and flush/close
    tokio::spawn(async move {
        let _ = stdin.write_all(spec.stdin_line.as_bytes()).await;
        let _ = stdin.write_all(b"\n").await;
        let _ = stdin.flush().await;
        drop(stdin);
    });

    let (tx, rx) = mpsc::channel(128);
    let is_cancelled = Arc::new(AtomicBool::new(false));
    let is_cancelled_clone = Arc::clone(&is_cancelled);

    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();

        if is_claude {
            let mut parser = ClaudeParser::new();
            while let Ok(Some(line)) = reader.next_line().await {
                if is_cancelled_clone.load(Ordering::SeqCst) {
                    break;
                }
                let events = parser.feed(&line);
                for ev in events {
                    if tx.send(ev).await.is_err() {
                        return;
                    }
                }
            }
        } else {
            let mut parser = AgyParser::new();
            while let Ok(Some(line)) = reader.next_line().await {
                if is_cancelled_clone.load(Ordering::SeqCst) {
                    break;
                }
                let events = parser.feed(&line);
                for ev in events {
                    if tx.send(ev).await.is_err() {
                        return;
                    }
                }
            }
        }

        // Wait for child exit
        let _ = child.wait().await;

        if is_cancelled_clone.load(Ordering::SeqCst) {
            let _ = tx
                .send(Event::Done {
                    status: DoneStatus::Cancelled,
                    text: None,
                })
                .await;
        }
    });

    Ok(Turn {
        receiver: rx,
        child_pid: pid,
        is_cancelled,
        temp_files: spec.temp_files,
    })
}
