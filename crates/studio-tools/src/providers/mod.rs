//! Builtin AI generator providers: Google Imagen, OpenAI Images, and Google Veo.

pub mod http;
pub mod imagen;
pub mod import;
pub mod openai;
pub mod openai_videos;
pub mod veo;

use crate::error::{Result, ToolError};
use serde_json::Value;
use std::path::Path;
use studio_workspace::Workspace;

/// Run a builtin provider in-process by identifier.
pub async fn run_builtin(
    tool_id: &str,
    ws: &Workspace,
    out_dir: &Path,
    input: &Value,
    api_key: &str,
    base_override: Option<&str>,
    pricing: Option<&crate::pricing::ProviderPricing>,
) -> Result<Value> {
    match tool_id {
        "media.import.yt-dlp" => import::run_media_import(ws, out_dir, input).await,
        "google.imagen" => {
            imagen::run_google_imagen(ws, out_dir, input, api_key, base_override, pricing).await
        }
        "openai.images" => {
            openai::run_openai_images(ws, out_dir, input, api_key, base_override, pricing).await
        }
        "openai.videos" => {
            openai_videos::run_openai_videos(ws, out_dir, input, api_key, base_override, pricing)
                .await
        }
        "google.veo" => {
            veo::run_google_veo(ws, out_dir, input, api_key, base_override, pricing).await
        }
        other => Err(ToolError::ToolNotFound(format!(
            "Không hỗ trợ builtin tool: {other}"
        ))),
    }
}
