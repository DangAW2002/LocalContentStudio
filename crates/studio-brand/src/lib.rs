//! `studio-brand` crate — Brand, Channel, Playbook schemas, ScopeChain loader,
//! and verification checkers for Local Content Studio.

pub mod checkers;
pub mod error;
pub mod loader;
pub mod schema;

pub use checkers::{
    run_checks, CaptionsRequiredChecker, CheckInput, CheckSubject, Checker, CheckerRegistry,
    ContrastChecker, DimensionsChecker, DurationChecker, Finding, LogoPresenceChecker,
    PaletteChecker, Report, Rgb, SafeZoneChecker, Severity, TextLimitsChecker, VoiceWordsChecker,
};
pub use error::{BrandError, Result};
pub use loader::{compute_folder_hash, BrandSet, LoadedBrand};
pub use schema::{
    export_json_schemas, BrandColors, BrandFonts, BrandLogos, BrandMotion, BrandSpec, ChannelSpec,
    FileConstraints, FormatSpec, Playbook, PlaybookFrontmatter, PlaybookStep, SafeZone, TextLimits,
    VoiceWords,
};
