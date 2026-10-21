pub mod captions_required;
pub mod color;
pub mod consistency;
pub mod contrast;
pub mod declared_inputs;
pub mod dimensions;
pub mod duration;
pub mod logo_presence;
pub mod palette;
pub mod safe_zone;
pub mod text_limits;
pub mod voice_words;

use crate::loader::BrandSet;
use crate::schema::ChannelSpec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use captions_required::CaptionsRequiredChecker;
pub use color::Rgb;
pub use consistency::ConsistencyChecker;
pub use contrast::ContrastChecker;
pub use declared_inputs::DeclaredInputsChecker;
pub use dimensions::DimensionsChecker;
pub use duration::DurationChecker;
pub use logo_presence::LogoPresenceChecker;
pub use palette::PaletteChecker;
pub use safe_zone::SafeZoneChecker;
pub use text_limits::TextLimitsChecker;
pub use voice_words::{normalize_vietnamese, VoiceWordsChecker};

/// Severity level of a checker finding.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational note
    Info,
    /// Warning: should be reviewed, does not necessarily block export
    Warn,
    /// Error: non-compliant with brand or channel requirements
    Error,
}

/// A specific finding produced by a brand/channel checker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Finding {
    /// Severity level
    pub severity: Severity,
    /// Machine-readable code (e.g. `PALETTE_OFF`, `DURATION_EXCEEDED`)
    pub code: String,
    /// User-facing Vietnamese message
    pub message_vi: String,
    /// Optional location or file reference (e.g. `index.html:24` or `title`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Optional Vietnamese fix hint
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix_hint_vi: Option<String>,
}

impl Finding {
    /// Create a new error finding.
    pub fn error(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message_vi: msg.into(),
            location: None,
            fix_hint_vi: None,
        }
    }

    /// Create a new warning finding.
    pub fn warn(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warn,
            code: code.into(),
            message_vi: msg.into(),
            location: None,
            fix_hint_vi: None,
        }
    }

    /// Create a new info finding.
    pub fn info(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            code: code.into(),
            message_vi: msg.into(),
            location: None,
            fix_hint_vi: None,
        }
    }

    /// Set location.
    pub fn with_location(mut self, loc: impl Into<String>) -> Self {
        self.location = Some(loc.into());
        self
    }

    /// Set fix hint.
    pub fn with_fix_hint(mut self, hint: impl Into<String>) -> Self {
        self.fix_hint_vi = Some(hint.into());
        self
    }
}

/// Input context passed to each checker.
#[derive(Debug, Clone)]
pub struct CheckInput<'a> {
    /// Artifact directory path (e.g. `projects/<slug>/artifacts/<id>/`)
    pub artifact_dir: &'a Path,
    /// Main source file contents (if text-based like HTML, Markdown, JSON, SVG)
    pub source_content: Option<&'a str>,
    /// Main source file name (e.g. `index.html`, `text.md`)
    pub source_file: Option<&'a str>,
    /// Rendered output paths (e.g. PNG, MP4, SRT files)
    pub renders: &'a [PathBuf],
    /// ffprobe JSON output for any video renders (parsed)
    pub ffprobe_json: Option<&'a serde_json::Value>,
    /// Brand specification being checked against
    pub brand: &'a crate::schema::BrandSpec,
    /// Absolute directory of the brand (to check logo/font files)
    pub brand_dir: Option<&'a Path>,
    /// Channel specification being checked against
    pub channel: &'a ChannelSpec,
}

/// Checker trait. All checkers must implement this trait.
pub trait Checker: Send + Sync {
    /// Checker unique identifier (e.g. `palette`, `contrast`, `logo_presence`)
    fn id(&self) -> &'static str;

    /// Run verification check on the input.
    fn check(&self, input: &CheckInput) -> Vec<Finding>;
}

/// Final report produced across multiple channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Overall status (true if no errors in any channel)
    pub passed: bool,
    /// Per-channel results: channel_id -> ChannelReport
    pub channels: BTreeMap<String, ChannelReport>,
}

/// Results for a specific channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelReport {
    /// Channel ID
    pub channel_id: String,
    /// Whether all checks passed for this channel (no Error severity)
    pub passed: bool,
    /// List of findings for this channel
    pub findings: Vec<Finding>,
    /// Expected specifications for this channel (formats, text limits, captions)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_specs: Option<ChannelSpec>,
}

