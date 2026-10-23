//! Secret management and Credential Manager integration.

use crate::error::{Result, ToolError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Interface for retrieving and managing secrets across the application scope chain.
pub trait SecretStore: Send + Sync {
    /// Retrieve secret value inspecting workspace -> org -> app scope chain.
    fn get(&self, org: &str, workspace: Option<&str>, key: &str) -> Option<String>;

    /// Set a secret value for a specific scope.
    fn set(&self, org: &str, workspace: Option<&str>, key: &str, val: &str) -> Result<()>;

    /// Delete a secret value for a specific scope.
    fn delete(&self, org: &str, workspace: Option<&str>, key: &str) -> Result<()>;
}

/// Compute canonical target name: `LocalContentStudio/<org>[/<ws>]/<KEY>`.
pub fn format_target_name(org: &str, workspace: Option<&str>, key: &str) -> String {
    if org.is_empty() {
        format!("LocalContentStudio/{}", key)
    } else if let Some(ws) = workspace {
        format!("LocalContentStudio/{}/{}/{}", org, ws, key)
    } else {
        format!("LocalContentStudio/{}/{}", org, key)
    }
}

/// In-memory secret store primarily used for unit testing.
#[derive(Debug, Default)]
pub struct MemoryStore {
    data: RwLock<HashMap<String, String>>,
}

impl MemoryStore {
    /// Create a new empty in-memory secret store.
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, org: &str, workspace: Option<&str>, key: &str) -> Option<String> {
        let lock = self.data.read().ok()?;

        // 1. Workspace scope
        if let Some(ws) = workspace {
            let target = format_target_name(org, Some(ws), key);
            if let Some(val) = lock.get(&target) {
                return Some(val.clone());
            }
        }

        // 2. Org scope
        let org_target = format_target_name(org, None, key);
        if let Some(val) = lock.get(&org_target) {
            return Some(val.clone());
        }

        // 3. App scope
        let app_target = format_target_name("", None, key);
        lock.get(&app_target).cloned()
    }

    fn set(&self, org: &str, workspace: Option<&str>, key: &str, val: &str) -> Result<()> {
        let mut lock = self
            .data
            .write()
            .map_err(|e| ToolError::SecretStoreError(e.to_string()))?;
        let target = format_target_name(org, workspace, key);
        lock.insert(target, val.to_string());
        Ok(())
    }

    fn delete(&self, org: &str, workspace: Option<&str>, key: &str) -> Result<()> {
        let mut lock = self
            .data
            .write()
            .map_err(|e| ToolError::SecretStoreError(e.to_string()))?;
        let target = format_target_name(org, workspace, key);
        lock.remove(&target);
        Ok(())
    }
}

/// Persistent file-backed secret store for non-Windows environments (Linux, macOS) or local testing.
/// Secrets are written atomically (temp file + rename) with permissions 0600 on Unix.
#[derive(Debug)]
pub struct FileSecretStore {
    path: PathBuf,
    data: RwLock<HashMap<String, String>>,
}

impl FileSecretStore {
    /// Create or load a secret store backed by the specified file path.
    pub fn new(path: PathBuf) -> Self {
        let mut map = HashMap::new();
        if path.exists() {
            let parsed = std::fs::read_to_string(&path)
                .ok()
                .and_then(|c| serde_json::from_str::<HashMap<String, String>>(&c).ok());
            match parsed {
                Some(deserialized) => map = deserialized,
                // Keep the unreadable file aside so the next `set` cannot wipe the user's keys.
                None => {
                    let backup = path.with_extension(format!(
                        "json.corrupt-{}",
                        chrono::Utc::now().format("%Y%m%d%H%M%S")
                    ));
                    let _ = std::fs::rename(&path, &backup);
                    eprintln!(
                        "Không đọc được file secrets, đã chuyển sang {}",
                        backup.display()
                    );
                }
            }
        }
        Self {
            path,
            data: RwLock::new(map),
        }
    }

