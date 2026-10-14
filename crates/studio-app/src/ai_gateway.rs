use serde::{Deserialize, Serialize};
use studio_core::error::StudioError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderKind {
    Api,
    Local,
    Cli,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIProviderConfig {
    pub id: String,
    pub kind: ProviderKind,
    pub base_url: Option<String>,
    pub model: String,
    pub enabled: bool,
    pub timeout_seconds: u32,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedCompletionResponse {
    pub text: String,
    pub model: String,
    pub usage_tokens: u32,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AIGateway {
    config: AIProviderConfig,
    api_key: Option<String>,
}

impl AIGateway {
    pub fn new(config: AIProviderConfig, api_key: Option<String>) -> Self {
        Self { config, api_key }
    }

    pub fn provider_config(&self) -> &AIProviderConfig {
        &self.config
    }

    /// Probes provider connectivity and schema readiness
    pub async fn probe(&self) -> Result<bool, StudioError> {
        if !self.config.enabled {
            return Ok(false);
        }

        match self.config.kind {
            ProviderKind::Local => {
                // Check if local endpoint is reachable
                if let Some(ref url) = self.config.base_url {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(2))
                        .build()
                        .map_err(|e| StudioError::Internal {
                            message: e.to_string(),
                            correlation_id: "probe".into(),
                        })?;
                    let res = client.get(url).send().await;
                    Ok(res.is_ok())
                } else {
                    Ok(false)
                }
            }
            ProviderKind::Api => {
                // If API provider has key configured
                Ok(self.api_key.is_some())
            }
            ProviderKind::Cli => {
                // Verify that system tool / CLI actually exists
                let exists = std::process::Command::new("cargo")
                    .arg("--version")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                Ok(exists)
            }
        }
    }

    /// Normalizes completions across OpenAI-compatible or local adapters
    pub async fn generate_completion(
        &self,
        prompt: &str,
        system_instructions: &str,
    ) -> Result<NormalizedCompletionResponse, StudioError> {
        if !self.config.enabled {
            return Err(StudioError::PreconditionFailed {
                reason: "AI Provider hiện đang bị tắt. Người dùng có thể tiếp tục sử dụng chế độ chỉnh sửa thủ công (No-AI mode).".into(),
                correlation_id: "ai_gateway".into(),
            });
        }

        // Truth in advertising: Never generate fake completion text when unconfigured!
        if self.api_key.is_none() && self.config.base_url.is_none() {
            return Err(StudioError::PreconditionFailed {
                reason: "Chưa cấu hình API Key hoặc endpoint AI. Hệ thống hoạt động ở chế độ Không-AI (No-AI mode) cho phép người dùng sáng tạo và chỉnh sửa thủ công hoàn toàn.".into(),
                correlation_id: "ai_gateway".into(),
            });
        }

        let base = self
            .config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let endpoint = format!("{}/chat/completions", base.trim_end_matches('/'));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                self.config.timeout_seconds as u64,
            ))
            .build()
            .map_err(|e| StudioError::Internal {
                message: format!("HTTP client init failed: {}", e),
                correlation_id: "ai".into(),
            })?;

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                { "role": "system", "content": system_instructions },
                { "role": "user", "content": prompt }
            ],
            "max_tokens": self.config.max_tokens,
            "temperature": 0.7
        });

        let mut req = client.post(&endpoint).json(&body);
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await.map_err(|e| StudioError::Internal {
            message: format!("AI Gateway request failed: {}", e),
            correlation_id: "ai".into(),
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            return Err(StudioError::Internal {
                message: format!("AI Provider error {}: {}", status, err_body),
                correlation_id: "ai".into(),
            });
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| StudioError::Internal {
            message: format!("AI Provider response JSON invalid: {}", e),
            correlation_id: "ai".into(),
        })?;

        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let total_tokens = json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32;

        Ok(NormalizedCompletionResponse {
            text,
            model: self.config.model.clone(),
            usage_tokens: total_tokens,
            warnings: vec![],
        })
    }
}
