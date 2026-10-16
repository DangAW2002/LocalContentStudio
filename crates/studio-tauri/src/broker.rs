//! Permission broker that asks the user through the Thread UI (ADR-008 §5).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::{json, Value};
use studio_mcp::{permission::policy_check, BoxFuture, Decision, PermissionBroker, TurnContext};
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;

/// How long a permission dialog waits before denying.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(600);

/// Routes engine permission requests to a UI dialog and waits for `permission_respond`.
#[derive(Default)]
pub struct UiBroker {
    app: OnceLock<AppHandle>,
    pending: Mutex<HashMap<String, oneshot::Sender<bool>>>,
    /// turn id → thread id, so events reach the right Thread pane.
    threads: Mutex<HashMap<String, String>>,
}

impl UiBroker {
    /// Attach the app handle once the Tauri app is built.
    pub fn attach(&self, app: AppHandle) {
        let _ = self.app.set(app);
    }

    /// Remember which thread a turn belongs to.
    pub fn register_turn(&self, turn_id: &str, thread_id: &str) {
        if let Ok(mut m) = self.threads.lock() {
            m.insert(turn_id.to_string(), thread_id.to_string());
        }
    }

    /// Forget a finished turn.
    pub fn unregister_turn(&self, turn_id: &str) {
        if let Ok(mut m) = self.threads.lock() {
            m.remove(turn_id);
        }
    }

    /// Deliver the user's answer. Returns false when the request is unknown or expired.
    pub fn respond(&self, request_id: &str, allow: bool) -> bool {
        let sender = self
            .pending
            .lock()
            .ok()
            .and_then(|mut m| m.remove(request_id));
        sender.is_some_and(|tx| tx.send(allow).is_ok())
    }
}

impl PermissionBroker for UiBroker {
    fn decide<'a>(
        &'a self,
        ctx: &'a TurnContext,
        tool_name: &'a str,
        input: &'a Value,
    ) -> BoxFuture<'a, Decision> {
        Box::pin(async move {
            if let Some(reason) = policy_check(ctx, tool_name, input) {
                return Decision::Deny(reason);
            }
            let (Some(app), Some(thread_id)) = (
                self.app.get(),
                self.threads
                    .lock()
                    .ok()
                    .and_then(|m| m.get(&ctx.turn_id).cloned()),
            ) else {
                return Decision::Deny("Studio không hiển thị được hộp thoại cấp quyền.".into());
            };
            let request_id = uuid::Uuid::new_v4().to_string();
            let (tx, rx) = oneshot::channel();
            if let Ok(mut m) = self.pending.lock() {
                m.insert(request_id.clone(), tx);
            }
            let event = json!({ "type": "PermissionRequest", "requestId": request_id, "tool": tool_name, "input": input });
            let _ = app.emit(
                "turn-event",
                json!({ "threadId": thread_id, "turnId": ctx.turn_id, "event": event }),
            );
            match tokio::time::timeout(ANSWER_TIMEOUT, rx).await {
                Ok(Ok(true)) => Decision::Allow(input.clone()),
                Ok(Ok(false)) => Decision::Deny("Người dùng đã từ chối.".into()),
                _ => {
                    if let Ok(mut m) = self.pending.lock() {
                        m.remove(&request_id);
                    }
                    Decision::Deny("Hết thời gian chờ người dùng cấp quyền.".into())
                }
            }
        })
    }
}
