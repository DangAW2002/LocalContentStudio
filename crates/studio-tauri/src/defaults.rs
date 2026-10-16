//! First-run installation of app-layer defaults (channels, playbooks, example brand) and the catalog.

use std::path::Path;

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

/// Install defaults without overwriting files the user already has; refresh the app-layer catalog.
pub fn install(shared_dir: &Path, catalog_path: &Path) -> std::io::Result<()> {
    for (rel, contents) in SHARED {
        let target = shared_dir.join(rel);
        if !target.exists() {
            if let Some(dir) = target.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&target, contents)?;
        }
    }
    if let Some(dir) = catalog_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if std::fs::read_to_string(catalog_path).ok().as_deref() != Some(CATALOG) {
        std::fs::write(catalog_path, CATALOG)?;
    }
    Ok(())
}
