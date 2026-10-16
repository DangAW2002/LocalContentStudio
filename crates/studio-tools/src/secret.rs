//! Secret management and Credential Manager integration.

use crate::error::{Result, ToolError};
use std::collections::HashMap;
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

/// Windows Credential Manager store.
/// Uses target names: `LocalContentStudio/<org>[/<ws>]/<KEY>`.
#[derive(Debug, Default)]
pub struct CredentialManagerStore {
    #[cfg(not(windows))]
    fallback: MemoryStore,
}

impl CredentialManagerStore {
    /// Create a new store backed by Windows Credential Manager.
    pub fn new() -> Self {
        Self {
            #[cfg(not(windows))]
            fallback: MemoryStore::new(),
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
        self.fallback.get(org, workspace, key)
    }

    fn set(&self, org: &str, workspace: Option<&str>, key: &str, val: &str) -> Result<()> {
        self.fallback.set(org, workspace, key, val)
    }

    fn delete(&self, org: &str, workspace: Option<&str>, key: &str) -> Result<()> {
        self.fallback.delete(org, workspace, key)
    }
}
