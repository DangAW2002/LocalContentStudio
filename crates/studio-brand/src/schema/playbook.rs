use crate::error::{BrandError, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Playbook step in markdown frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlaybookStep {
    /// Step identifier or sequence name
    pub id: String,
    /// Human-readable step name
    pub name: String,
    /// Capabilities or tools involved
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Brief description
    #[serde(default)]
    pub description: String,
}

/// Frontmatter metadata of a playbook (`SKILL.md` style).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlaybookFrontmatter {
    /// Playbook name
    pub name: String,
    /// Description of process
    pub description: String,
    /// Kind discriminator (always "playbook")
    #[serde(default = "default_playbook_kind")]
    pub kind: String,
    /// Target artifacts produced (e.g. `["video", "thumbnail"]`)
    #[serde(default)]
    pub artifacts: Vec<String>,
    /// Sequential or suggested steps
    #[serde(default)]
    pub steps: Vec<PlaybookStep>,
    /// Checkers required to validate artifacts (by id)
    #[serde(default)]
    pub checkers: Vec<String>,
    /// Points where agent must ask the user
    #[serde(default)]
    pub ask_user_at: Vec<String>,
    /// Recipes referenced or pinned
    #[serde(default)]
    pub recipes: Vec<String>,
}

fn default_playbook_kind() -> String {
    "playbook".to_string()
}

/// Parsed playbook containing frontmatter and markdown body instructions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playbook {
    /// Playbook unique ID (usually filename stem)
    pub id: String,
    /// Parsed frontmatter
    pub frontmatter: PlaybookFrontmatter,
    /// Markdown body instructions for agent
    pub body: String,
}

impl Playbook {
    /// Parse markdown with YAML frontmatter `--- ... ---`.
    pub fn parse(id: &str, content: &str) -> Result<Self> {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return Err(BrandError::PlaybookParse(
                "Missing opening frontmatter delimiter `---`".to_string(),
            ));
        }

        let after_first = &trimmed[3..];
        let end_idx = after_first.find("\n---").ok_or_else(|| {
            BrandError::PlaybookParse("Missing closing frontmatter delimiter `---`".to_string())
        })?;

        let yaml_str = &after_first[..end_idx];
        let body_start = end_idx + 4; // skip \n---
        let body = if body_start < after_first.len() {
            after_first[body_start..]
                .trim_start_matches(['\r', '\n'])
                .to_string()
        } else {
            String::new()
        };

        let frontmatter: PlaybookFrontmatter = serde_yaml::from_str(yaml_str).map_err(|e| {
            BrandError::PlaybookParse(format!("Failed to parse YAML frontmatter: {e}"))
        })?;

        Ok(Self {
            id: id.to_string(),
            frontmatter,
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playbook_parse() {
        let md = r#"---
name: "Shorts Video"
description: "Quy trình làm video ngắn"
kind: playbook
artifacts:
  - "video"
  - "thumbnail"
steps:
  - id: "script"
    name: "Kịch bản"
    capabilities: ["llm.generate@1"]
  - id: "voice"
    name: "Thu âm AI"
    capabilities: ["tts.synthesize@1"]
checkers:
  - "dimensions"
  - "duration"
  - "safe_zone"
ask_user_at:
  - "Duyệt kịch bản trước khi gen voice"
recipes:
  - "reel-export"
---
# Hướng dẫn chi tiết
1. Tạo kịch bản ngắn dưới 60s
2. Dùng TTS tạo giọng đọc
"#;

        let pb = Playbook::parse("shorts", md).expect("parse playbook");
        assert_eq!(pb.id, "shorts");
        assert_eq!(pb.frontmatter.name, "Shorts Video");
        assert_eq!(pb.frontmatter.checkers.len(), 3);
        assert_eq!(pb.frontmatter.steps.len(), 2);
        assert!(pb.body.contains("Hướng dẫn chi tiết"));
    }
}
