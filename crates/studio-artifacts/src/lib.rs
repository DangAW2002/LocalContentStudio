//! `studio-artifacts` crate — manages artifact definitions, kind registry,
//! starter sources, deterministic input hash caching, render job generation,
//! proxy creation, and output validation for Local Content Studio.

pub mod artifact;
pub mod cache;
pub mod error;
pub mod kind;
pub mod probe;
pub mod render;
pub mod renderer;
pub mod starter;
pub mod timeline;
pub mod validate;

pub use artifact::Artifact;
pub use cache::{
    compute_input_hash, hash12, lookup_render_cache, record_render_outputs, RenderRecord,
};
pub use error::{ArtifactError, Result};
pub use kind::{KindRegistry, KindSpec};
pub use probe::{probe_chromium, probe_ffmpeg, probe_ffprobe, probe_node_npx, ProbeReport};
pub use render::{render, render_with, RenderOptions, RenderOutcome};
pub use renderer::{
    build_html_render_job, build_hyperframes_render_job, build_proxy_job,
    build_timeline_ffmpeg_args, build_timeline_render_job, detect_artboard_size,
    escape_ffmpeg_subtitles_path, file_url_for, is_image_source, is_subtitle_file, strip_verbatim,
};
pub use starter::*;
pub use timeline::{Clip, TimelineDoc, Track, TrackKind, Transition};
pub use validate::{validate_png, validate_video, ImageMeta, VideoMeta};
