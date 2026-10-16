use crate::app::AppPaths;
use crate::workspace::Workspace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Layer where an item was resolved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ResolutionLayer {
    /// App level (`%APPDATA%/LocalContentStudio/shared`)
    App,
    /// Organization level (`<org>/shared`)
    Org,
    /// Workspace level (`<workspace>`)
    Workspace,
}

/// Resolved item from scope chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedItem {
    /// Identifier: file stem (e.g. `youtube` for `youtube.yaml`) or directory name (for `brands/<slug>`)
    pub id: String,
    /// Absolute or canonical path to item
    pub path: PathBuf,
    /// Layer this item came from
    pub layer: ResolutionLayer,
}

/// Scope chain resolver (App -> Org -> Workspace).
pub struct ScopeChain<'a> {
    ws: &'a Workspace,
    app_paths: AppPaths,
}

impl<'a> ScopeChain<'a> {
    /// Create scope chain for a workspace using default AppPaths.
    pub fn new(ws: &'a Workspace) -> Self {
        Self {
            ws,
            app_paths: AppPaths::from_env(),
        }
    }

    /// Create scope chain with custom AppPaths (useful for tests).
    pub fn with_app_paths(ws: &'a Workspace, app_paths: AppPaths) -> Self {
        Self { ws, app_paths }
    }

    /// Resolve items for a given kind:
    /// `brands`, `channels`, `playbooks`, `skills`, `tools`, `capabilities`, `library`.
    /// Nearer layer overrides by id (Workspace > Org > App).
    pub fn resolve(&self, kind: &str) -> Vec<ResolvedItem> {
        // Collect map: id -> ResolvedItem
        // We evaluate App first, then Org overrides, then Workspace overrides.
        let mut map: HashMap<String, ResolvedItem> = HashMap::new();

        // 1. App layer: <app_data>/shared/<kind>
        let app_kind_dir = self.app_paths.shared_dir().join(kind);
        Self::scan_dir(&app_kind_dir, ResolutionLayer::App, &mut map);

        // 2. Org layer: <org>/shared/<kind>
        let org_kind_dir = self.ws.org().shared_dir().join(kind);
        Self::scan_dir(&org_kind_dir, ResolutionLayer::Org, &mut map);

        // 3. Workspace layer: <ws>/<kind>
        let ws_kind_dir = self.ws.root().join(kind);
        Self::scan_dir(&ws_kind_dir, ResolutionLayer::Workspace, &mut map);

        let mut items: Vec<ResolvedItem> = map.into_values().collect();
        items.sort_by(|a, b| a.id.cmp(&b.id));
        items
    }

    fn scan_dir(dir: &Path, layer: ResolutionLayer, map: &mut HashMap<String, ResolvedItem>) {
        if !dir.exists() || !dir.is_dir() {
            return;
        }

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files or directories
            if file_name.starts_with('.') {
                continue;
            }

            let id = if path.is_dir() {
                // For directory items (like brands/<slug>/ or tools/<id>/)
                file_name
            } else if let Some(stem) = path.file_stem() {
                // For file items (channels/<id>.yaml, playbooks/<id>.md, etc.)
                stem.to_string_lossy().to_string()
            } else {
                file_name
            };

            map.insert(id.clone(), ResolvedItem { id, path, layer });
        }
    }
}
