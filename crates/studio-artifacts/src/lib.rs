//! `studio-artifacts` crate — manages artifact definitions, kind registry,
//! starter sources, deterministic input hash caching, render job generation,
//! proxy creation, and output validation for Local Content Studio.

pub mod artifact;
pub mod cache;
pub mod error;
pub mod graph;
pub mod graph_edit;
pub mod kind;
pub mod probe;
pub mod render;
pub mod renderer;
pub mod starter;
pub mod timeline;
pub mod timeline_build;
pub mod validate;

pub use artifact::{Artifact, ArtifactInput, InputRole};
pub use cache::{
    compute_input_hash, hash12, latest_render_on_disk, lookup_render_cache, record_render_outputs,
    RenderRecord,
};
pub use error::{ArtifactError, Result};
pub use graph::{
    build_project_graph, calculate_impact, detect_cycle, normalize_ref, topo_sort, ContentGraph,
    GraphEdge, GraphNode, ImpactNode, NodeStatus,
};
pub use graph_edit::*;
pub use kind::{KindRegistry, KindSpec};
pub use probe::{probe_chromium, probe_ffmpeg, probe_ffprobe, probe_node_npx, ProbeReport};
pub use render::{
    effective_renderer, render, render_cache_key, render_options_for, render_with,
    timeline_schema_hint, RenderOptions, RenderOutcome,
};
pub use renderer::{
    build_html_render_job, build_hyperframes_render_job, build_proxy_job,
    build_timeline_ffmpeg_args, build_timeline_render_job, calculate_timeline_ram_mb,
    detect_artboard_size, escape_drawtext_text, escape_ffmpeg_subtitles_path, file_url_for,
    is_image_source, is_subtitle_file, is_video_source, strip_verbatim,
};
pub use starter::*;
pub use timeline::{
    resolve_auto_motion, CaptionStyle, Clip, MotionKind, TimelineDoc, Track, TrackKind, Transition,
};
pub use timeline_build::{
    build_timeline, build_timeline_tool, OverlayInput, SegmentInput, SegmentOutput,
    TimelineBuildParams, TimelineBuildResult, VisualInput, VisualSpec,
};
pub use validate::{validate_png, validate_video, ImageMeta, VideoMeta};
