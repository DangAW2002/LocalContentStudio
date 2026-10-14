use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorRole {
    Human,
    Agent,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorContext {
    pub actor_id: String,
    pub role: ActorRole,
    pub active_workspace_id: String,
    pub grant_token: Option<String>,
}

impl ActorContext {
    pub fn new_human(actor_id: impl Into<String>, workspace_id: impl Into<String>) -> Self {
        Self {
            actor_id: actor_id.into(),
            role: ActorRole::Human,
            active_workspace_id: workspace_id.into(),
            grant_token: None,
        }
    }

    pub fn new_agent(actor_id: impl Into<String>, workspace_id: impl Into<String>) -> Self {
        Self {
            actor_id: actor_id.into(),
            role: ActorRole::Agent,
            active_workspace_id: workspace_id.into(),
            grant_token: None,
        }
    }

    pub fn is_human(&self) -> bool {
        self.role == ActorRole::Human
    }

    pub fn is_agent(&self) -> bool {
        self.role == ActorRole::Agent
    }
}