    /// Path to the secrets file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Persist current secrets map to disk atomically with 0600 permissions on Unix.
    fn persist(&self, map: &HashMap<String, String>) -> Result<()> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)
            .map_err(|e| ToolError::SecretStoreError(format!("Lỗi tạo thư mục secrets: {e}")))?;

        let serialized = serde_json::to_string_pretty(map)
            .map_err(|e| ToolError::SecretStoreError(e.to_string()))?;

        let mut temp_file = tempfile::NamedTempFile::new_in(parent)
            .map_err(|e| ToolError::SecretStoreError(format!("Lỗi tạo file tạm: {e}")))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = temp_file
                .as_file()
                .metadata()
                .map_err(|e| ToolError::SecretStoreError(format!("Lỗi metadata: {e}")))?
                .permissions();
            perms.set_mode(0o600);
            temp_file
                .as_file()
                .set_permissions(perms)
                .map_err(|e| ToolError::SecretStoreError(format!("Lỗi phân quyền 0600: {e}")))?;
        }

        use std::io::Write;
        temp_file
            .write_all(serialized.as_bytes())
            .map_err(|e| ToolError::SecretStoreError(format!("Lỗi ghi dữ liệu: {e}")))?;
        temp_file
            .flush()
            .map_err(|e| ToolError::SecretStoreError(format!("Lỗi flush file: {e}")))?;
        temp_file
            .as_file()
            .sync_all()
            .map_err(|e| ToolError::SecretStoreError(format!("Lỗi sync file: {e}")))?;

        temp_file.persist(&self.path).map_err(|e| {
            ToolError::SecretStoreError(format!("Không thể lưu file secret: {}", e.error))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&self.path)
                .map_err(|e| ToolError::SecretStoreError(format!("Lỗi metadata: {e}")))?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&self.path, perms)
                .map_err(|e| ToolError::SecretStoreError(format!("Lỗi phân quyền 0600: {e}")))?;
        }

        Ok(())
    }
}

impl SecretStore for FileSecretStore {
    fn get(&self, org: &str, workspace: Option<&str>, key: &str) -> Option<String> {
        let lock = self.data.read().ok()?;

        // 1. Workspace scope
        if let Some(ws) = workspace {
            let target = format_target_name(org, Some(ws), key);
            if let Some(val) = lock.get(&target) {
                return Some(val.clone());
            }
        }

        // 2. Org scope
        let org_target = format_target_name(org, None, key);
        if let Some(val) = lock.get(&org_target) {
            return Some(val.clone());
        }

        // 3. App scope
        let app_target = format_target_name("", None, key);
        lock.get(&app_target).cloned()
    }

    fn set(&self, org: &str, workspace: Option<&str>, key: &str, val: &str) -> Result<()> {
        let mut lock = self
            .data
            .write()
            .map_err(|e| ToolError::SecretStoreError(e.to_string()))?;
        let target = format_target_name(org, workspace, key);
        lock.insert(target, val.to_string());
        self.persist(&lock)
    }

    fn delete(&self, org: &str, workspace: Option<&str>, key: &str) -> Result<()> {
        let mut lock = self
            .data
            .write()
            .map_err(|e| ToolError::SecretStoreError(e.to_string()))?;
        let target = format_target_name(org, workspace, key);
        lock.remove(&target);
        self.persist(&lock)
    }
}

/// Windows Credential Manager store on Windows, or persistent file store on other platforms (Linux).
/// Uses target names: `LocalContentStudio/<org>[/<ws>]/<KEY>`.
#[derive(Debug)]
pub struct CredentialManagerStore {
    #[cfg(not(windows))]
    file_store: FileSecretStore,
}

impl Default for CredentialManagerStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialManagerStore {
    /// Create a new store backed by Windows Credential Manager on Windows,
    /// or `secrets.json` in the app data directory on non-Windows.
    pub fn new() -> Self {
        Self {
            #[cfg(not(windows))]
            file_store: {
                let path = studio_workspace::AppPaths::from_env()
                    .data_dir()
                    .join("secrets.json");
                FileSecretStore::new(path)
            },
        }
    }

    #[cfg(not(windows))]
    /// Create a store with a specific secrets file path (useful for testing or custom dirs).
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            file_store: FileSecretStore::new(path),
        }
    }
}

#[cfg(windows)]
impl SecretStore for CredentialManagerStore {
    fn get(&self, org: &str, workspace: Option<&str>, key: &str) -> Option<String> {
        // Try workspace scope
        if let Some(ws) = workspace {
            let target = format_target_name(org, Some(ws), key);
            if let Some(val) = Self::read_credential(&target) {
                return Some(val);
            }
        }

        // Try org scope
        let org_target = format_target_name(org, None, key);
        if let Some(val) = Self::read_credential(&org_target) {
            return Some(val);
        }

        // Try app scope
        let app_target = format_target_name("", None, key);
        Self::read_credential(&app_target)
    }

    fn set(&self, org: &str, workspace: Option<&str>, key: &str, val: &str) -> Result<()> {
        let target = format_target_name(org, workspace, key);
        Self::write_credential(&target, val)
    }

    fn delete(&self, org: &str, workspace: Option<&str>, key: &str) -> Result<()> {
        let target = format_target_name(org, workspace, key);
        Self::delete_credential(&target)
    }
}

#[cfg(windows)]
impl CredentialManagerStore {
    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn read_credential(target: &str) -> Option<String> {
        use std::ptr::null_mut;
        use windows_sys::Win32::Security::Credentials::{
            CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
        };

        let wide_target = Self::to_wide(target);
        let mut cred_ptr: *mut CREDENTIALW = null_mut();

        unsafe {
            let success = CredReadW(wide_target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut cred_ptr);
            if success == 0 || cred_ptr.is_null() {
                return None;
            }

            let cred = &*cred_ptr;
            let blob_slice =
                std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
            let result = String::from_utf8(blob_slice.to_vec()).ok();
            CredFree(cred_ptr as *const std::ffi::c_void);
            result
        }
    }

