//! Consistency checker for series and profiles.
//!
//! Compares episodes in the same series:
//! 1. Consistent intro/outro clips (same source for first and last video clip in timeline).
//! 2. Consistent thumbnail title font-family.
//! 3. Consistent TTS voice.
//! 4. Presence of declared series character/reference profiles via `role: reference` edges.
//!
//! Produces warnings only (never blocks export).

use crate::checkers::{CheckInput, Checker, Finding};
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static FONT_FAMILY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)font-family\s*:\s*([^;\}]+)"#).unwrap());

static H1_FONT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)h1[^\{]*\{[^}]*font-family\s*:\s*([^;\}]+)"#).unwrap());

/// Checker comparing consistency across episodes in the same series.
pub struct ConsistencyChecker;

impl ConsistencyChecker {
    pub fn new() -> Self {
        Self
    }

    /// Extract workspace root and current project slug from artifact directory.
    fn get_ws_and_project(artifact_dir: &Path) -> Option<(PathBuf, String)> {
        // Expected structure: <ws_root>/projects/<proj_slug>/artifacts/<art_id>
        let artifacts_dir = artifact_dir.parent()?;
        if artifacts_dir.file_name()?.to_str()? != "artifacts" {
            return None;
        }
        let project_dir = artifacts_dir.parent()?;
        let projects_dir = project_dir.parent()?;
        if projects_dir.file_name()?.to_str()? != "projects" {
            return None;
        }
        let ws_root = projects_dir.parent()?.to_path_buf();
        let proj_slug = project_dir.file_name()?.to_str()?.to_string();
        Some((ws_root, proj_slug))
    }

    /// Read series slug for a project.
    fn get_project_series(ws_root: &Path, proj_slug: &str) -> Option<String> {
        let proj_yaml = ws_root
            .join("projects")
            .join(proj_slug)
            .join("project.yaml");
        if !proj_yaml.is_file() {
            return None;
        }
        let content = fs::read_to_string(&proj_yaml).ok()?;
        let val: Value = serde_yaml::from_str(&content).ok()?;
        val.get("series")
            .and_then(Value::as_str)
            .map(|s| s.to_string())
    }

