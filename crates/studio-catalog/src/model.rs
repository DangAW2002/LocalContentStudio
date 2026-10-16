//! Data model for catalog.yaml and multi-layer merging.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::CatalogError;

/// Full catalog schema representing `catalog.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Catalog {
    #[serde(default)]
    pub schema: Option<String>,

    #[serde(default)]
    pub updated: Option<String>,

    #[serde(default)]
    pub sources: BTreeMap<String, Source>,

    #[serde(default)]
    pub excluded: BTreeMap<String, BTreeMap<String, String>>,

    #[serde(default)]
    pub adapt: BTreeMap<String, String>,

    #[serde(default)]
    pub denied_commands: Vec<String>,

    #[serde(default)]
    pub modules: BTreeMap<String, Module>,

    #[serde(default)]
    pub kits: BTreeMap<String, Kit>,
}

/// A third-party source repository definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Source {
    pub commit: String,

    #[serde(default)]
    pub license: Option<String>,

    #[serde(default)]
    pub kind: Option<String>,

    #[serde(default)]
    pub sparse: Vec<String>,
}

/// Module skills map: repo -> list of skill names.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModuleSkills(pub BTreeMap<String, Vec<String>>);

impl<'de> Deserialize<'de> for ModuleSkills {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Map(BTreeMap<String, Vec<String>>),
            List(serde::de::IgnoredAny),
        }

        match Repr::deserialize(deserializer)? {
            Repr::Map(m) => Ok(ModuleSkills(m)),
            Repr::List(_) => Ok(ModuleSkills(BTreeMap::new())),
        }
    }
}

impl Serialize for ModuleSkills {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl std::ops::Deref for ModuleSkills {
    type Target = BTreeMap<String, Vec<String>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ModuleSkills {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> IntoIterator for &'a ModuleSkills {
    type Item = (&'a String, &'a Vec<String>);
    type IntoIter = std::collections::btree_map::Iter<'a, String, Vec<String>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut ModuleSkills {
    type Item = (&'a String, &'a mut Vec<String>);
    type IntoIter = std::collections::btree_map::IterMut<'a, String, Vec<String>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl IntoIterator for ModuleSkills {
    type Item = (String, Vec<String>);
    type IntoIter = std::collections::btree_map::IntoIter<String, Vec<String>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// A module declaration grouping units and skills.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Module {
    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub always: bool,

    #[serde(default)]
    pub optional: bool,

    #[serde(default)]
    pub level: Option<serde_yaml::Value>,

    #[serde(default)]
    pub needs: Vec<String>,

    #[serde(default)]
    pub requires: Vec<String>,

    #[serde(default)]
    pub router: Option<String>,

    #[serde(default)]
    pub mcp: Option<String>,

    #[serde(default)]
    pub skills: ModuleSkills,

    #[serde(default)]
    pub skill_requires: BTreeMap<String, Vec<String>>,
}

/// Skill selection in a kit: either "all" or explicit list of skill names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SkillSelection {
    All(String),
    Skills(Vec<String>),
}

impl SkillSelection {
    /// Returns true if this selection represents "all".
    pub fn is_all(&self) -> bool {
        match self {
            Self::All(s) => s == "all",
            Self::Skills(_) => false,
        }
    }

    /// Returns the explicit slice of skill names if specified.
    pub fn as_slice(&self) -> Option<&[String]> {
        match self {
            Self::All(_) => None,
            Self::Skills(vec) => Some(vec.as_slice()),
        }
    }
}

/// A kit definition grouping modules for a project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Kit {
    pub title: String,
    pub keywords: Vec<String>,
    pub use_modules: BTreeMap<String, SkillSelection>,
}

// Serialization / Deserialization proxy for Kit so `use` key in YAML maps to `use_modules`.
#[derive(Serialize, Deserialize)]
struct KitProxy {
    title: String,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default, rename = "use")]
    use_modules: BTreeMap<String, SkillSelection>,
}

impl From<KitProxy> for Kit {
    fn from(p: KitProxy) -> Self {
        Self {
            title: p.title,
            keywords: p.keywords,
            use_modules: p.use_modules,
        }
    }
}

impl From<Kit> for KitProxy {
    fn from(k: Kit) -> Self {
        Self {
            title: k.title,
            keywords: k.keywords,
            use_modules: k.use_modules,
        }
    }
}

// Override Kit serde
impl Serialize for Kit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        KitProxy::from(self.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Kit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        KitProxy::deserialize(deserializer).map(Into::into)
    }
}

impl Catalog {
    /// Parse a catalog from a YAML string.
    pub fn from_yaml(s: &str) -> Result<Self, CatalogError> {
        let cat: Catalog = serde_yaml::from_str(s)?;
        Ok(cat)
    }

    /// Load a catalog from a single file path.
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<Self, CatalogError> {
        let p = path.as_ref();
        let content = fs::read_to_string(p).map_err(|e| CatalogError::Io {
            path: p.to_path_buf(),
            source: e,
        })?;
        Self::from_yaml(&content)
    }

    /// Merge multiple catalog layers: app -> org -> workspace.
    /// Later layers override earlier layers by key.
    pub fn load_layers<P: AsRef<Path>>(paths: &[P]) -> Result<Self, CatalogError> {
        let mut merged = Catalog::default();

        for path in paths {
            let p = path.as_ref();
            if !p.exists() {
                continue;
            }
            let layer = Self::load_file(p)?;
            merged.merge_layer(layer);
        }

        Ok(merged)
    }

    /// Merge another layer into self, overriding existing keys.
    pub fn merge_layer(&mut self, other: Catalog) {
        if other.schema.is_some() {
            self.schema = other.schema;
        }
        if other.updated.is_some() {
            self.updated = other.updated;
        }

        // Sources: merge by key
        for (k, v) in other.sources {
            self.sources.insert(k, v);
        }

        // Excluded: merge sub-maps
        for (repo, skills) in other.excluded {
            let entry = self.excluded.entry(repo).or_default();
            for (skill, reason) in skills {
                entry.insert(skill, reason);
            }
        }

        // Adapt: merge by key
        for (k, v) in other.adapt {
            self.adapt.insert(k, v);
        }

        // Denied commands: union / replace
        for cmd in other.denied_commands {
            if !self.denied_commands.contains(&cmd) {
                self.denied_commands.push(cmd);
            }
        }

        // Modules: merge by module id
        for (k, v) in other.modules {
            self.modules.insert(k, v);
        }

        // Kits: merge by kit id
        for (k, v) in other.kits {
            self.kits.insert(k, v);
        }
    }
}
