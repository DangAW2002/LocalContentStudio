//! Budget queries and management for workspace daily and monthly limits.

use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use studio_workspace::{Workspace, WorkspaceDb};
use tauri::State;

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;

/// Default per-turn budget when workspace has no custom limit.
pub const DEFAULT_TURN_BUDGET_USD: f64 = 1.0;

/// Summary of workspace budget and current spend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetStatus {
    /// Spent today in USD
    pub today_spend_usd: f64,
    /// Daily budget in USD if configured
    pub daily_budget_usd: Option<f64>,
    /// Spent this month in USD
    pub month_spend_usd: f64,
    /// Monthly budget in USD if configured
    pub monthly_budget_usd: Option<f64>,
    /// Warning threshold ratio (e.g. 0.8 = 80%)
    pub warning_threshold: f64,
    /// Whether daily budget is exceeded
    pub is_daily_over: bool,
    /// Whether monthly budget is exceeded
    pub is_monthly_over: bool,
}

/// Query current spend and budget status for the workspace.
pub fn get_budget_status(ws: &Workspace) -> CmdResult<BudgetStatus> {
    let db = WorkspaceDb::open(ws)?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let month = chrono::Utc::now().format("%Y-%m").to_string();

    let today_spend: f64 = db.conn().query_row(
        "SELECT COALESCE(SUM(cost_usd), 0.0) FROM cost_ledger WHERE substr(created_at, 1, 10) = ?1",
        [today],
        |r| r.get(0),
    )?;

    let month_spend: f64 = db.conn().query_row(
        "SELECT COALESCE(SUM(cost_usd), 0.0) FROM cost_ledger WHERE substr(created_at, 1, 7) = ?1",
        [month],
        |r| r.get(0),
    )?;

    let meta = ws.meta().ok();
    let daily_budget = meta.as_ref().and_then(|m| m.daily_budget_usd);
    let monthly_budget = meta.as_ref().and_then(|m| m.monthly_budget_usd);

    let is_daily_over = daily_budget.map(|d| today_spend >= d).unwrap_or(false);
    let is_monthly_over = monthly_budget.map(|m| month_spend >= m).unwrap_or(false);

    Ok(BudgetStatus {
        today_spend_usd: today_spend,
        daily_budget_usd: daily_budget,
        month_spend_usd: month_spend,
        monthly_budget_usd: monthly_budget,
        warning_threshold: 0.8,
        is_daily_over,
        is_monthly_over,
    })
}

/// Calculate remaining turn budget for paid tools given daily/monthly limits.
pub fn calculate_turn_budget(ws: &Workspace) -> Option<f64> {
    let status = get_budget_status(ws).ok()?;
    if status.is_daily_over || status.is_monthly_over {
        // Budget exhausted: 0.0 ensures paid tools trigger approval
        return Some(0.0);
    }

    let mut remaining = DEFAULT_TURN_BUDGET_USD;
    if let Some(daily) = status.daily_budget_usd {
        let left_today = (daily - status.today_spend_usd).max(0.0);
        remaining = remaining.min(left_today);
    }
    if let Some(monthly) = status.monthly_budget_usd {
        let left_month = (monthly - status.month_spend_usd).max(0.0);
        remaining = remaining.min(left_month);
    }

    Some(remaining)
}

/// Read budget settings implementation.
pub fn budget_get_impl(state: &AppState) -> CmdResult<BudgetStatus> {
    let ws = state.workspace()?;
    get_budget_status(&ws)
}

/// Update daily and monthly budget in studio.yaml.
pub fn budget_set_impl(
    state: &AppState,
    daily_budget_usd: Option<f64>,
    monthly_budget_usd: Option<f64>,
) -> CmdResult<BudgetStatus> {
    let ws = state.workspace()?;
    write_budget_to_studio_yaml(&ws, daily_budget_usd, monthly_budget_usd)?;
    get_budget_status(&ws)
}

/// Sets the budget keys in `studio.yaml`, keeping every other key.
fn write_budget_to_studio_yaml(
    ws: &Workspace,
    daily_budget_usd: Option<f64>,
    monthly_budget_usd: Option<f64>,
) -> CmdResult<()> {
    let studio_yaml_path = ws.root().join("studio.yaml");

    // Edit studio.yaml as a YAML mapping so keys this struct does not know are kept, and refuse
    // to touch a file that does not parse instead of replacing it with defaults.
    let mut doc: serde_yaml::Mapping = if studio_yaml_path.exists() {
        let content = std::fs::read_to_string(&studio_yaml_path)?;
        match serde_yaml::from_str::<serde_yaml::Value>(&content) {
            Ok(serde_yaml::Value::Mapping(m)) => m,
            Ok(serde_yaml::Value::Null) => serde_yaml::Mapping::new(),
            Ok(_) | Err(_) => {
                return Err(CmdError::msg(
                    "studio.yaml không hợp lệ — sửa tệp trước rồi đặt lại ngân sách.",
                ))
            }
        }
    } else {
        let mut m = serde_yaml::Mapping::new();
        m.insert("name".into(), ws.name().into());
        m.insert("version".into(), 1.into());
        m
    };

    for (key, alias, value) in [
        ("daily_budget_usd", "budget_day_usd", daily_budget_usd),
        ("monthly_budget_usd", "budget_month_usd", monthly_budget_usd),
    ] {
        doc.remove(alias);
        match value {
            Some(v) => {
                doc.insert(key.into(), v.into());
            }
            None => {
                doc.remove(key);
            }
        }
    }

    let yaml = serde_yaml::to_string(&doc)
        .map_err(|e| CmdError::msg(format!("Không thể chuyển đổi cấu hình YAML: {e}")))?;
    std::fs::write(&studio_yaml_path, yaml)?;
    Ok(())
}

