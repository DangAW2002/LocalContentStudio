use serde::{Deserialize, Serialize};

use crate::schema::{BrandSpec, ChannelSpec, PlaybookFrontmatter};

/// A single validation error with line, column and friendly Vietnamese message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationErrorItem {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

/// Validation result containing validity status and error list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationErrorItem>,
}

/// Translate serde_yaml error to friendly Vietnamese message.
pub fn format_yaml_error_vi(e: &serde_yaml::Error) -> String {
    let raw = e.to_string();

    // Check for common serde_yaml error patterns
    if raw.contains("missing field `") {
        if let Some(start) = raw.find("missing field `") {
            let rest = &raw[start + 15..];
            if let Some(end) = rest.find('`') {
                let field = &rest[..end];
                return format!("Thiếu trường bắt buộc `{field}`.");
            }
        }
    }

    if raw.contains("unknown field `") {
        if let Some(start) = raw.find("unknown field `") {
            let rest = &raw[start + 15..];
            if let Some(end) = rest.find('`') {
                let field = &rest[..end];
                return format!("Trường không xác định `{field}`.");
            }
        }
    }

    if raw.contains("invalid type:") {
        return format!("Kiểu dữ liệu không đúng định dạng ({raw}).");
    }

    if raw.contains("did not find expected") || raw.contains("while parsing") {
        return format!("Lỗi cú pháp định dạng YAML: {raw}");
    }

    format!("Lỗi cấu trúc: {raw}")
}

/// Validate YAML/markdown text for Brand, Channel, or Playbook schemas.
pub fn validate_scope_item(kind: &str, text: &str) -> ValidationResult {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ValidationResult {
            valid: false,
            errors: vec![ValidationErrorItem {
                line: 1,
                column: 1,
                message: "Nội dung tệp cấu hình không được để trống.".to_string(),
            }],
        };
    }

    match kind {
        "brands" | "brand" => match serde_yaml::from_str::<BrandSpec>(text) {
            Ok(_) => ValidationResult {
                valid: true,
                errors: vec![],
            },
            Err(e) => {
                let (line, column) = e
                    .location()
                    .map(|l| (l.line(), l.column()))
                    .unwrap_or((1, 1));
                ValidationResult {
                    valid: false,
                    errors: vec![ValidationErrorItem {
                        line,
                        column,
                        message: format_yaml_error_vi(&e),
                    }],
                }
            }
        },
        "channels" | "channel" => match serde_yaml::from_str::<ChannelSpec>(text) {
            Ok(_) => ValidationResult {
                valid: true,
                errors: vec![],
            },
            Err(e) => {
                let (line, column) = e
                    .location()
                    .map(|l| (l.line(), l.column()))
                    .unwrap_or((1, 1));
                ValidationResult {
                    valid: false,
                    errors: vec![ValidationErrorItem {
                        line,
                        column,
                        message: format_yaml_error_vi(&e),
                    }],
                }
            }
        },
        "playbooks" | "playbook" => {
            let trimmed_start = text.trim_start();
            if !trimmed_start.starts_with("---") {
                return ValidationResult {
                    valid: false,
                    errors: vec![ValidationErrorItem {
                        line: 1,
                        column: 1,
                        message: "Thiếu dấu phân cách mở đầu `---` cho phần frontmatter."
                            .to_string(),
                    }],
                };
            }

            let leading_newlines = text[..text.len() - trimmed_start.len()]
                .matches('\n')
                .count();
            let start_line = 1 + leading_newlines;

            let after_first = &trimmed_start[3..];
            let Some(end_idx) = after_first.find("\n---") else {
                return ValidationResult {
                    valid: false,
                    errors: vec![ValidationErrorItem {
                        line: start_line,
                        column: 1,
                        message: "Thiếu dấu phân cách kết thúc `---` cho phần frontmatter."
                            .to_string(),
                    }],
                };
            };

            let yaml_str = &after_first[..end_idx];
            match serde_yaml::from_str::<PlaybookFrontmatter>(yaml_str) {
                Ok(_) => ValidationResult {
                    valid: true,
                    errors: vec![],
                },
                Err(e) => {
                    let (rel_line, column) = e
                        .location()
                        .map(|l| (l.line(), l.column()))
                        .unwrap_or((1, 1));
                    let line = start_line + rel_line;
                    ValidationResult {
                        valid: false,
                        errors: vec![ValidationErrorItem {
                            line,
                            column,
                            message: format_yaml_error_vi(&e),
                        }],
                    }
                }
            }
        }
        other => ValidationResult {
            valid: false,
            errors: vec![ValidationErrorItem {
                line: 1,
                column: 1,
                message: format!("Loại cấu hình không hợp lệ: '{other}'."),
            }],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_brand() {
        let valid_yaml = r##"
name: "Kênh Test"
slogan: "Sáng tạo"
colors:
  primary: "#1E88E5"
  accent: "#FFB300"
  bg: "#000000"
  text: "#FFFFFF"
fonts:
  heading: "Inter"
  body: "Inter"
logos:
  full: "logo.svg"
  mark: "mark.svg"
"##;
        let res = validate_scope_item("brands", valid_yaml);
        assert!(res.valid);
        assert!(res.errors.is_empty());
    }

    #[test]
    fn test_validate_invalid_brand_returns_line_column() {
        let invalid_yaml = r##"
name: "Kênh Test"
slogan: "Sáng tạo"
colors:
  primary: "#1E88E5"
  accent: "#FFB300"
  # Missing bg and text!
fonts:
  heading: "Inter"
"##;
        let res = validate_scope_item("brands", invalid_yaml);
        assert!(!res.valid);
        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].line >= 4);
        assert!(res.errors[0].message.contains("Thiếu trường bắt buộc"));
    }

    #[test]
    fn test_validate_channel() {
        let channel_yaml = r#"
id: "tiktok"
platform: "tiktok"
title: "TikTok Feed"
formats:
  - name: "vertical"
    width: 1080
    height: 1920
    aspect: "9:16"
"#;
        let res = validate_scope_item("channels", channel_yaml);
        assert!(res.valid);

        let bad_channel = "invalid: yaml: [broken";
        let res_bad = validate_scope_item("channels", bad_channel);
        assert!(!res_bad.valid);
        assert_eq!(res_bad.errors[0].line, 1);
    }

    #[test]
    fn test_validate_playbook() {
        let playbook_md = r#"---
name: "Shorts Video"
description: "Làm video"
---
# Nội dung hướng dẫn
"#;
        let res = validate_scope_item("playbooks", playbook_md);
        assert!(res.valid);

        let missing_delimiter = "# Không có frontmatter";
        let res_bad = validate_scope_item("playbooks", missing_delimiter);
        assert!(!res_bad.valid);
        assert_eq!(res_bad.errors[0].line, 1);
        assert!(res_bad.errors[0]
            .message
            .contains("Thiếu dấu phân cách mở đầu"));

        let bad_fm = r#"---
name: 12345
description: true
kind: [invalid]
---
# Body
"#;
        let res_bad_fm = validate_scope_item("playbooks", bad_fm);
        assert!(!res_bad_fm.valid);
        assert!(res_bad_fm.errors[0].line >= 2);
    }
}
