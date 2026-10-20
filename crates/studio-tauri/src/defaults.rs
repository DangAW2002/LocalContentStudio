//! First-run installation of app-layer defaults (channels, playbooks, example brand) and the catalog.

use std::collections::BTreeMap;
use std::path::Path;

use include_dir::{include_dir, Dir};
use sha2::{Digest, Sha256};

/// Studio-owned skills (`studio/local` catalog source), installed as `<sources>/studio__local`.
static STUDIO_SKILLS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../catalog/studio");

/// Built-in app-layer files: (path relative to the app shared dir, contents).
const SHARED: &[(&str, &str)] = &[
    (
        "brands/example/brand.yaml",
        include_str!("../../studio-brand/defaults/brands/example/brand.yaml"),
    ),
    (
        "brands/example/logo.svg",
        include_str!("../../studio-brand/defaults/brands/example/logo.svg"),
    ),
    (
        "brands/example/mark.svg",
        include_str!("../../studio-brand/defaults/brands/example/mark.svg"),
    ),
    (
        "brands/example/voice.md",
        include_str!("../../studio-brand/defaults/brands/example/voice.md"),
    ),
    (
        "channels/facebook-feed.yaml",
        include_str!("../../studio-brand/defaults/channels/facebook-feed.yaml"),
    ),
    (
        "channels/instagram-carousel.yaml",
        include_str!("../../studio-brand/defaults/channels/instagram-carousel.yaml"),
    ),
    (
        "channels/instagram-feed.yaml",
        include_str!("../../studio-brand/defaults/channels/instagram-feed.yaml"),
    ),
    (
        "channels/instagram-reels.yaml",
        include_str!("../../studio-brand/defaults/channels/instagram-reels.yaml"),
    ),
    (
        "channels/linkedin.yaml",
        include_str!("../../studio-brand/defaults/channels/linkedin.yaml"),
    ),
    (
        "channels/tiktok.yaml",
        include_str!("../../studio-brand/defaults/channels/tiktok.yaml"),
    ),
    (
        "channels/x.yaml",
        include_str!("../../studio-brand/defaults/channels/x.yaml"),
    ),
    (
        "channels/youtube-shorts.yaml",
        include_str!("../../studio-brand/defaults/channels/youtube-shorts.yaml"),
    ),
    (
        "channels/youtube.yaml",
        include_str!("../../studio-brand/defaults/channels/youtube.yaml"),
    ),
    (
        "playbooks/carousel.md",
        include_str!("../../studio-brand/defaults/playbooks/carousel.md"),
    ),
    (
        "playbooks/shorts.md",
        include_str!("../../studio-brand/defaults/playbooks/shorts.md"),
    ),
    (
        "playbooks/thumbnail.md",
        include_str!("../../studio-brand/defaults/playbooks/thumbnail.md"),
    ),
    (
        "playbooks/youtube-video.md",
        include_str!("../../studio-brand/defaults/playbooks/youtube-video.md"),
    ),
];

/// Built-in catalog (ADR-010); the app layer is product-owned and refreshed on every start.
const CATALOG: &str = include_str!("../../../catalog/catalog.yaml");

/// Shipped contents of defaults that changed later (path, sha256 without CR bytes). A file from an
/// install that predates the manifest and still matches one of these was never edited by the user.
const LEGACY_SHIPPED: &[(&str, &str)] = &[(
    "channels/youtube.yaml",
    "fea796d96bdc29797967054bd3672d1b88a2a45b37bdbef6558fbbfe8d78b32f",
)];

/// Records which default contents were installed, so unedited files can follow later releases.
const MANIFEST: &str = ".studio-defaults.json";

/// Line-ending independent content hash (builds embed CRLF or LF depending on the checkout).
fn content_hash(bytes: &[u8]) -> String {
    let normalized: Vec<u8> = bytes.iter().copied().filter(|b| *b != 13u8).collect();
    format!("{:x}", Sha256::digest(&normalized))
}

