//! Publishing subsystems: draft preparation, channel limits validation,
//! YouTube Data API v3 and Facebook Graph API publishing, OAuth PKCE / Loopback, and export bundles.

pub mod export;
pub mod facebook;
pub mod loopback;
pub mod model;
pub mod prepare;
pub mod validation;
pub mod youtube;

pub use export::export_bundle;
pub use facebook::{
    connect_facebook, map_facebook_error, publish_facebook_complete, upload_photo_to_page,
    upload_video_resumable_to_page, FacebookPageAccount, FacebookPublishParams,
    KEY_FACEBOOK_APP_ID, KEY_FACEBOOK_APP_SECRET, KEY_FACEBOOK_PAGE_TOKEN,
};
pub use model::{PublishMetadata, PublishPrivacy, PublishRequest, PublishStatus, PublishTarget};
pub use prepare::{prepare_publish, resolve_artifact_identity, PrepareOutcome};
pub use validation::{count_hashtags, find_newest_render, validate_publish, ValidationResult};
pub use youtube::{
    connect_youtube, extract_client_id_last6, fetch_my_channel, generate_pkce_codes,
    map_youtube_error, migrate_legacy_youtube_account, parse_google_client_json,
    publish_youtube_complete, refresh_access_token, resolve_google_client_credentials,
    upload_thumbnail, upload_video_resumable, YouTubeChannelAccount, YouTubeErrorDetail,
    YouTubePublishParams, KEY_YOUTUBE_CLIENT_ID, KEY_YOUTUBE_CLIENT_SECRET,
    KEY_YOUTUBE_REFRESH_TOKEN,
};
