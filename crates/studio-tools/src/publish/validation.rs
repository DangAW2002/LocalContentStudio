//! Validation of publish requests against Channel@1 limits and render requirements.

use crate::error::Result;
use crate::publish::model::PublishMetadata;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::PathBuf;
use studio_brand::checkers::{Finding, Severity};
use studio_brand::loader::BrandSet;
use studio_brand::schema::ChannelSpec;
use studio_workspace::{Project, Workspace};

/// Render record loaded from workspace SQLite DB.
#[derive(Debug, Clone)]
pub struct NewestRender {
    pub id: String,
    pub output_path: String,
    pub format: String,
    pub size_bytes: u64,
}

/// Result of validating publish metadata and render state.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Findings produced during validation.
    pub findings: Vec<Finding>,
    /// Resolved absolute/relative files to be published.
    pub files: Vec<String>,
    /// Whether any finding has Error severity.
    pub has_blocking_errors: bool,
}

/// Find newest render for artifact.
pub fn find_newest_render(
    conn: &Connection,
    ws: &Workspace,
    project_slug: &str,
    artifact_id: &str,
) -> Option<NewestRender> {
    // 1. Check SQLite renders table
    let mut stmt = conn
        .prepare(
            "SELECT id, output_path, format, size_bytes
             FROM renders WHERE artifact_id = ?1
             ORDER BY created_at DESC LIMIT 1",
        )
        .ok()?;

    let db_res: Option<NewestRender> = stmt
        .query_row([artifact_id], |row| {
            let size: i64 = row.get(3)?;
            Ok(NewestRender {
                id: row.get(0)?,
                output_path: row.get(1)?,
                format: row.get(2)?,
                size_bytes: size.max(0) as u64,
            })
        })
        .ok();

    if let Some(r) = db_res {
        return Some(r);
    }

    // 2. Fallback: check project's renders folder
    let proj = Project::open(ws, project_slug).ok()?;
    let renders_dir = proj.renders_dir();
    if let Ok(entries) = std::fs::read_dir(&renders_dir) {
        let mut candidates: Vec<(PathBuf, std::time::SystemTime, u64)> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(meta) = path.metadata() {
                    let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    candidates.push((path, mtime, meta.len()));
                }
            }
        }
        candidates.sort_by_key(|b| std::cmp::Reverse(b.1));
        if let Some((best_path, _, size)) = candidates.into_iter().next() {
            let ext = best_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("bin")
                .to_lowercase();
            let rel = best_path
                .strip_prefix(ws.root())
                .unwrap_or(&best_path)
                .to_string_lossy()
                .replace('\\', "/");
            return Some(NewestRender {
                id: format!("disk-{}", uuid::Uuid::new_v4().simple()),
                output_path: rel,
                format: ext,
                size_bytes: size,
            });
        }
    }

    None
}

/// Count total unique hashtags across tags and body text.
pub fn count_hashtags(metadata: &PublishMetadata) -> usize {
    let mut tags = HashSet::new();

    for t in &metadata.tags {
        let clean = t.trim().trim_start_matches('#').to_lowercase();
        if !clean.is_empty() {
            tags.insert(clean);
        }
    }

    let extract_from = |text: &str, set: &mut HashSet<String>| {
        for word in text.split_whitespace() {
            if let Some(tag) = word.strip_prefix('#') {
                let clean: String = tag
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !clean.is_empty() {
                    set.insert(clean.to_lowercase());
                }
            }
        }
    };

    extract_from(&metadata.title, &mut tags);
    extract_from(&metadata.description, &mut tags);

    tags.len()
}

