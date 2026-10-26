//! Artifacts agy writes outside the workspace (generated images, plans, walkthroughs) live in its
//! per-conversation "brain" folder `~/.gemini/antigravity-cli/brain/<conversation>/`. After a turn
//! the files touched during that turn are copied into the project so they show up as turn output.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Project sub-folder receiving the copies.
pub const DEST_DIR: &str = "agy-artifacts";

/// Brain folder of one agy conversation. `None` for ids that could escape the brain root.
pub fn brain_dir(conversation_id: &str) -> Option<PathBuf> {
    let id = conversation_id.trim();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(
        PathBuf::from(home)
            .join(".gemini")
            .join("antigravity-cli")
            .join("brain")
            .join(id),
    )
}

/// Whether `path` points into an agy brain folder (those edits are reported via the synced copy).
pub fn is_brain_path(path: &Path) -> bool {
    path.to_string_lossy()
        .replace('\\', "/")
        .contains("/.gemini/antigravity-cli/brain/")
}

/// Copies the top-level files of `src` modified at or after `since` into `dest`.
/// Skips hidden entries, sub-folders (`scratch/` holds throwaway scripts) and agy's sidecar
/// metadata. Returns `(copied path, existed before)` pairs.
pub fn sync(src: &Path, dest: &Path, since: SystemTime) -> Vec<(PathBuf, bool)> {
    let Ok(entries) = std::fs::read_dir(src) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let n = name.to_string_lossy();
        if n.starts_with('.') || n.ends_with(".metadata.json") || n.contains(".resolved") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() || meta.modified().map_or(true, |m| m < since) {
            continue;
        }
        if std::fs::create_dir_all(dest).is_err() {
            break;
        }
        let target = dest.join(&name);
        let existed = target.exists();
        if std::fs::copy(entry.path(), &target).is_ok() {
            out.push((target, existed));
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn copies_only_fresh_top_level_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("brain");
        let dest = tmp.path().join("proj").join(DEST_DIR);
        std::fs::create_dir_all(src.join("scratch")).unwrap();
        std::fs::write(src.join("old.md"), "old").unwrap();
        let since = SystemTime::now() + Duration::from_millis(50);
        std::thread::sleep(Duration::from_millis(120));
        std::fs::write(src.join("plan.md"), "plan").unwrap();
        std::fs::write(src.join("cover.png"), [1u8, 2, 3]).unwrap();
        std::fs::write(src.join("plan.md.metadata.json"), "{}").unwrap();
        std::fs::write(src.join(".hidden"), "x").unwrap();
        std::fs::write(src.join("scratch").join("tmp.js"), "x").unwrap();

        let copied = sync(&src, &dest, since);
        let names: Vec<_> = copied
            .iter()
            .map(|(p, _)| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["cover.png", "plan.md"]);
        assert!(copied.iter().all(|(_, existed)| !existed));
        assert_eq!(
            std::fs::read_to_string(dest.join("plan.md")).unwrap(),
            "plan"
        );

        let again = sync(&src, &dest, since);
        assert!(again.iter().all(|(_, existed)| *existed));
    }

    #[test]
    fn rejects_unsafe_conversation_ids() {
        assert!(brain_dir("../etc").is_none());
        assert!(brain_dir("").is_none());
        assert!(brain_dir("71b3bcd3-8678-4036-901d-38b2f6678630").is_some());
    }

    #[test]
    fn detects_brain_paths() {
        assert!(is_brain_path(Path::new(
            "C:\\Users\\a\\.gemini\\antigravity-cli\\brain\\id\\plan.md"
        )));
        assert!(!is_brain_path(Path::new("D:/ws/projects/x/script.md")));
    }
}