/// What is being checked: the artifact source and its renders.
#[derive(Debug, Clone, Copy)]
pub struct CheckSubject<'a> {
    /// Artifact directory.
    pub artifact_dir: &'a Path,
    /// Source file content (HTML/CSS/SVG/Markdown), if any.
    pub source_content: Option<&'a str>,
    /// Source file name relative to the artifact directory.
    pub source_file: Option<&'a str>,
    /// Rendered output files.
    pub renders: &'a [PathBuf],
    /// ffprobe JSON of the main render, if it is a media file.
    pub ffprobe_json: Option<&'a serde_json::Value>,
}

/// Checker registry to hold and execute checkers by ID.
pub struct CheckerRegistry {
    checkers: BTreeMap<&'static str, Box<dyn Checker>>,
}

impl Default for CheckerRegistry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.register(Box::new(PaletteChecker::new()));
        reg.register(Box::new(ContrastChecker::new()));
        reg.register(Box::new(LogoPresenceChecker::new()));
        reg.register(Box::new(DimensionsChecker::new()));
        reg.register(Box::new(DurationChecker::new()));
        reg.register(Box::new(SafeZoneChecker::new()));
        reg.register(Box::new(TextLimitsChecker::new()));
        reg.register(Box::new(CaptionsRequiredChecker::new()));
        reg.register(Box::new(VoiceWordsChecker::new()));
        reg.register(Box::new(DeclaredInputsChecker::new()));
        reg.register(Box::new(ConsistencyChecker::new()));
        reg
    }
}

impl CheckerRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            checkers: BTreeMap::new(),
        }
    }

    /// Register a checker.
    pub fn register(&mut self, checker: Box<dyn Checker>) {
        self.checkers.insert(checker.id(), checker);
    }

    /// Get checker by ID.
    pub fn get(&self, id: &str) -> Option<&dyn Checker> {
        self.checkers.get(id).map(|b| b.as_ref())
    }

    /// List all registered checker IDs.
    pub fn ids(&self) -> Vec<&'static str> {
        self.checkers.keys().copied().collect()
    }

    /// Run checks for an artifact across a set of channels.
    pub fn run_checks(
        &self,
        subject: CheckSubject<'_>,
        brand: &crate::schema::BrandSpec,
        brand_dir: Option<&Path>,
        channels: &[&ChannelSpec],
        checker_ids: Option<&[&str]>,
    ) -> Report {
        let mut channel_reports = BTreeMap::new();
        let mut overall_passed = true;

        for channel in channels {
            let input = CheckInput {
                artifact_dir: subject.artifact_dir,
                source_content: subject.source_content,
                source_file: subject.source_file,
                renders: subject.renders,
                ffprobe_json: subject.ffprobe_json,
                brand,
                brand_dir,
                channel,
            };

            let mut findings = Vec::new();

            for (id, checker) in &self.checkers {
                if let Some(selected) = checker_ids {
                    if !selected.contains(id) {
                        continue;
                    }
                }
                let mut f = checker.check(&input);
                findings.append(&mut f);
            }

            let has_error = findings.iter().any(|f| f.severity == Severity::Error);
            if has_error {
                overall_passed = false;
            }

            channel_reports.insert(
                channel.id.clone(),
                ChannelReport {
                    channel_id: channel.id.clone(),
                    passed: !has_error,
                    findings,
                    expected_specs: Some((*channel).clone()),
                },
            );
        }

        Report {
            passed: overall_passed,
            channels: channel_reports,
        }
    }
}

/// Helper function matching the specification:
/// `run_checks(subject, &BrandSet, brand, channels[]) -> Report`
pub fn run_checks(
    subject: CheckSubject<'_>,
    brand_set: &BrandSet,
    brand_slug: &str,
    channel_ids: &[&str],
) -> Option<Report> {
    let brand = brand_set.get_brand(brand_slug)?;
    let mut channel_specs = Vec::new();
    for ch_id in channel_ids {
        if let Some(ch) = brand_set.get_channel(ch_id) {
            channel_specs.push(ch);
        }
    }

    let registry = CheckerRegistry::default();
    Some(registry.run_checks(subject, &brand.spec, Some(&brand.dir), &channel_specs, None))
}