    fn write_credential(target: &str, val: &str) -> Result<()> {
        use std::ptr::null_mut;
        use windows_sys::Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        };

        let wide_target = Self::to_wide(target);
        let val_bytes = val.as_bytes();

        let mut cred: CREDENTIALW = unsafe { std::mem::zeroed() };
        cred.Type = CRED_TYPE_GENERIC;
        cred.TargetName = wide_target.as_ptr() as *mut u16;
        cred.CredentialBlobSize = val_bytes.len() as u32;
        cred.CredentialBlob = val_bytes.as_ptr() as *mut u8;
        cred.Persist = CRED_PERSIST_LOCAL_MACHINE;
        cred.UserName = null_mut();

        unsafe {
            let success = CredWriteW(&cred, 0);
            if success == 0 {
                return Err(ToolError::SecretStoreError(format!(
                    "CredWriteW failed with OS error: {}",
                    std::io::Error::last_os_error()
                )));
            }
        }

        Ok(())
    }

    fn delete_credential(target: &str) -> Result<()> {
        use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};

        let wide_target = Self::to_wide(target);
        unsafe {
            let success = CredDeleteW(wide_target.as_ptr(), CRED_TYPE_GENERIC, 0);
            if success == 0 {
                // If not found, not an error
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(1168) {
                    // ERROR_NOT_FOUND = 1168
                    return Ok(());
                }
                return Err(ToolError::SecretStoreError(format!(
                    "CredDeleteW failed with OS error: {}",
                    err
                )));
            }
        }

        Ok(())
    }
}

#[cfg(not(windows))]
impl SecretStore for CredentialManagerStore {
    fn get(&self, org: &str, workspace: Option<&str>, key: &str) -> Option<String> {
        self.file_store.get(org, workspace, key)
    }

    fn set(&self, org: &str, workspace: Option<&str>, key: &str, val: &str) -> Result<()> {
        self.file_store.set(org, workspace, key, val)
    }

    fn delete(&self, org: &str, workspace: Option<&str>, key: &str) -> Result<()> {
        self.file_store.delete(org, workspace, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_secret_store_keeps_corrupt_file_aside() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let path = temp_dir.path().join("secrets.json");
        std::fs::write(&path, "{ not json").expect("write corrupt");

        let store = FileSecretStore::new(path.clone());
        store.set("", None, "NEW_KEY", "v").expect("set");

        let backups: Vec<_> = std::fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".corrupt-"))
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            std::fs::read_to_string(backups[0].path()).expect("read backup"),
            "{ not json"
        );
    }

    #[test]
    fn test_file_secret_store_crud_and_persistence() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let path = temp_dir.path().join("secrets.json");

        // 1. Initial write
        {
            let store = FileSecretStore::new(path.clone());
            store
                .set("org_test", Some("ws_test"), "API_KEY", "secret_ws")
                .expect("set ws");
            store
                .set("org_test", None, "API_KEY", "secret_org")
                .expect("set org");
            store
                .set("", None, "GLOBAL_KEY", "secret_global")
                .expect("set app");

            assert_eq!(
                store.get("org_test", Some("ws_test"), "API_KEY"),
                Some("secret_ws".to_string())
            );
            assert_eq!(
                store.get("org_test", None, "API_KEY"),
                Some("secret_org".to_string())
            );
            assert_eq!(
                store.get("org_test", Some("ws_other"), "API_KEY"),
                Some("secret_org".to_string())
            );
            assert_eq!(
                store.get("org_test", Some("ws_test"), "GLOBAL_KEY"),
                Some("secret_global".to_string())
            );
        }

        // 2. Simulate restart: create brand new store pointing at same file
        {
            let store = FileSecretStore::new(path.clone());
            assert_eq!(
                store.get("org_test", Some("ws_test"), "API_KEY"),
                Some("secret_ws".to_string())
            );
            assert_eq!(
                store.get("org_test", None, "API_KEY"),
                Some("secret_org".to_string())
            );
            assert_eq!(
                store.get("", None, "GLOBAL_KEY"),
                Some("secret_global".to_string())
            );

            // Delete workspace secret -> falls back to org secret
            store
                .delete("org_test", Some("ws_test"), "API_KEY")
                .expect("delete ws");
            assert_eq!(
                store.get("org_test", Some("ws_test"), "API_KEY"),
                Some("secret_org".to_string())
            );

            // Delete org secret -> none
            store
                .delete("org_test", None, "API_KEY")
                .expect("delete org");
            assert_eq!(store.get("org_test", Some("ws_test"), "API_KEY"), None);
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&path).expect("metadata");
            let mode = metadata.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "file permissions must be 0600 on unix");
        }
    }
}
