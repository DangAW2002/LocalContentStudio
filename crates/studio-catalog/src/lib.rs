//! studio-catalog — Skills, modules, kits catalog and plugin materializer.
//!
//! See ADR-010 for architectural decisions.

pub mod check;
pub mod error;
pub mod fetch;
pub mod inventory;
pub mod materialize;
pub mod model;
pub mod resolve;
pub mod suggest;

pub use check::{check, Report};
pub use error::CatalogError;
pub use fetch::{fetch, update, SourceDiff};
pub use inventory::{frontmatter, Inventory, RepoInventory, SkillInventory};
pub use materialize::{materialize_plugin, MaterializeOptions};
pub use model::{Catalog, Kit, Module, SkillSelection, Source};
pub use resolve::{resolve, ActiveSet, ResolvedSkill};
pub use suggest::suggest_kits;

/// Returns the global list of commands denied in Studio turns.
pub fn denied_commands() -> Vec<String> {
    vec![
        "npx skills add".to_string(),
        "npx skills install".to_string(),
    ]
}
