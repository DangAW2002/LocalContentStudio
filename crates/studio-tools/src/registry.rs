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

    /// Create registry initialized with built-in capabilities and tools, plus app-level installed tools.
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        reg.load_builtins();
        let app_tools_dir = studio_workspace::AppPaths::from_env()
            .data_dir()
            .join("tools");
        reg.load_tools_from_dir(&app_tools_dir);
        reg
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

        // 4. Also scan installed tools (<app>/tools/ and <ws>/.studio/tools/)
        let app_paths = ScopeChain::new(ws);
        let _ = app_paths;
        let app_tools_dir = studio_workspace::AppPaths::from_env()
            .data_dir()
            .join("tools");
        registry.load_tools_from_dir(&app_tools_dir);

        let ws_installed_tools = ws.root().join(".studio").join("tools");
        registry.load_tools_from_dir(&ws_installed_tools);

        registry
    }

    /// Load tools from directory containing `<id>` or `<id>@<version>` subdirectories.
    pub fn load_tools_from_dir(&mut self, dir: &std::path::Path) {
        if !dir.exists() || !dir.is_dir() {
            return;
        }
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let yml = path.join("tool.yaml");
                let yml_path = if yml.exists() {
                    yml
                } else {
                    path.join("tool.yml")
                };
                if yml_path.exists() {
                    match fs::read_to_string(&yml_path) {
                        Ok(content) => match serde_yaml::from_str::<ToolManifest>(&content) {
                            Ok(tool) => {
                                self.tools.insert(tool.id.clone(), tool);
                            }
                            Err(err) => {
                                self.problems.push(ManifestProblem {
                                    path: yml_path,
                                    message: format!(
                                        "Không thể phân tích cú pháp tệp tool.yaml: {}",
                                        err
                                    ),
                                });
                            }
                        },
                        Err(err) => {
                            self.problems.push(ManifestProblem {
                                path: yml_path,
                                message: format!("Không thể đọc tệp tool.yaml: {}", err),
                            });
                        }
                    }
                }
            }
        }
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

    /// List all secret keys referenced in tool manifests across `needs`.
    pub fn referenced_secrets(&self) -> std::collections::BTreeSet<String> {
        let mut set = std::collections::BTreeSet::new();
        for tool in self.tools.values() {
            for need in &tool.needs {
                if let Some(key) = need.strip_prefix("secret:") {
                    set.insert(key.to_string());
                }
            }
        }
        set
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
            (
                "image.generate@1",
                include_str!("../builtin/capabilities/image.generate@1.json"),
            ),
            (
                "video.generate@1",
                include_str!("../builtin/capabilities/video.generate@1.json"),
            ),
            (
                "media.import@1",
                include_str!("../builtin/capabilities/media.import@1.json"),
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
            include_str!("../builtin/tools/google.imagen/tool.yaml"),
            include_str!("../builtin/tools/openai.images/tool.yaml"),
            include_str!("../builtin/tools/openai.videos/tool.yaml"),
            include_str!("../builtin/tools/google.veo/tool.yaml"),
            include_str!("../builtin/tools/media.import.yt-dlp/tool.yaml"),
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
