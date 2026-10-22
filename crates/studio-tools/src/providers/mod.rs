//! Builtin AI generator providers: Google Imagen, OpenAI Images, and Google Veo.

pub mod http;
pub mod imagen;
pub mod import;
pub mod inspect;
pub mod openai;
pub mod openai_videos;
pub mod piper;
pub mod resize;
pub mod veo;
pub mod vieneu;

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
        "media.inspect" => inspect::run_media_inspect(ws, out_dir, input).await,
        "media.import.yt-dlp" => import::run_media_import(ws, out_dir, input).await,
        "tts.vieneu" => vieneu::run_vieneu(ws, out_dir, input).await,
        "tts.piper" => piper::run_piper(ws, out_dir, input).await,
        "google.imagen" => {
            let mut res =
                imagen::run_google_imagen(ws, out_dir, input, api_key, base_override, pricing)
                    .await?;
            resize::post_process_image_output(ws, out_dir, input, &mut res).await?;
            Ok(res)
        }
        "openai.images" => {
            let mut res =
                openai::run_openai_images(ws, out_dir, input, api_key, base_override, pricing)
                    .await?;
            resize::post_process_image_output(ws, out_dir, input, &mut res).await?;
            Ok(res)
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
