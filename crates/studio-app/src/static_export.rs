use sha2::{Digest, Sha256};
use studio_core::{
    document::{Artboard, CreativeDocument, ElementType},
    error::StudioError,
    preview::{ExportReceipt, ExportedFileItem, PreviewFidelity, PreviewReceipt},
};

pub struct StaticExportEngine;

impl StaticExportEngine {
    pub const RENDERER_ID: &'static str = "studio-canvas-exporter";
    pub const RENDERER_VERSION: &'static str = "1.0.0";

    /// Generates a PreviewReceipt verifying fidelity and state:
    /// - Saved revisions get EXACT fidelity
    /// - Unsaved changes or draft flags get ESTIMATE fidelity and is_draft = true
    pub fn generate_preview_receipt(
        doc: &CreativeDocument,
        revision_id: &str,
        is_draft: bool,
    ) -> PreviewReceipt {
        let source_hash = doc.compute_hash();

        let mut warnings = Vec::new();
        if is_draft {
            warnings.push("Bản xem trước DRAFT chưa được lưu thành revision chính thức.".into());
        }

        let mut hasher = Sha256::new();
        hasher.update(source_hash.as_bytes());
        hasher.update(revision_id.as_bytes());
        let output_hash = hex::encode(hasher.finalize());

        PreviewReceipt {
            revision_id: revision_id.to_string(),
            source_hash,
            renderer_id: Self::RENDERER_ID.to_string(),
            renderer_version: Self::RENDERER_VERSION.to_string(),
            fidelity: if is_draft {
                PreviewFidelity::Estimate
            } else {
                PreviewFidelity::Exact
            },
            is_draft,
            generated_at: chrono::Utc::now(),
            output_hash,
            warnings,
        }
    }

    /// Renders SVG/PNG markup per artboard deterministically
    pub fn export_artboards_svg(
        project_id: &str,
        revision_id: &str,
        doc: &CreativeDocument,
    ) -> Result<ExportReceipt, StudioError> {
        let source_hash = doc.compute_hash();
        let mut files = Vec::new();
        let mut total_bytes: u64 = 0;

        for (idx, board) in doc.artboards.iter().enumerate() {
            let svg_content = Self::render_artboard_to_svg(board);
            let bytes = svg_content.into_bytes();
            let size = bytes.len() as u64;
            total_bytes += size;

            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let sha256 = hex::encode(hasher.finalize());

            files.push(ExportedFileItem {
                artboard_id: Some(board.id.clone()),
                file_name: format!("artboard-{:02}.svg", idx + 1),
                mime_type: "image/svg+xml".into(),
                sha256,
                size_bytes: size,
            });
        }

        Ok(ExportReceipt {
            project_id: project_id.to_string(),
            revision_id: revision_id.to_string(),
            source_hash,
            export_format: "svg".into(),
            files,
            total_bytes,
            generated_at: chrono::Utc::now(),
        })
    }

    /// Renders standalone, sandboxed static HTML document
    pub fn export_static_html(
        project_id: &str,
        revision_id: &str,
        doc: &CreativeDocument,
    ) -> Result<ExportReceipt, StudioError> {
        let source_hash = doc.compute_hash();

        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html lang=\"vi\">\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
        // CSP sandbox security restriction: no script, no external connect
        html.push_str("<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; img-src data:;\">\n");
        html.push_str("<title>Local Content Studio — Preview</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { margin: 0; padding: 24px; background: #0f172a; color: #f8fafc; font-family: sans-serif; display: flex; flex-direction: column; align-items: center; gap: 32px; }\n");
        html.push_str(".artboard { width: 540px; height: 675px; background: #ffffff; color: #000000; box-shadow: 0 10px 25px rgba(0,0,0,0.5); position: relative; border-radius: 8px; overflow: hidden; }\n");
        html.push_str(".element { position: absolute; box-sizing: border-box; }\n");
        html.push_str("</style>\n</head>\n<body>\n");

        for board in &doc.artboards {
            html.push_str(&format!("<div class=\"artboard\" id=\"{}\">\n", board.id));
            for el in &board.elements {
                let scale = 540.0 / 1080.0;
                let x = el.box_model.x * scale;
                let y = el.box_model.y * scale;
                let w = el.box_model.w * scale;
                let h = el.box_model.h * scale;

                let content = match el.element_type {
                    ElementType::Text => Self::escape_xml(el.text.as_deref().unwrap_or("")),
                    ElementType::Image => "[Hình ảnh]".into(),
                    _ => "".into(),
                };

                html.push_str(&format!(
                    "  <div class=\"element\" style=\"left: {:.1}px; top: {:.1}px; width: {:.1}px; height: {:.1}px;\">{}</div>\n",
                    x, y, w, h, content
                ));
            }
            html.push_str("</div>\n");
        }

        html.push_str("</body>\n</html>\n");

        let bytes = html.into_bytes();
        let size = bytes.len() as u64;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let sha256 = hex::encode(hasher.finalize());

        let file = ExportedFileItem {
            artboard_id: None,
            file_name: "index.html".into(),
            mime_type: "text/html; charset=utf-8".into(),
            sha256,
            size_bytes: size,
        };

        Ok(ExportReceipt {
            project_id: project_id.to_string(),
            revision_id: revision_id.to_string(),
            source_hash,
            export_format: "html".into(),
            files: vec![file],
            total_bytes: size,
            generated_at: chrono::Utc::now(),
        })
    }

    fn render_artboard_to_svg(board: &Artboard) -> String {
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
            board.width, board.height, board.width, board.height
        );
        svg.push_str("  <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");

        for el in &board.elements {
            if !el.visible {
                continue;
            }
            match el.element_type {
                ElementType::Text => {
                    let text = Self::escape_xml(el.text.as_deref().unwrap_or(""));
                    let font_size = el.style.font_size.unwrap_or(24);
                    svg.push_str(&format!(
                        "  <text x=\"{}\" y=\"{}\" font-size=\"{}\" font-family=\"sans-serif\" fill=\"#0f172a\">{}</text>\n",
                        el.box_model.x, el.box_model.y + font_size as f64, font_size, text
                    ));
                }
                ElementType::Image => {
                    svg.push_str(&format!(
                        "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#f1f5f9\" stroke=\"#cbd5e1\"/>\n",
                        el.box_model.x, el.box_model.y, el.box_model.w, el.box_model.h
                    ));
                }
                _ => {}
            }
        }

        svg.push_str("</svg>\n");
        svg
    }

    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}
