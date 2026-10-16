//! Registry for discovering, validating, and managing capabilities and tools.

use crate::manifest::{CapabilitySchema, ToolManifest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use studio_workspace::{ResolvedItem, ScopeChain, Workspace};

/// Problem or validation error encountered while parsing a capability or tool manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestProblem {
    /// Path to the problematic file or directory.
    pub path: PathBuf,
    /// Detailed error message.
    pub message: String,
}

/// Registry of capabilities and tools resolved across the scope chain.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    capabilities: BTreeMap<String, CapabilitySchema>,
    tools: BTreeMap<String, ToolManifest>,
    problems: Vec<ManifestProblem>,
}

impl Registry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load registry for a given workspace across the scope chain (App -> Org -> Workspace).
    /// Built-in tools and capabilities form the base layer.
    /// Does not fail hard on invalid manifests; records problems into `problems`.
    pub fn load(ws: &Workspace) -> Self {
        let mut registry = Self::new();

        // 1. Load built-in capabilities and tools
        registry.load_builtins();

        // 2. Resolve capabilities across scope chain
        let chain = ScopeChain::new(ws);
        let resolved_caps = chain.resolve("capabilities");
        for item in resolved_caps {
            registry.load_capability_item(&item);
        }

        // 3. Resolve tools across scope chain
        let resolved_tools = chain.resolve("tools");
        for item in resolved_tools {
            registry.load_tool_item(&item);
        }

        registry
    }

    /// Get capability by full ID (e.g. `tts.synthesize@1`) or base ID (e.g. `tts.synthesize`).
    pub fn get_capability(&self, id: &str) -> Option<&CapabilitySchema> {
        if let Some(c) = self.capabilities.get(id) {
            return Some(c);
        }

        // Try matching by base ID if version was omitted
        for (k, v) in &self.capabilities {
            if k == id || v.id == id {
                return Some(v);
            }
        }

        None
    }

    /// Get tool manifest by ID.
    pub fn get_tool(&self, id: &str) -> Option<&ToolManifest> {
        self.tools.get(id)
    }

    /// List all registered capabilities.
    pub fn list_capabilities(&self) -> Vec<&CapabilitySchema> {
        self.capabilities.values().collect()
    }

    /// List all registered tools.
    pub fn list_tools(&self) -> Vec<&ToolManifest> {
        self.tools.values().collect()
    }

    /// List all manifest problems reported during loading.
    pub fn problems(&self) -> &[ManifestProblem] {
        &self.problems
    }

    /// Manually register or override a capability.
    pub fn register_capability(&mut self, cap: CapabilitySchema) {
        let key = cap.full_id();
        self.capabilities.insert(key, cap);
    }

    /// Manually register or override a tool.
    pub fn register_tool(&mut self, tool: ToolManifest) {
        self.tools.insert(tool.id.clone(), tool);
    }

    fn load_builtins(&mut self) {
        // Built-in capabilities
        let builtin_caps: &[(&str, &str)] = &[
            (
                "echo@1",
                include_str!("../builtin/capabilities/echo@1.json"),
            ),
            (
                "tts.synthesize@1",
                include_str!("../builtin/capabilities/tts.synthesize@1.json"),
            ),
            (
                "ffmpeg.probe@1",
                include_str!("../builtin/capabilities/ffmpeg.probe@1.json"),
            ),
        ];

        for (id, content) in builtin_caps {
            match serde_json::from_str::<CapabilitySchema>(content) {
                Ok(cap) => {
                    self.capabilities.insert(id.to_string(), cap);
                }
                Err(err) => self.problems.push(ManifestProblem {
                    path: PathBuf::from(format!("builtin:capabilities/{id}.json")),
                    message: format!("Capability có sẵn không hợp lệ: {err}"),
                }),
            }
        }

        // Built-in tools
        let builtin_tools: &[&str] = &[
            include_str!("../builtin/tools/echo/tool.yaml"),
            include_str!("../builtin/tools/ffmpeg.probe/tool.yaml"),
            include_str!("../builtin/tools/tts.windows-sapi/tool.yaml"),
            include_str!("../builtin/tools/tts.fake/tool.yaml"),
        ];

        for (i, content) in builtin_tools.iter().enumerate() {
            match serde_yaml::from_str::<ToolManifest>(content) {
                Ok(tool) => {
                    self.tools.insert(tool.id.clone(), tool);
                }
                Err(err) => self.problems.push(ManifestProblem {
                    path: PathBuf::from(format!("builtin:tools/{i}/tool.yaml")),
                    message: format!("Tool có sẵn không hợp lệ: {err}"),
                }),
            }
        }
    }

    fn load_capability_item(&mut self, item: &ResolvedItem) {
        let path = if item.path.is_file() {
            item.path.clone()
        } else {
            item.path.join(format!("{}.json", item.id))
        };

        if !path.exists() {
            return;
        }

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<CapabilitySchema>(&content) {
                Ok(cap) => {
                    let key = cap.full_id();
                    self.capabilities.insert(key, cap);
                }
                Err(err) => {
                    self.problems.push(ManifestProblem {
                        path,
                        message: format!("Không thể phân tích tệp capability JSON: {}", err),
                    });
                }
            },
            Err(err) => {
                self.problems.push(ManifestProblem {
                    path,
                    message: format!("Không thể đọc tệp capability: {}", err),
                });
            }
        }
    }

    fn load_tool_item(&mut self, item: &ResolvedItem) {
        let manifest_path = if item.path.is_dir() {
            let yml = item.path.join("tool.yaml");
            if yml.exists() {
                yml
            } else {
                item.path.join("tool.yml")
            }
        } else {
            item.path.clone()
        };

        if !manifest_path.exists() {
            self.problems.push(ManifestProblem {
                path: item.path.clone(),
                message: "Thư mục công cụ không chứa tệp cấu hình tool.yaml".to_string(),
            });
            return;
        }

        match fs::read_to_string(&manifest_path) {
            Ok(content) => match serde_yaml::from_str::<ToolManifest>(&content) {
                Ok(tool) => {
                    self.tools.insert(tool.id.clone(), tool);
                }
                Err(err) => {
                    self.problems.push(ManifestProblem {
                        path: manifest_path,
                        message: format!("Không thể phân tích cú pháp tệp tool.yaml: {}", err),
                    });
                }
            },
            Err(err) => {
                self.problems.push(ManifestProblem {
                    path: manifest_path,
                    message: format!("Không thể đọc tệp tool.yaml: {}", err),
                });
            }
        }
    }
}