#[tauri::command]
pub fn budget_get(state: State<'_, AppState>) -> CmdResult<BudgetStatus> {
    budget_get_impl(&state)
}

#[tauri::command]
pub fn budget_set(
    state: State<'_, AppState>,
    daily_budget_usd: Option<f64>,
    monthly_budget_usd: Option<f64>,
) -> CmdResult<BudgetStatus> {
    budget_set_impl(&state, daily_budget_usd, monthly_budget_usd)
}

/// Switch model of an existing thread mid-thread.
pub fn thread_set_model_impl(
    state: &AppState,
    thread_id: String,
    model: Option<String>,
) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;

    let affected = db.conn().execute(
        "UPDATE threads SET model = ?2, updated_at = ?3 WHERE id = ?1",
        params![thread_id, model, chrono::Utc::now().to_rfc3339()],
    )?;

    if affected == 0 {
        return Err(CmdError::msg("Không tìm thấy hội thoại để cập nhật model."));
    }

    state.emit(
        "thread-updated",
        json!({
            "threadId": thread_id,
            "model": model,
        }),
    );

    Ok(json!({ "success": true, "threadId": thread_id, "model": model }))
}

#[tauri::command]
pub fn thread_set_model(
    state: State<'_, AppState>,
    thread_id: String,
    model: Option<String>,
) -> CmdResult<Value> {
    thread_set_model_impl(&state, thread_id, model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_workspace::Workspace;
    use tempfile::tempdir;

    #[test]
    fn test_budget_status_and_ledger() {
        let tmp = tempdir().unwrap();
        let ws = Workspace::create(tmp.path().join("ws"), "Test WS").unwrap();
        let db = WorkspaceDb::open(&ws).unwrap();

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        db.conn()
            .execute(
                "INSERT INTO cost_ledger (id, turn_id, job_id, provider, model, cost_usd, details_json, created_at)
                 VALUES (?1, 'turn-1', NULL, 'claude', 'sonnet', 0.25, NULL, ?2)",
                params![uuid::Uuid::new_v4().to_string(), format!("{today}T12:00:00Z")],
            )
            .unwrap();

        let status = get_budget_status(&ws).unwrap();
        assert!((status.today_spend_usd - 0.25).abs() < 1e-4);
        assert!((status.month_spend_usd - 0.25).abs() < 1e-4);
        assert!(!status.is_daily_over);
    }

    #[test]
    fn test_calculate_turn_budget_caps_at_remaining_daily() {
        let tmp = tempdir().unwrap();
        let ws = Workspace::create(tmp.path().join("ws"), "Test WS").unwrap();
        let db = WorkspaceDb::open(&ws).unwrap();

        // Write studio.yaml with daily_budget_usd: 1.0
        let studio_yaml_path = ws.root().join("studio.yaml");
        let meta = studio_workspace::WorkspaceMeta {
            name: ws.name().to_string(),
            version: 1,
            default_brand: None,
            monthly_budget_usd: Some(10.0),
            daily_budget_usd: Some(1.0),
            ..Default::default()
        };
        std::fs::write(&studio_yaml_path, serde_yaml::to_string(&meta).unwrap()).unwrap();

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        db.conn()
            .execute(
                "INSERT INTO cost_ledger (id, turn_id, job_id, provider, model, cost_usd, details_json, created_at)
                 VALUES (?1, 'turn-1', NULL, 'claude', 'sonnet', 0.80, NULL, ?2)",
                params![uuid::Uuid::new_v4().to_string(), format!("{today}T12:00:00Z")],
            )
            .unwrap();

        let budget = calculate_turn_budget(&ws).unwrap();
        // 1.0 - 0.8 = 0.2 remaining today, which is < DEFAULT_TURN_BUDGET_USD (1.0)
        assert!((budget - 0.20).abs() < 1e-4);

        // When spent >= daily limit -> budget is 0.0
        db.conn()
            .execute(
                "INSERT INTO cost_ledger (id, turn_id, job_id, provider, model, cost_usd, details_json, created_at)
                 VALUES (?1, 'turn-2', NULL, 'claude', 'sonnet', 0.30, NULL, ?2)",
                params![uuid::Uuid::new_v4().to_string(), format!("{today}T13:00:00Z")],
            )
            .unwrap();

        let budget_exhausted = calculate_turn_budget(&ws).unwrap();
        assert_eq!(budget_exhausted, 0.0);
    }

    #[test]
    fn budget_write_keeps_other_keys_and_refuses_invalid_yaml() {
        let tmp = tempdir().unwrap();
        let ws = Workspace::create(tmp.path().join("ws"), "Test WS").unwrap();
        let path = ws.root().join("studio.yaml");
        std::fs::write(
            &path,
            "name: Test WS\nversion: 1\ndefault_brand: demo\ncustom_key: keep-me\nbudget_day_usd: 2.0\n",
        )
        .unwrap();

        write_budget_to_studio_yaml(&ws, Some(5.0), None).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("custom_key: keep-me"));
        assert!(text.contains("default_brand: demo"));
        assert!(text.contains("daily_budget_usd: 5.0"));
        assert!(!text.contains("budget_day_usd"));

        std::fs::write(&path, "name: [unclosed").unwrap();
        assert!(write_budget_to_studio_yaml(&ws, Some(1.0), None).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "name: [unclosed");
    }
}
