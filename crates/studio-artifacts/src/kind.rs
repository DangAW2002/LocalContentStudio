use serde::{Deserialize, Serialize};

/// Kind specification in the kind registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KindSpec {
    /// Unique kind identifier, e.g. "design", "motion", "timeline", "text", "generation", "audio", "reference".
    pub id: String,
    /// Default source filename, e.g. "index.html", "timeline.json", "text.md", "profile.md".
    pub default_source: String,
    /// Default renderer tool id, e.g. "render.html_png@1", "render.html_video@1", "render.timeline_ffmpeg@1".
    pub renderer: Option<String>,
    /// Expected output formats, e.g. ["png", "pdf"], ["mp4", "webm"].
    pub outputs: Vec<String>,
    /// Viewer type name for UI stage rendering, e.g. "canvas", "player", "timeline", "text", "reference".
    pub viewer: String,
}

/// Registry of known artifact kinds.
pub struct KindRegistry;

impl KindRegistry {
    /// Return all standard registered artifact kinds.
    pub fn all() -> Vec<KindSpec> {
        vec![
            Self::design(),
            Self::motion(),
            Self::timeline(),
            Self::text(),
            Self::generation(),
            Self::audio(),
            Self::reference(),
        ]
    }

    /// Find a kind specification by id.
    pub fn get(kind_id: &str) -> Option<KindSpec> {
        Self::all().into_iter().find(|k| k.id == kind_id)
    }

    /// Design kind: HTML/CSS artboards rendered with Chromium.
    pub fn design() -> KindSpec {
        KindSpec {
            id: "design".into(),
            default_source: "index.html".into(),
            renderer: Some("render.html_png@1".into()),
            outputs: vec!["png".into(), "pdf".into()],
            viewer: "canvas".into(),
        }
    }

    /// Motion kind: HyperFrames HTML composition rendered with HyperFrames CLI.
    pub fn motion() -> KindSpec {
        KindSpec {
            id: "motion".into(),
            default_source: "index.html".into(),
            renderer: Some("render.html_video@1".into()),
            outputs: vec!["mp4".into(), "webm".into()],
            viewer: "player".into(),
        }
    }

    /// Timeline kind: JSON sequence rendered with FFmpeg filtergraph.
    pub fn timeline() -> KindSpec {
        KindSpec {
            id: "timeline".into(),
            default_source: "timeline.json".into(),
            renderer: Some("render.timeline_ffmpeg@1".into()),
            outputs: vec!["mp4".into(), "srt".into()],
            viewer: "timeline".into(),
        }
    }

    /// Text kind: Markdown with channel frontmatter.
    pub fn text() -> KindSpec {
        KindSpec {
            id: "text".into(),
            default_source: "text.md".into(),
            renderer: None,
            outputs: vec!["txt".into(), "md".into()],
            viewer: "text".into(),
        }
    }

    /// Generation kind: Model prompt & seed parameters.
    pub fn generation() -> KindSpec {
        KindSpec {
            id: "generation".into(),
            default_source: "recipe.json".into(),
            renderer: None,
            outputs: vec!["asset".into()],
            viewer: "generation".into(),
        }
    }

    /// Audio kind: Voice script / TTS config.
    pub fn audio() -> KindSpec {
        KindSpec {
            id: "audio".into(),
            default_source: "script.md".into(),
            renderer: Some("tts.synthesize@1".into()),
            outputs: vec!["wav".into(), "mp3".into()],
            viewer: "audio".into(),
        }
    }

    /// Reference kind: Consistency profile (character, style, voice, fixed clips).
    pub fn reference() -> KindSpec {
        KindSpec {
            id: "reference".into(),
            default_source: "profile.md".into(),
            renderer: None,
            outputs: vec!["md".into()],
            viewer: "reference".into(),
        }
    }
}
