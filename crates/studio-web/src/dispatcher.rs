//! Abstraction for invoking command implementations and accessing workspace state.

use serde_json::Value;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Dispatcher trait implemented by the application core to handle web requests.
pub trait WebDispatcher: Send + Sync + 'static {
    /// Invoke a command implementation with JSON arguments.
    fn invoke<'a>(
        &'a self,
        command: &'a str,
        args: Value,
        is_sudo: bool,
    ) -> BoxFuture<'a, Result<Value, String>>;

    /// Resolve a scoped file inside the active workspace, ensuring no traversal outside.
    fn resolve_file(&self, project: Option<&str>, path: &str) -> Result<PathBuf, String>;

    /// Read owner password hash from storage (e.g. app.db).
    fn get_password_hash(&self) -> Result<Option<String>, String>;

    /// Update owner password hash in storage.
    fn set_password_hash(&self, hash: &str) -> Result<(), String>;
}
