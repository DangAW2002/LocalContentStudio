//! Capability provider routing.

use crate::manifest::ToolManifest;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use studio_workspace::Workspace;

/// Read `providers` map from a YAML file (`studio.yaml` or `org.yaml`).
fn read_providers_from_yaml(path: &Path) -> Option<std::collections::HashMap<String, String>> {
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    let providers_obj = value.get("providers")?.as_mapping()?;

    let mut map = std::collections::HashMap::new();
    for (k, v) in providers_obj {
        if let (Some(k_str), Some(v_str)) = (k.as_str(), v.as_str()) {
            map.insert(k_str.to_string(), v_str.to_string());
        }
    }
    Some(map)
}

/// Route a capability request to an ordered list of candidate tool providers.
///
/// Order:
/// 1. `explicit_provider` (if specified)
/// 2. Workspace default (`studio.yaml` -> `providers: { cap: tool_id }`)
/// 3. Org default (`org.yaml` -> `providers: { cap: tool_id }`)
/// 4. App default (app data shared/settings)
/// 5. Fallback list: all registered tools that declare `provides: [cap]`
pub fn route(
    cap: &str,
    explicit_provider: Option<&str>,
    ws: &Workspace,
    registered_tools: &[ToolManifest],
) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();

    let mut add = |id: &str| {
        if !seen.contains(id) {
            seen.insert(id.to_string());
            ordered.push(id.to_string());
        }
    };

    // 1. Explicit provider
    if let Some(explicit) = explicit_provider {
        if !explicit.trim().is_empty() {
            add(explicit.trim());
        }
    }

    // 2. Workspace default
    let ws_yaml = ws.root().join("studio.yaml");
    if let Some(providers) = read_providers_from_yaml(&ws_yaml) {
        if let Some(tool_id) = providers.get(cap).or_else(|| {
            // Also check without version tag (e.g. tts.synthesize for tts.synthesize@1)
            let base_cap = cap.split('@').next().unwrap_or(cap);
            providers.get(base_cap)
        }) {
            add(tool_id);
        }
    }

    // 3. Org default
    let org_yaml = ws.org().root().join("org.yaml");
    if let Some(providers) = read_providers_from_yaml(&org_yaml) {
        if let Some(tool_id) = providers.get(cap).or_else(|| {
            let base_cap = cap.split('@').next().unwrap_or(cap);
            providers.get(base_cap)
        }) {
            add(tool_id);
        }
    }

    // 4. App layer default
    let app_paths = studio_workspace::AppPaths::from_env();
    let app_yaml = app_paths.data_dir().join("app.yaml");
    if let Some(providers) = read_providers_from_yaml(&app_yaml) {
        if let Some(tool_id) = providers.get(cap).or_else(|| {
            let base_cap = cap.split('@').next().unwrap_or(cap);
            providers.get(base_cap)
        }) {
            add(tool_id);
        }
    }

    // 5. Fallback list: any tool that provides this capability
    let dev_mode = std::env::var("STUDIO_DEV").as_deref() == Ok("1");
    let base_cap = cap.split('@').next().unwrap_or(cap);

    // Prioritize Vietnamese TTS providers for tts.synthesize: vieneu > piper > windows-sapi
    if base_cap == "tts.synthesize" {
        let tts_priority = ["tts.vieneu", "tts.piper", "tts.windows-sapi"];
        for pref in &tts_priority {
            if let Some(tool) = registered_tools.iter().find(|t| &t.id == pref) {
                if !tool.internal || dev_mode {
                    add(&tool.id);
                }
            }
        }
    }

    for tool in registered_tools {
        if tool.internal && !dev_mode {
            continue;
        }
        let provides_cap = tool
            .provides
            .iter()
            .any(|p| p == cap || p == base_cap || p.split('@').next() == Some(base_cap));

        if provides_cap {
            add(&tool.id);
        }
    }

    ordered
}

/// Route a capability request to an ordered list of candidate tool providers,
/// skipping any providers whose probe fails.
pub fn route_available(
    cap: &str,
    explicit_provider: Option<&str>,
    ws: &Workspace,
    registered_tools: &[ToolManifest],
    secret_store: &dyn crate::secret::SecretStore,
) -> Vec<String> {
    let candidates = route(cap, explicit_provider, ws, registered_tools);
    candidates
        .into_iter()
        .filter(|tool_id| {
            if let Some(tool) = registered_tools.iter().find(|t| &t.id == tool_id) {
                crate::probe::probe(tool, secret_store, Some(ws)).available
            } else {
                false
            }
        })
        .collect()
}
