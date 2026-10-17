//! Dynamic PATH management: on Windows, combines current process PATH with
//! latest user and machine PATH values from the Windows registry, expanding
//! REG_EXPAND_SZ environment variables, deduplicating paths (process PATH first),
//! and updating the process environment so spawned child processes inherit it.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Cache TTL for refreshed PATH (5 seconds).
const CACHE_TTL: Duration = Duration::from_secs(5);

struct PathCache {
    last_updated: Instant,
    paths: Vec<PathBuf>,
}

static CACHE: Mutex<Option<PathCache>> = Mutex::new(None);

/// Clean and normalize a path entry for PATH search:
/// - Strips quotes
/// - Strips verbatim prefixes (`\\?\` or `\\?\UNC\`)
/// - Trims trailing slashes/backslashes (preserving drive roots like `C:\`)
pub fn clean_path_entry(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim().trim_matches('"').trim();
    if trimmed.is_empty() {
        return None;
    }

    let stripped = if let Some(rest) = trimmed.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = trimmed.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        trimmed.to_string()
    };

    let stripped_trimmed = stripped.trim_end_matches(['/', '\\']);
    let normalized = if stripped_trimmed.len() == 2 && stripped_trimmed.ends_with(':') {
        format!("{stripped_trimmed}\\")
    } else if stripped_trimmed.is_empty() {
        stripped
    } else {
        stripped_trimmed.to_string()
    };

    Some(PathBuf::from(normalized))
}

/// Expand `%VAR%` variables in a string using a provided lookup function.
///
/// If a variable is unknown, it remains `%VAR%` as standard Windows behavior.
pub fn expand_vars_with<F>(input: &str, lookup: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let mut var_name = String::new();
            let mut found_close = false;

            while let Some(&next_c) = chars.peek() {
                if next_c == '%' {
                    chars.next();
                    found_close = true;
                    break;
                } else if next_c == ';' || next_c == '\\' || next_c == '/' {
                    break;
                } else {
                    var_name.push(chars.next().unwrap());
                }
            }

            if found_close && !var_name.is_empty() {
                if let Some(val) = lookup(&var_name) {
                    result.push_str(&val);
                } else {
                    result.push('%');
                    result.push_str(&var_name);
                    result.push('%');
                }
            } else {
                result.push('%');
                result.push_str(&var_name);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Merge process PATH, user PATH, and machine PATH:
/// - Process PATH first
/// - Expands `%VAR%` in user and machine PATH using `expander`
/// - Deduplicates paths while preserving first-seen order (case-insensitive on Windows)
pub fn merge_paths_with_expander<F>(
    process_path: &str,
    user_path: &str,
    machine_path: &str,
    expander: F,
) -> Vec<PathBuf>
where
    F: Fn(&str) -> String,
{
    let expanded_user = expander(user_path);
    let expanded_machine = expander(machine_path);

    let mut out = Vec::new();
    let mut seen = HashSet::new();

    let split_entries = |path_str: &str| -> Vec<String> {
        std::env::split_paths(&OsString::from(path_str))
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    };

    let all_entries = split_entries(process_path)
        .into_iter()
        .chain(split_entries(&expanded_user))
        .chain(split_entries(&expanded_machine));

    for entry in all_entries {
        if let Some(cleaned) = clean_path_entry(&entry) {
            let key = if cfg!(windows) {
                cleaned.to_string_lossy().to_lowercase()
            } else {
                cleaned.to_string_lossy().to_string()
            };

            if seen.insert(key) {
                out.push(cleaned);
            }
        }
    }

    out
}

#[cfg(windows)]
unsafe fn read_reg_string(
    hkey: windows_sys::Win32::System::Registry::HKEY,
    subkey: &str,
    value_name: &str,
) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, KEY_READ,
    };

    let subkey_w: Vec<u16> = std::ffi::OsStr::new(subkey)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let value_name_w: Vec<u16> = std::ffi::OsStr::new(value_name)
        .encode_wide()
        .chain(Some(0))
        .collect();

    let mut key_handle = 0;
    if RegOpenKeyExW(hkey, subkey_w.as_ptr(), 0, KEY_READ, &mut key_handle) != ERROR_SUCCESS {
        return None;
    }

    let mut val_type = 0u32;
    let mut data_len = 0u32;
    let res = RegQueryValueExW(
        key_handle,
        value_name_w.as_ptr(),
        std::ptr::null_mut(),
        &mut val_type,
        std::ptr::null_mut(),
        &mut data_len,
    );

    if res != ERROR_SUCCESS || data_len == 0 {
        RegCloseKey(key_handle);
        return None;
    }

    let mut buf = vec![0u8; data_len as usize];
    let res = RegQueryValueExW(
        key_handle,
        value_name_w.as_ptr(),
        std::ptr::null_mut(),
        &mut val_type,
        buf.as_mut_ptr(),
        &mut data_len,
    );
    RegCloseKey(key_handle);

    if res != ERROR_SUCCESS {
        return None;
    }

    // Decode byte pairs instead of casting the u8 buffer to &[u16] (no alignment guarantee).
    let wide: Vec<u16> = buf[..(data_len as usize).min(buf.len())]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let u16_slice: &[u16] = &wide;
    let end = u16_slice
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(u16_slice.len());
    Some(String::from_utf16_lossy(&u16_slice[..end]))
}