/// Validate publish request against ChannelSpec and available render files.
pub fn validate_publish(
    ws: &Workspace,
    conn: &Connection,
    project_slug: &str,
    artifact_id: &str,
    channel_id: &str,
    metadata: &PublishMetadata,
) -> Result<ValidationResult> {
    let mut findings = Vec::new();
    let mut files = Vec::new();

    // 1. Load channel spec if available
    let brand_set = BrandSet::load(ws).ok();
    let channel: Option<&ChannelSpec> =
        brand_set.as_ref().and_then(|bs| bs.get_channel(channel_id));

    // 2. Validate metadata against text limits
    if let Some(ch) = channel {
        let title_chars = metadata.title.chars().count();
        if let Some(max_title) = ch.text.title_max {
            if title_chars > max_title {
                findings.push(Finding {
                    severity: Severity::Error,
                    code: "TITLE_TOO_LONG".into(),
                    message_vi: format!(
                        "Tiêu đề ({title_chars} ký tự) vượt quá giới hạn tối đa ({max_title} ký tự) của kênh '{}'.",
                        ch.title
                    ),
                    location: Some("title".into()),
                    fix_hint_vi: Some(format!("Rút ngắn tiêu đề còn dưới {max_title} ký tự.")),
                });
            }
        }

        let desc_chars = metadata.description.chars().count();
        if let Some(max_desc) = ch.text.description_max {
            if desc_chars > max_desc {
                findings.push(Finding {
                    severity: Severity::Error,
                    code: "DESCRIPTION_TOO_LONG".into(),
                    message_vi: format!(
                        "Mô tả ({desc_chars} ký tự) vượt quá giới hạn tối đa ({max_desc} ký tự) của kênh '{}'.",
                        ch.title
                    ),
                    location: Some("description".into()),
                    fix_hint_vi: Some(format!("Rút ngắn phần mô tả còn dưới {max_desc} ký tự.")),
                });
            }
        }

        let hashtag_count = count_hashtags(metadata);
        if let Some(max_tags) = ch.text.hashtags_max {
            if hashtag_count > max_tags {
                findings.push(Finding {
                    severity: Severity::Error,
                    code: "HASHTAGS_TOO_MANY".into(),
                    message_vi: format!(
                        "Số lượng hashtag ({hashtag_count}) vượt quá giới hạn tối đa ({max_tags}) của kênh '{}'.",
                        ch.title
                    ),
                    location: Some("tags".into()),
                    fix_hint_vi: Some(format!("Giảm bớt hashtag còn tối đa {max_tags} thẻ.")),
                });
            }
        }
    }

    // 3. Find newest render
    let render_opt = find_newest_render(conn, ws, project_slug, artifact_id);
    match render_opt {
        None => {
            findings.push(Finding {
                severity: Severity::Error,
                code: "NO_RENDERS".into(),
                message_vi: "Chưa có bản render nào cho artifact. Hãy render artifact trước khi chuẩn bị bản đăng.".into(),
                location: Some(format!("artifacts/{artifact_id}")),
                fix_hint_vi: Some("Thực hiện render artifact ra tệp đầu ra (PNG, MP4, ...) trước rồi chuẩn bị lại.".into()),
            });
        }
        Some(render) => {
            files.push(render.output_path.clone());

            if let Some(ch) = channel {
                // Check allowed containers
                if !ch.file.containers.is_empty() {
                    let fmt_norm = render.format.trim().trim_start_matches('.').to_lowercase();
                    let matches_container = ch
                        .file
                        .containers
                        .iter()
                        .any(|c| c.trim().trim_start_matches('.').to_lowercase() == fmt_norm);
                    if !matches_container {
                        findings.push(Finding {
                            severity: Severity::Error,
                            code: "INVALID_CONTAINER".into(),
                            message_vi: format!(
                                "Định dạng bản render \"{}\" không nằm trong danh sách định dạng được hỗ trợ bởi kênh '{}' ({:?}).",
                                render.format, ch.title, ch.file.containers
                            ),
                            location: Some(render.output_path.clone()),
                            fix_hint_vi: Some(format!("Render lại artifact dưới định dạng: {:?}", ch.file.containers)),
                        });
                    }
                }

                // Check file size cap
                if let Some(max_mb) = ch.file.max_size_mb {
                    let size_mb = (render.size_bytes as f64) / (1024.0 * 1024.0);
                    if size_mb > max_mb as f64 {
                        findings.push(Finding {
                            severity: Severity::Error,
                            code: "FILE_TOO_LARGE".into(),
                            message_vi: format!(
                                "Dung lượng bản render ({:.1} MB) vượt quá giới hạn tối đa ({} MB) của kênh '{}'.",
                                size_mb, max_mb, ch.title
                            ),
                            location: Some(render.output_path.clone()),
                            fix_hint_vi: Some("Giảm độ phân giải hoặc nén tệp đầu ra.".into()),
                        });
                    }
                }
            }
        }
    }

    // 4. Validate thumbnail if provided
    if let Some(ref thumb) = metadata.thumbnail {
        // The thumbnail may come from the agent: accept workspace files only.
        let thumb_path = ws.scoped_path(thumb).ok();

        if let Some(thumb_path) = thumb_path.filter(|p| p.is_file()) {
            let rel = thumb_path
                .strip_prefix(ws.root())
                .unwrap_or(&thumb_path)
                .to_string_lossy()
                .replace('\\', "/");
            files.push(rel);
        } else {
            findings.push(Finding {
                severity: Severity::Warn,
                code: "THUMBNAIL_NOT_FOUND".into(),
                message_vi: format!("Không tìm thấy tệp ảnh thu nhỏ tại: \"{thumb}\"."),
                location: Some("thumbnail".into()),
                fix_hint_vi: Some(
                    "Kiểm tra lại đường dẫn ảnh thu nhỏ hoặc bỏ trống nếu không sử dụng.".into(),
                ),
            });
        }
    }

    let has_blocking_errors = findings.iter().any(|f| f.severity == Severity::Error);

    Ok(ValidationResult {
        findings,
        files,
        has_blocking_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publish::model::PublishPrivacy;

    #[test]
    fn test_count_hashtags() {
        let meta = PublishMetadata {
            title: "Video review #tech #studio".into(),
            description: "Xem chi tiết tại link dưới #review #tech".into(),
            tags: vec!["#studio".into(), "gadget".into()],
            privacy: PublishPrivacy::Private,
            schedule: None,
            thumbnail: None,
        };

        // Unique hashtags: tech, studio, review, gadget -> 4
        let count = count_hashtags(&meta);
        assert_eq!(count, 4);
    }
}
