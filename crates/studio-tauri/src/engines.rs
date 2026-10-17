//! Engine model listing for the model picker.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use studio_engine::ModelInfo;
use tauri::State;

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;

/// `agy models` hits the network: keep the answer for a while.
const AGY_MODELS_TTL: Duration = Duration::from_secs(600);
static AGY_MODELS: Mutex<Option<(Instant, Vec<ModelInfo>)>> = Mutex::new(None);

/// Models the given engine accepts; the first entry is the suggested default.
#[tauri::command]
pub async fn engine_models(state: State<'_, AppState>, engine: String) -> CmdResult<Vec<Value>> {
    let models = match engine.as_str() {
        "claude" => state.claude.list_models(),
        "agy" => {
            let cached = AGY_MODELS
                .lock()
                .ok()
                .and_then(|c| c.clone())
                .filter(|(at, _)| at.elapsed() < AGY_MODELS_TTL)
                .map(|(_, m)| m);
            match cached {
                Some(m) => m,
                None => {
                    let fresh = state.agy.list_models().await.map_err(|e| {
                        CmdError::msg(format!("Không lấy được danh sách model của agy: {e}"))
                    })?;
                    if let Ok(mut c) = AGY_MODELS.lock() {
                        *c = Some((Instant::now(), fresh.clone()));
                    }
                    fresh
                }
            }
        }
        other => return Err(CmdError::msg(format!("Engine không hỗ trợ: {other}"))),
    };
    Ok(models
        .into_iter()
        .map(|m| json!({ "value": m.id, "label": m.label }))
        .collect())
}
