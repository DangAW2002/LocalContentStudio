//! `studio-tools` — Capability contracts, tool registry, dependency probes,
//! scope-based provider routing, Credential Manager secrets, and supervised execution.

pub mod error;
pub mod install;
pub mod manifest;
pub mod pricing;
pub mod probe;
pub mod providers;
pub mod publish;
pub mod registry;
pub mod router;
pub mod runner;
pub mod secret;
pub mod setup;
pub mod tts;

pub use error::{Result, ToolError};
pub use install::{
    confirm_install, extract_zip_safely, list_tools_with_probe, preview_install, uninstall_tool,
    InstalledToolInfo, ToolInstallPreview, ToolInstallScope, MAX_ZIP_BYTES,
};
pub use manifest::{
    CapabilitySchema, SideEffect, ToolCost, ToolKind, ToolManifest, ToolRunSpec, ToolRunType,
};
pub use pricing::{ProviderPricing, DEFAULT_IMAGE_USD, DEFAULT_VIDEO_USD_PER_S};
pub use probe::{find_in_path, probe, probe_application, probe_binary, ProbeResult};
pub use providers::import::probe_ytdlp;
pub use providers::piper::{
    build_piper_args, build_piper_args_full, find_piper_binary, get_installed_piper_voices,
    install_piper_binary, install_piper_voice, piper_models_dir, probe_piper, PiperCliOptions,
    PiperVoiceMeta, PIPER_HF_BASE_URL, PIPER_RELEASE_VERSION, PIPER_VIETNAMESE_VOICES,
    PIPER_ZIP_SHA256, PIPER_ZIP_SIZE_BYTES, PIPER_ZIP_URL, VIETNAMESE_VOICE_LICENSE_NOTE,
};
pub use providers::vieneu::{
    build_vieneu_args, ensure_helper_script, probe_vieneu, VieneuVoiceConfig,
    VIENEU_DEFAULT_VOICES, VIENEU_HELPER_SCRIPT,
};
pub use registry::{ManifestProblem, Registry};
pub use router::{route, route_available};
pub use runner::{run_tool, ToolRun, ToolRunContext, MAX_INLINE_RESULT_BYTES};
pub use secret::{
    format_target_name, CredentialManagerStore, FileSecretStore, MemoryStore, SecretStore,
};
pub use setup::{default_agy_settings_path, remove_agy, remove_agy_at, setup_agy, setup_agy_at};
pub use tts::{
    compute_tts_cache_key, concat_wav_files_with_silence, normalize_vietnamese_text, read_pcm_wav,
    split_into_tts_segments, write_pcm_wav, PauseConfig, TtsSegment, WavSpec,
};
