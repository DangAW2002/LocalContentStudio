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

/// The answer delivered by the user to a pending broker request.
#[derive(Debug)]
pub enum BrokerAnswer {
    /// Permission decision (allow/deny).
    Permission(bool),
    /// Answer map for questions (`AskUserQuestion`).
    Question(Value),
}

/// A permission request still waiting for the user.
struct Pending {
    tx: oneshot::Sender<BrokerAnswer>,
    /// What the UI needs to redraw the dialog (`requestId`, `threadId`, `turnId`, `tool`, `input`).
    info: Value,
}

/// Routes engine permission requests to a UI dialog and waits for `permission_respond` or `question_respond`.
///
/// The `turn-event` it emits is transient, so open requests are also kept here and listed by
/// `permission_pending`: a Thread pane mounted later (or another view) can still show them.
#[derive(Default)]
pub struct UiBroker {
    app: OnceLock<AppHandle>,
    pending: Mutex<HashMap<String, Pending>>,
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

    /// Deliver the user's permission answer. Returns false when the request is unknown or expired.
    pub fn respond(&self, request_id: &str, allow: bool) -> bool {
        let sender = self
            .pending
            .lock()
            .ok()
            .and_then(|mut m| m.remove(request_id));
        sender.is_some_and(|p| p.tx.send(BrokerAnswer::Permission(allow)).is_ok())
    }

    /// Deliver the user's answer to a question prompt. Returns false when the request is unknown or expired.
    pub fn respond_question(&self, request_id: &str, answers: Value) -> bool {
        let sender = self
            .pending
            .lock()
            .ok()
            .and_then(|mut m| m.remove(request_id));
        sender.is_some_and(|p| p.tx.send(BrokerAnswer::Question(answers)).is_ok())
    }

    /// Open requests, oldest first, optionally for one thread.
    pub fn list_pending(&self, thread_id: Option<&str>) -> Vec<Value> {
        let Ok(m) = self.pending.lock() else {
            return Vec::new();
        };
        let mut out: Vec<Value> = m
            .values()
            .filter(|p| thread_id.is_none_or(|t| p.info["threadId"] == t))
            .map(|p| p.info.clone())
            .collect();
        out.sort_by(|a, b| a["createdAt"].as_str().cmp(&b["createdAt"].as_str()));
        out
    }

    /// Inserts a fake pending permission for unit tests.
    pub fn insert_pending_for_test(&self, request_id: &str, thread_id: &str, turn_id: &str) {
        let (tx, _rx) = oneshot::channel();
        let info = json!({
            "requestId": request_id,
            "threadId": thread_id,
            "turnId": turn_id,
            "createdAt": chrono::Utc::now().to_rfc3339(),
        });
        if let Ok(mut m) = self.pending.lock() {
            m.insert(request_id.to_string(), Pending { tx, info });
        }
    }

    /// Inserts a pending request with a receiver channel for unit tests.
    #[cfg(test)]
    pub fn insert_pending_with_rx_for_test(
        &self,
        request_id: &str,
        thread_id: &str,
        turn_id: &str,
    ) -> oneshot::Receiver<BrokerAnswer> {
        let (tx, rx) = oneshot::channel();
        let info = json!({
            "requestId": request_id,
            "threadId": thread_id,
            "turnId": turn_id,
            "createdAt": chrono::Utc::now().to_rfc3339(),
        });
        if let Ok(mut m) = self.pending.lock() {
            m.insert(request_id.to_string(), Pending { tx, info });
        }
        rx
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
            let info = json!({
                "requestId": request_id,
                "threadId": thread_id,
                "turnId": ctx.turn_id,
                "project": ctx.project,
                "tool": tool_name,
                "input": input,
                "createdAt": chrono::Utc::now().to_rfc3339(),
            });
            if let Ok(mut m) = self.pending.lock() {
                m.insert(request_id.clone(), Pending { tx, info });
            }
            let event = json!({ "type": "PermissionRequest", "requestId": request_id, "tool": tool_name, "input": input });
            let _ = app.emit(
                "turn-event",
                json!({ "threadId": thread_id, "turnId": ctx.turn_id, "event": event }),
            );
            match tokio::time::timeout(ANSWER_TIMEOUT, rx).await {
                Ok(Ok(BrokerAnswer::Permission(true))) => Decision::Allow(input.clone()),
                Ok(Ok(BrokerAnswer::Permission(false))) => {
                    Decision::Deny("Người dùng đã từ chối.".into())
                }
                Ok(Ok(BrokerAnswer::Question(answers))) => {
                    let mut updated = input.clone();
                    if let Value::Object(ref mut map) = updated {
                        map.insert("answers".to_string(), answers);
                    }
                    Decision::Allow(updated)
                }
                _ => {
                    if let Ok(mut m) = self.pending.lock() {
                        m.remove(&request_id);
                    }
                    Decision::Deny("Hết thời gian chờ người dùng phản hồi.".into())
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broker_permission_respond() {
        let broker = UiBroker::default();
        let rx = broker.insert_pending_with_rx_for_test("req-1", "th-1", "turn-1");
        assert_eq!(broker.list_pending(None).len(), 1);

        assert!(broker.respond("req-1", true));
        let answer = rx.blocking_recv().expect("must receive answer");
        match answer {
            BrokerAnswer::Permission(allow) => assert!(allow),
            _ => panic!("expected Permission"),
        }
        assert_eq!(broker.list_pending(None).len(), 0);
    }

    #[test]
    fn test_broker_question_respond() {
        let broker = UiBroker::default();
        let rx = broker.insert_pending_with_rx_for_test("req-2", "th-1", "turn-1");

        let answers = json!({
            "Chọn thư viện nào?": "date-fns",
            "Tính năng bổ sung?": ["Lint", "Prettier"]
        });

        assert!(broker.respond_question("req-2", answers.clone()));
        let answer = rx.blocking_recv().expect("must receive answer");
        match answer {
            BrokerAnswer::Question(recv_answers) => assert_eq!(recv_answers, answers),
            _ => panic!("expected Question"),
        }
        assert_eq!(broker.list_pending(None).len(), 0);
    }

    #[test]
    fn test_broker_respond_unknown() {
        let broker = UiBroker::default();
        assert!(!broker.respond("nonexistent", true));
        assert!(!broker.respond_question("nonexistent", json!({})));
    }
}