/// Install defaults: missing files are written; files the user has not edited (same hash as the
/// version Studio installed) are updated to the bundled version; edited files are left alone.
/// Also refreshes the app-layer catalog and the bundled Studio skills.
pub fn install(shared_dir: &Path, catalog_path: &Path, sources_dir: &Path) -> std::io::Result<()> {
    install_studio_skills(&sources_dir.join("studio__local"))?;
    let manifest_path = shared_dir.join(MANIFEST);
    let mut manifest: BTreeMap<String, String> = std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let before = manifest.clone();
    for (rel, contents) in SHARED {
        let target = shared_dir.join(rel);
        let bundled = content_hash(contents.as_bytes());
        let write = match std::fs::read(&target) {
            Err(_) => true,
            Ok(current) => {
                let current = content_hash(&current);
                let unedited = manifest.get(*rel) == Some(&current)
                    || LEGACY_SHIPPED.contains(&(*rel, current.as_str()));
                if current == bundled {
                    manifest.insert((*rel).to_string(), bundled.clone());
                }
                unedited && current != bundled
            }
        };
        if write {
            if let Some(dir) = target.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&target, contents)?;
            manifest.insert((*rel).to_string(), bundled);
        }
    }
    if manifest != before {
        std::fs::create_dir_all(shared_dir)?;
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    }
    if let Some(dir) = catalog_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if std::fs::read_to_string(catalog_path).ok().as_deref() != Some(CATALOG) {
        std::fs::write(catalog_path, CATALOG)?;
    }
    Ok(())
}

/// Replace `<target>` with the bundled skills when their content changed (marker = content hash).
fn install_studio_skills(target: &Path) -> std::io::Result<()> {
    let mut files = Vec::new();
    collect(&STUDIO_SKILLS, &mut files);
    files.sort_by(|a, b| a.path().cmp(b.path()));
    let mut hasher = Sha256::new();
    for f in &files {
        hasher.update(f.path().to_string_lossy().as_bytes());
        hasher.update(f.contents());
    }
    let hash = format!("{:x}", hasher.finalize());
    let marker = target.join(".studio-bundle");
    if std::fs::read_to_string(&marker).ok().as_deref() == Some(hash.as_str()) {
        return Ok(());
    }
    if target.exists() {
        std::fs::remove_dir_all(target)?;
    }
    for f in &files {
        let path = target.join(f.path());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, f.contents())?;
    }
    std::fs::write(marker, hash)
}

fn collect<'a>(dir: &'a Dir<'a>, out: &mut Vec<&'a include_dir::File<'a>>) {
    out.extend(dir.files());
    for d in dir.dirs() {
        collect(d, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn youtube() -> &'static str {
        SHARED
            .iter()
            .find(|(rel, _)| *rel == "channels/youtube.yaml")
            .map(|(_, c)| *c)
            .unwrap()
    }

    #[test]
    fn unedited_defaults_follow_the_bundle_and_edits_are_kept() {
        let tmp = tempfile::tempdir().unwrap();
        let shared = tmp.path().join("shared");
        let catalog = tmp.path().join("catalog/catalog.yaml");
        let sources = tmp.path().join("catalog/sources");
        install(&shared, &catalog, &sources).unwrap();
        let yt = shared.join("channels/youtube.yaml");
        assert_eq!(std::fs::read_to_string(&yt).unwrap(), youtube());

        // Studio wrote an older version (recorded in the manifest): it is updated.
        let old = "id: \"youtube\"\nold: true\n";
        std::fs::write(&yt, old).unwrap();
        let mut m: BTreeMap<String, String> =
            serde_json::from_str(&std::fs::read_to_string(shared.join(MANIFEST)).unwrap()).unwrap();
        m.insert("channels/youtube.yaml".into(), content_hash(old.as_bytes()));
        std::fs::write(shared.join(MANIFEST), serde_json::to_string(&m).unwrap()).unwrap();
        install(&shared, &catalog, &sources).unwrap();
        assert_eq!(std::fs::read_to_string(&yt).unwrap(), youtube());

        // The user edited it: kept as is.
        std::fs::write(&yt, "id: \"youtube\"\nmine: true\n").unwrap();
        install(&shared, &catalog, &sources).unwrap();
        assert!(std::fs::read_to_string(&yt).unwrap().contains("mine: true"));
    }

    #[test]
    fn unknown_file_without_manifest_is_kept() {
        let tmp = tempfile::tempdir().unwrap();
        let shared = tmp.path().join("shared");
        let yt = shared.join("channels/youtube.yaml");
        std::fs::create_dir_all(yt.parent().unwrap()).unwrap();
        let legacy = "id: \"youtube\"\r\nplatform: \"youtube\"\r\n";
        assert_ne!(content_hash(legacy.as_bytes()), LEGACY_SHIPPED[0].1);
        std::fs::write(&yt, legacy).unwrap();
        install(&shared, &tmp.path().join("c.yaml"), &tmp.path().join("s")).unwrap();
        // Not a known shipped version → treated as a user edit and kept.
        assert_eq!(std::fs::read_to_string(&yt).unwrap(), legacy);
    }
}
