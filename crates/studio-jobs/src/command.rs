//! Command specification and environment sanitization.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Standard environment variables allowed to be passed down from the parent process.
pub const ALLOWED_ENV_VARS: &[&str] = &[
    "SystemRoot",
    "PATH",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "PATHEXT",
    "COMSPEC",
];

/// Specification of a command to be executed by a worker.
///
/// Note: child processes do not inherit the parent environment wholesale.
/// The environment is constructed starting from an empty map plus the allowed system variables,
/// then overwritten by the explicit `env` map in this spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSpec {
    /// Program executable name or path.
    pub program: String,
    /// Arguments passed to the program.
    pub args: Vec<String>,
    /// Working directory for execution.
    pub cwd: PathBuf,
    /// Explicit environment variables to provide.
    pub env: BTreeMap<String, String>,
    /// Optional stdin bytes to pipe to the child process.
    pub stdin: Option<Vec<u8>>,
}

impl CommandSpec {
    /// Creates a new `CommandSpec` with empty explicit env and no stdin.
    pub fn new(program: impl Into<String>, args: Vec<String>, cwd: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args,
            cwd: cwd.into(),
            env: BTreeMap::new(),
            stdin: None,
        }
    }

    /// Build the full sanitized environment map that should be given to the child process.
    ///
    /// Starts with an empty environment, copies allowed variables from the current process
    /// (case-insensitively matched for Windows compatibility), then merges explicit `self.env`.
    pub fn build_sanitized_env(&self) -> BTreeMap<String, String> {
        let mut sanitized = BTreeMap::new();

        // Query current process environment
        let current_env: Vec<(String, String)> = std::env::vars().collect();

        for allowed_key in ALLOWED_ENV_VARS {
            // Find in current environment ignoring case on Windows
            let found = current_env.iter().find(|(k, _)| {
                #[cfg(windows)]
                {
                    k.eq_ignore_ascii_case(allowed_key)
                }
                #[cfg(not(windows))]
                {
                    k == allowed_key
                }
            });

            if let Some((k, v)) = found {
                sanitized.insert(k.clone(), v.clone());
            }
        }

        // Apply explicit user env vars
        for (k, v) in &self.env {
            sanitized.insert(k.clone(), v.clone());
        }

        sanitized
    }
}