#[cfg(windows)]
fn expand_reg_sz(raw: &str) -> String {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;

    let wide: Vec<u16> = std::ffi::OsStr::new(raw)
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe {
        let len = ExpandEnvironmentStringsW(wide.as_ptr(), std::ptr::null_mut(), 0);
        if len == 0 {
            return raw.to_string();
        }
        let mut buf = vec![0u16; len as usize];
        let res = ExpandEnvironmentStringsW(wide.as_ptr(), buf.as_mut_ptr(), len);
        if res == 0 {
            return raw.to_string();
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }
}

/// Retrieve search paths.
///
/// On Windows:
/// Reads current process PATH, HKCU\Environment Path, and
/// HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment Path.
/// Merges and deduplicates them (process PATH first), expands %VAR%,
/// updates the current process environment so spawned child processes inherit the refreshed PATH,
/// and caches the result for a few seconds. If `force` is true, cache is bypassed.
///
/// On non-Windows: returns current process PATH unchanged.
pub fn get_fresh_paths(force: bool) -> Vec<PathBuf> {
    #[cfg(not(windows))]
    {
        let _ = force;
        let path_var = std::env::var_os("PATH").unwrap_or_default();
        std::env::split_paths(&path_var).collect()
    }
    #[cfg(windows)]
    {
        if !force {
            if let Ok(guard) = CACHE.lock() {
                if let Some(cache) = guard.as_ref() {
                    if cache.last_updated.elapsed() < CACHE_TTL {
                        return cache.paths.clone();
                    }
                }
            }
        }

        let proc_path = std::env::var("PATH").unwrap_or_default();
        let user_path = unsafe {
            read_reg_string(
                windows_sys::Win32::System::Registry::HKEY_CURRENT_USER,
                "Environment",
                "Path",
            )
        }
        .unwrap_or_default();

        let machine_path = unsafe {
            read_reg_string(
                windows_sys::Win32::System::Registry::HKEY_LOCAL_MACHINE,
                r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
                "Path",
            )
        }
        .unwrap_or_default();

        let paths = merge_paths_with_expander(&proc_path, &user_path, &machine_path, expand_reg_sz);

        // Update current process PATH so child processes inherit it
        if let Ok(joined) = std::env::join_paths(&paths) {
            std::env::set_var("PATH", &joined);
        }

        if let Ok(mut guard) = CACHE.lock() {
            *guard = Some(PathCache {
                last_updated: Instant::now(),
                paths: paths.clone(),
            });
        }

        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_expand_vars_with_injected() {
        let mut vars = HashMap::new();
        vars.insert("USERPROFILE".to_string(), r"C:\Users\Tester".to_string());
        vars.insert("PROGRAMFILES".to_string(), r"C:\Program Files".to_string());

        let lookup = |name: &str| vars.get(name).cloned();

        let input =
            r"%USERPROFILE%\AppData\Local\Programs;%PROGRAMFILES%\Git\cmd;%UNKNOWN_VAR%\bin";
        let expanded = expand_vars_with(input, lookup);

        assert_eq!(
            expanded,
            r"C:\Users\Tester\AppData\Local\Programs;C:\Program Files\Git\cmd;%UNKNOWN_VAR%\bin"
        );
    }

    #[test]
    fn test_merge_dedupe_process_path_first() {
        let proc_path = r"C:\ProcessBin;C:\Tools;\\?\C:\Normalized";
        let user_path = r"%USER_DIR%\bin;c:\tools\;C:\ExtraUser";
        let machine_path = r"C:\Windows\System32;C:\ProcessBin\;%SYS_DIR%\tools";

        let mut vars = HashMap::new();
        vars.insert("USER_DIR".to_string(), r"C:\Users\Tester".to_string());
        vars.insert("SYS_DIR".to_string(), r"C:\MachineTools".to_string());

        let expander = |raw: &str| expand_vars_with(raw, |name| vars.get(name).cloned());

        let merged = merge_paths_with_expander(proc_path, user_path, machine_path, expander);

        // Convert all to strings for checking
        let merged_strs: Vec<String> = merged
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        // 1. Process paths come first
        assert_eq!(merged_strs[0], r"C:\ProcessBin");
        assert_eq!(merged_strs[1], r"C:\Tools");
        assert_eq!(merged_strs[2], r"C:\Normalized");

        // 2. User paths added (expanded)
        assert_eq!(merged_strs[3], r"C:\Users\Tester\bin");
        // c:\tools\ is duplicate of C:\Tools -> MUST be dropped!
        assert!(!merged_strs.contains(&r"c:\tools\".to_string()));
        assert_eq!(merged_strs[4], r"C:\ExtraUser");

        // 3. Machine paths added
        assert_eq!(merged_strs[5], r"C:\Windows\System32");
        // C:\ProcessBin\ is duplicate of C:\ProcessBin -> MUST be dropped!
        assert_eq!(merged_strs[6], r"C:\MachineTools\tools");

        // Total should be exactly 7 entries
        assert_eq!(merged.len(), 7);
    }
}
