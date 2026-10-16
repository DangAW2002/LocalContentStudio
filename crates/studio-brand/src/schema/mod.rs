pub mod brand;
pub mod channel;
pub mod playbook;

pub use brand::{BrandColors, BrandFonts, BrandLogos, BrandMotion, BrandSpec, VoiceWords};
pub use channel::{ChannelSpec, FileConstraints, FormatSpec, SafeZone, TextLimits};
pub use playbook::{Playbook, PlaybookFrontmatter, PlaybookStep};

/// Export JSON Schema for all brand@1 and channel@1 specs.
pub fn export_json_schemas() -> serde_json::Value {
    let brand_schema = schemars::schema_for!(BrandSpec);
    let channel_schema = schemars::schema_for!(ChannelSpec);
    let playbook_schema = schemars::schema_for!(PlaybookFrontmatter);

    serde_json::json!({
        "brand@1": brand_schema,
        "channel@1": channel_schema,
        "playbook@1": playbook_schema,
    })
}