    /// Find all other projects in the same series.
    fn get_sibling_episodes(ws_root: &Path, current_proj: &str, series_slug: &str) -> Vec<String> {
        let projects_dir = ws_root.join("projects");
        let mut siblings = Vec::new();
        if let Ok(entries) = fs::read_dir(projects_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if let Some(slug) = p.file_name().and_then(|n| n.to_str()) {
                        if slug != current_proj {
                            if let Some(s) = Self::get_project_series(ws_root, slug) {
                                if s == series_slug {
                                    siblings.push(slug.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        siblings.sort();
        siblings
    }

    /// Extract (intro_source, outro_source) from a timeline.json file.
    fn extract_intro_outro(timeline_path: &Path) -> Option<(String, String)> {
        let content = fs::read_to_string(timeline_path).ok()?;
        let val: Value = serde_json::from_str(&content).ok()?;
        let tracks = val.get("tracks")?.as_array()?;

        // Find video track
        for t in tracks {
            if t.get("kind").and_then(Value::as_str) == Some("video") {
                let clips = t.get("clips")?.as_array()?;
                if clips.is_empty() {
                    return None;
                }
                let first_src = clips.first()?.get("source")?.as_str()?.to_string();
                let last_src = clips.last()?.get("source")?.as_str()?.to_string();
                return Some((first_src, last_src));
            }
        }
        None
    }

    /// Extract heading font family from HTML / CSS.
    fn extract_heading_font(html_path: &Path) -> Option<String> {
        let content = fs::read_to_string(html_path).ok()?;
        if let Some(caps) = H1_FONT_REGEX.captures(&content) {
            return Some(caps[1].trim().to_string());
        }
        if let Some(caps) = FONT_FAMILY_REGEX.captures(&content) {
            return Some(caps[1].trim().to_string());
        }
        None
    }

    /// Extract voice from script.md or asset metadata.
    fn extract_voice(art_dir: &Path) -> Option<String> {
        let script_path = art_dir.join("script.md");
        if script_path.is_file() {
            if let Ok(content) = fs::read_to_string(&script_path) {
                for line in content.lines() {
                    let trim = line.trim();
                    if trim.starts_with("voice:") {
                        let v = trim.trim_start_matches("voice:").trim();
                        if !v.is_empty() {
                            return Some(v.to_string());
                        }
                    }
                }
            }
        }

        // Check artifact.yaml params.voice
        let art_yaml = art_dir.join("artifact.yaml");
        if art_yaml.is_file() {
            if let Ok(content) = fs::read_to_string(&art_yaml) {
                if let Ok(val) = serde_yaml::from_str::<Value>(&content) {
                    if let Some(v) = val
                        .get("params")
                        .and_then(|p| p.get("voice"))
                        .and_then(Value::as_str)
                    {
                        return Some(v.to_string());
                    }
                }
            }
        }

        // Check sidecars in directory
        if let Ok(entries) = fs::read_dir(art_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.to_string_lossy().ends_with(".meta.yaml") {
                    if let Ok(content) = fs::read_to_string(&p) {
                        if let Ok(val) = serde_yaml::from_str::<Value>(&content) {
                            if let Some(m) = val.get("model").and_then(Value::as_str) {
                                return Some(m.to_string());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Collect all referenced profiles in a project.
    fn collect_project_reference_edges(ws_root: &Path, proj_slug: &str) -> HashSet<String> {
        let mut refs = HashSet::new();
        let arts_dir = ws_root.join("projects").join(proj_slug).join("artifacts");
        if let Ok(entries) = fs::read_dir(arts_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let yaml_path = p.join("artifact.yaml");
                    if yaml_path.is_file() {
                        if let Ok(content) = fs::read_to_string(&yaml_path) {
                            if let Ok(val) = serde_yaml::from_str::<Value>(&content) {
                                if let Some(inputs) = val.get("inputs").and_then(Value::as_array) {
                                    for inp in inputs {
                                        if let Some(obj) = inp.as_object() {
                                            if obj.get("role").and_then(Value::as_str)
                                                == Some("reference")
                                            {
                                                if let Some(r) =
                                                    obj.get("ref").and_then(Value::as_str)
                                                {
                                                    let clean = r
                                                        .strip_prefix("artifact:")
                                                        .or_else(|| r.strip_prefix("series:"))
                                                        .unwrap_or(r);
                                                    refs.insert(clean.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        refs
    }
}

impl Default for ConsistencyChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for ConsistencyChecker {
    fn id(&self) -> &'static str {
        "consistency"
    }

    fn check(&self, input: &CheckInput) -> Vec<Finding> {
        let mut findings = Vec::new();

        let Some((ws_root, current_proj)) = Self::get_ws_and_project(input.artifact_dir) else {
            return findings;
        };

        let Some(series_slug) = Self::get_project_series(&ws_root, &current_proj) else {
            return findings;
        };

        let siblings = Self::get_sibling_episodes(&ws_root, &current_proj, &series_slug);

        // 1. Check Intro / Outro for timeline artifacts
        let cur_timeline_path = input.artifact_dir.join("timeline.json");
        if cur_timeline_path.is_file() {
            if let Some((cur_intro, cur_outro)) = Self::extract_intro_outro(&cur_timeline_path) {
                for sib in &siblings {
                    let sib_artifacts_dir = ws_root.join("projects").join(sib).join("artifacts");
                    if let Ok(entries) = fs::read_dir(&sib_artifacts_dir) {
                        for entry in entries.flatten() {
                            let p = entry.path().join("timeline.json");
                            if p.is_file() {
                                if let Some((sib_intro, sib_outro)) = Self::extract_intro_outro(&p)
                                {
                                    if cur_intro != sib_intro {
                                        findings.push(Finding::warn(
                                            "SERIES_INTRO_MISMATCH",
                                            format!(
                                                "Clip mở đầu (intro) khác với tập {}: '{}' so với '{}'",
                                                sib, cur_intro, sib_intro
                                            ),
                                        ).with_location("timeline.json"));
                                    }
                                    if cur_outro != sib_outro {
                                        findings.push(Finding::warn(
                                            "SERIES_OUTRO_MISMATCH",
                                            format!(
                                                "Clip kết thúc (outro) khác với tập {}: '{}' so với '{}'",
                                                sib, cur_outro, sib_outro
                                            ),
                                        ).with_location("timeline.json"));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Check thumbnail heading font for design artifacts
        let cur_html_path = input.artifact_dir.join("index.html");
        if cur_html_path.is_file() {
            if let Some(cur_font) = Self::extract_heading_font(&cur_html_path) {
                for sib in &siblings {
                    let sib_artifacts_dir = ws_root.join("projects").join(sib).join("artifacts");
                    if let Ok(entries) = fs::read_dir(&sib_artifacts_dir) {
                        for entry in entries.flatten() {
                            let p = entry.path().join("index.html");
                            if p.is_file() {
                                if let Some(sib_font) = Self::extract_heading_font(&p) {
                                    if cur_font != sib_font {
                                        findings.push(Finding::warn(
                                            "SERIES_FONT_MISMATCH",
                                            format!(
                                                "Font tiêu đề thumbnail khác với tập {}: '{}' so với '{}'",
                                                sib, cur_font, sib_font
                                            ),
                                        ).with_location("index.html"));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Check TTS voice for audio artifacts
        if let Some(cur_voice) = Self::extract_voice(input.artifact_dir) {
            for sib in &siblings {
                let sib_artifacts_dir = ws_root.join("projects").join(sib).join("artifacts");
                if let Ok(entries) = fs::read_dir(&sib_artifacts_dir) {
                    for entry in entries.flatten() {
                        let dir = entry.path();
                        if dir.is_dir() {
                            if let Some(sib_voice) = Self::extract_voice(&dir) {
                                if cur_voice != sib_voice {
                                    findings.push(Finding::warn(
                                        "SERIES_VOICE_MISMATCH",
                                        format!(
                                            "Giọng đọc TTS khác với tập {}: '{}' so với '{}'",
                                            sib, cur_voice, sib_voice
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Check presence of series references
        let series_yaml_path = ws_root
            .join("series")
            .join(&series_slug)
            .join("series.yaml");
        if series_yaml_path.is_file() {
            if let Ok(content) = fs::read_to_string(&series_yaml_path) {
                if let Ok(series_val) = serde_yaml::from_str::<Value>(&content) {
                    if let Some(req_refs) = series_val.get("references").and_then(Value::as_array) {
                        let project_refs =
                            Self::collect_project_reference_edges(&ws_root, &current_proj);
                        for r_val in req_refs {
                            if let Some(r_str) = r_val.as_str() {
                                if !project_refs.contains(r_str) {
                                    findings.push(Finding::warn(
                                        "SERIES_REFERENCE_MISSING",
                                        format!(
                                            "Dự án chưa gắn hồ sơ tham chiếu '{}' được khai báo trong series {}",
                                            r_str, series_slug
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        findings
    }
}
