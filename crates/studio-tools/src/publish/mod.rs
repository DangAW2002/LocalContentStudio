//! Publishing subsystems: draft preparation, channel limits validation,
//! YouTube Data API v3 resumable upload, OAuth PKCE, and export bundles.

pub mod export;
pub mod model;
pub mod prepare;
pub mod validation;
pub mod youtube;

pub use export::export_bundle;
pub use model::{PublishMetadata, PublishPrivacy, PublishRequest, PublishStatus, PublishTarget};
pub use prepare::{prepare_publish, resolve_artifact_identity, PrepareOutcome};
pub use validation::{count_hashtags, find_newest_render, validate_publish, ValidationResult};
pub use youtube::{
    connect_youtube, generate_pkce_codes, publish_youtube_complete, refresh_access_token,
    upload_thumbnail, upload_video_resumable, KEY_YOUTUBE_CLIENT_ID, KEY_YOUTUBE_CLIENT_SECRET,
    KEY_YOUTUBE_REFRESH_TOKEN,
};
