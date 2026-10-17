//! Manifest and schema definitions for capabilities and tools.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::Duration;
use studio_jobs::{Lifecycle, ResourceSpec};

/// Capability schema definition loaded from `capabilities/<id>@<n>.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySchema {
    /// Capability identifier (e.g. `tts.synthesize`).
    pub id: String,
    /// Schema major version (e.g. 1).
    pub version: u32,
    /// Human-readable title.
    pub title: String,
    /// Detailed description.
    pub description: String,
    /// JSON Schema for tool inputs.
    #[serde(alias = "inputs", alias = "inputSchema", default)]
    pub input_schema: serde_json::Value,
    /// JSON Schema for tool outputs.
    #[serde(alias = "outputs", alias = "outputSchema", default)]
    pub output_schema: serde_json::Value,
}

impl CapabilitySchema {
    /// Formatted canonical identifier with version: `<id>@<version>`.
    pub fn full_id(&self) -> String {
        format!("{}@{}", self.id, self.version)
    }
}

/// Category of tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// Generates new media, assets, or content.
    Generator,
    /// Renders artifacts into output formats.
    Renderer,
    /// Imports external resources, URLs, or files.
    Importer,
    /// Verifies compliance, brand rules, or media specs.
    Checker,
    /// Publishes content to external platforms.
    Publisher,
    /// Visualizes or previews artifacts.
    Viewer,
}

impl ToolKind {
    /// User-facing Vietnamese translation of tool kind.
    pub fn to_vietnamese(&self) -> &'static str {
        match self {
            Self::Generator => "Tạo nội dung (Generator)",
            Self::Renderer => "Kết xuất (Renderer)",
            Self::Importer => "Nhập liệu (Importer)",
            Self::Checker => "Kiểm tra chuẩn (Checker)",
            Self::Publisher => "Đăng xuất bản (Publisher)",
            Self::Viewer => "Trình xem (Viewer)",
        }
    }
}

/// Execution backend type for a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRunType {
    /// Command-line executable or script.
    Cli,
    /// Remote or sub-process Model Context Protocol server.
    Mcp,
    /// In-process native Rust function or engine component.
    Builtin,
}

/// Execution specification for running a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRunSpec {
    /// Execution mode: cli, mcp, or builtin.
    #[serde(rename = "type")]
    pub run_type: ToolRunType,
    /// Program executable name, command string, or builtin identifier.
    pub command: String,
    /// Positional arguments for CLI invocation.
    #[serde(default)]
    pub args: Vec<String>,
}

/// Cost metrics and estimation for tool invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCost {
    /// Billing unit (e.g. "call", "second", "token", "character").
    pub unit: String,
    /// Cost estimate value (numeric USD cost or arbitrary descriptor).
    #[serde(default)]
    pub estimate: serde_json::Value,
}

/// Potential side-effect classifications of a tool invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffect {
    /// Safe execution with no external side effects.
    #[default]
    None,
    /// Incurs monetary cost; subject to user turn/daily budget checks.
    Paid,
    /// Publishes content to public platforms (e.g. YouTube, social media).
    ExternalPublish,
    /// Permanently modifies or deletes files or databases.
    Destructive,
}

impl SideEffect {
    /// Returns true if this side effect requires explicit approval before running.
    pub fn requires_approval(&self) -> bool {
        matches!(self, Self::Paid | Self::ExternalPublish | Self::Destructive)
    }

    /// User-facing Vietnamese explanation of side effect.
    pub fn to_vietnamese(&self) -> &'static str {
        match self {
            Self::None => "Không có tác dụng phụ",
            Self::Paid => "Phát sinh chi phí (có phí)",
            Self::ExternalPublish => "Đăng tải công khai ra ngoài",
            Self::Destructive => "Sửa đổi hoặc xóa dữ liệu",
        }
    }
}

/// Complete tool manifest loaded from `tools/<id>/tool.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolManifest {
    /// Unique identifier of the tool (e.g. `echo`, `ffmpeg.probe`).
    pub id: String,
    /// Tool version (e.g. "1.0.0" or "1").
    #[serde(default)]
    pub version: Option<String>,
    /// Tool category.
    pub kind: ToolKind,
    /// Human-readable description.
    pub description: String,
    /// Capabilities provided by this tool, formatted as `<id>@<version>`.
    #[serde(default)]
    pub provides: Vec<String>,
    /// How the tool is invoked.
    pub run: ToolRunSpec,
    /// Required dependencies before tool can be enabled (e.g. `["secret:KEY", "bin:ffmpeg", "app:blender"]`).
    #[serde(default)]
    pub needs: Vec<String>,
    /// Cost profile.
    #[serde(default)]
    pub cost: Option<ToolCost>,
    /// Side-effect level.
    #[serde(default)]
    pub side_effect: SideEffect,
    /// Resource specifications for scheduler admission.
    #[serde(default)]
    pub resources: ResourceSpec,
    /// Process lifecycle model (per_call or warm).
    #[serde(
        default,
        deserialize_with = "deserialize_lifecycle",
        serialize_with = "serialize_lifecycle"
    )]
    pub lifecycle: Lifecycle,
}

fn deserialize_lifecycle<'de, D>(deserializer: D) -> std::result::Result<Lifecycle, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum LifecycleHelper {
        Str(String),
        WarmObj { warm: WarmConfig },
    }

    #[derive(Deserialize)]
    struct WarmConfig {
        #[serde(default = "default_idle_secs")]
        idle_timeout_s: u64,
    }

    fn default_idle_secs() -> u64 {
        300
    }

    match LifecycleHelper::deserialize(deserializer)? {
        LifecycleHelper::Str(s) => match s.as_str() {
            "warm" => Ok(Lifecycle::Warm {
                idle_timeout: Duration::from_secs(300),
            }),
            _ => Ok(Lifecycle::PerCall),
        },
        LifecycleHelper::WarmObj { warm } => Ok(Lifecycle::Warm {
            idle_timeout: Duration::from_secs(warm.idle_timeout_s),
        }),
    }
}

fn serialize_lifecycle<S>(
    lifecycle: &Lifecycle,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match lifecycle {
        Lifecycle::PerCall => serializer.serialize_str("per_call"),
        Lifecycle::Warm { idle_timeout } => {
            #[derive(Serialize)]
            struct WarmObj {
                idle_timeout_s: u64,
            }
            #[derive(Serialize)]
            struct Wrapper {
                warm: WarmObj,
            }
            Wrapper {
                warm: WarmObj {
                    idle_timeout_s: idle_timeout.as_secs(),
                },
            }
            .serialize(serializer)
        }
    }
}
