use crate::error::{BrandError, Result};
use crate::schema::{BrandSpec, ChannelSpec, Playbook};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use studio_workspace::{ResolutionLayer, ScopeChain, Workspace};
use walkdir::WalkDir;

/// Represents a loaded brand including its root directory and calculated content hash.
#[derive(Debug, Clone)]
pub struct LoadedBrand {
    /// Brand slug / directory name
    pub slug: String,
    /// Absolute directory of the brand
    pub dir: PathBuf,
    /// Parsed spec
    pub spec: BrandSpec,
    /// Full voice.md text (if present)
    pub voice_text: Option<String>,
    /// sha256 hex hash of the brand folder contents
    pub version: String,
    /// Layer from which it was loaded
    pub layer: ResolutionLayer,
}

/// Aggregated set of loaded brands, channels, and playbooks resolved via ScopeChain.
#[derive(Debug, Clone, Default)]
pub struct BrandSet {
    /// Map of brand slug -> LoadedBrand
    pub brands: BTreeMap<String, LoadedBrand>,
    /// Map of channel id -> ChannelSpec
    pub channels: BTreeMap<String, ChannelSpec>,
    /// Map of playbook id -> Playbook
    pub playbooks: BTreeMap<String, Playbook>,
}

impl BrandSet {
    /// Load all brands, channels, and playbooks via the workspace's ScopeChain.
    pub fn load(ws: &Workspace) -> Result<Self> {
        let scope = ScopeChain::new(ws);
        Self::load_with_scope(&scope)
    }

    /// Load using a pre-configured ScopeChain (useful for tests with custom AppPaths).
    pub fn load_with_scope(scope: &ScopeChain) -> Result<Self> {
        let mut brand_set = BrandSet::default();

        // 1. Resolve brands (`brands/<slug>/`)
        for item in scope.resolve("brands") {
            if item.path.is_dir() {
                let brand_yaml_path = item.path.join("brand.yaml");
                if brand_yaml_path.exists() {
                    let content = fs::read_to_string(&brand_yaml_path)?;
                    let spec: BrandSpec = serde_yaml::from_str(&content).map_err(|e| {
                        BrandError::Validation(format!(
                            "Invalid brand.yaml in {:?}: {e}",
                            item.path
                        ))
                    })?;

                    // Load voice.md if present
                    let voice_path = item.path.join(&spec.voice);
                    let voice_text = if voice_path.exists() {
                        fs::read_to_string(&voice_path).ok()
                    } else {
                        None
                    };

                    // Compute sha256 version of the brand folder contents
                    let version = compute_folder_hash(&item.path)?;

                    brand_set.brands.insert(
                        item.id.clone(),
                        LoadedBrand {
                            slug: item.id,
                            dir: item.path,
                            spec,
                            voice_text,
                            version,
                            layer: item.layer,
                        },
                    );
                }
            }
        }

        // 2. Resolve channels (`channels/<id>.yaml`)
        for item in scope.resolve("channels") {
            if item.path.is_file() {
                let content = fs::read_to_string(&item.path)?;
                let spec: ChannelSpec = serde_yaml::from_str(&content).map_err(|e| {
                    BrandError::Validation(format!("Invalid channel YAML in {:?}: {e}", item.path))
                })?;
                brand_set.channels.insert(spec.id.clone(), spec);
            }
        }

        // 3. Resolve playbooks (`playbooks/<id>.md`)
        for item in scope.resolve("playbooks") {
            if item.path.is_file() {
                let content = fs::read_to_string(&item.path)?;
                let pb = Playbook::parse(&item.id, &content)?;
                brand_set.playbooks.insert(item.id, pb);
            }
        }

        Ok(brand_set)
    }

    /// Get brand by slug.
    pub fn get_brand(&self, slug: &str) -> Option<&LoadedBrand> {
        self.brands.get(slug)
    }

    /// Get channel by ID.
    pub fn get_channel(&self, id: &str) -> Option<&ChannelSpec> {
        self.channels.get(id)
    }

    /// Get playbook by ID.
    pub fn get_playbook(&self, id: &str) -> Option<&Playbook> {
        self.playbooks.get(id)
    }
}

/// Compute a reproducible SHA-256 hash of all files in a folder, sorted by relative path.
pub fn compute_folder_hash(folder: &Path) -> Result<String> {
    let mut files = Vec::new();

    for entry in WalkDir::new(folder)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            files.push(entry.into_path());
        }
    }

    files.sort();

    let mut hasher = Sha256::new();
    for file_path in files {
        if let Ok(rel_path) = file_path.strip_prefix(folder) {
            hasher.update(rel_path.to_string_lossy().as_bytes());
            hasher.update(b"\0");
            let data = fs::read(&file_path)?;
            hasher.update(&data);
            hasher.update(b"\0");
        }
    }

    Ok(hex::encode(hasher.finalize()))
}
