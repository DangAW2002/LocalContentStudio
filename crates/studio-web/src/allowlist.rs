//! Allowlist and security categorization of invoke commands exposed over the web interface.

use std::collections::HashSet;
use std::sync::OnceLock;

/// Commands that perform destructive changes or outward actions requiring recent sudo authorization (within 10 min).
pub const SUDO_COMMANDS: &[&str] = &[
    "publish_confirm",
    "tool_install_confirm",
    "secret_set",
    "secret_delete",
    "checkpoint_restore",
    "turn_undo",
    // Opening/creating a workspace can point Studio at any folder on disk.
    "workspace_create",
    "workspace_open",
    // Changing an endpoint changes where API keys are sent.
    "provider_endpoint_set",
    "tool_uninstall",
    "youtube_client_import",
    "youtube_channel_disconnect",
    // Owner password / LAN exposure.
    "web_configure",
    // TTS model/binary download executes binary downloads
    "tts_model_install",
    // Deletes a library/project asset file.
    "asset_delete",
    // Overwrites a pinned artifact with a checkpoint copy.
    "restore_pinned_artifact",
];

/// Commands allowed to be invoked via `POST /api/invoke/<command>`.
/// Anything not in this list is refused with an access denied error.
pub const ALLOWED_WEB_COMMANDS: &[&str] = &[
    // Workspace management
    "app_recent_workspaces",
    "workspace_create",
    "workspace_open",
    "workspace_current",
    // Folder listing and creation for the web workspace picker (no native dialog in a browser).
    "fs_list_dirs",
    "fs_create_dir",
    // Content graph (ADR-011), series and reference profiles
    "graph_get",
    "graph_link",
    "graph_unlink",
    "graph_set_pinned",
    "graph_set_notes",
    "graph_create_variant",
    "graph_make_reference",
    "graph_update_downstream_plan",
    "graph_update_downstream_run",
    "restore_pinned_artifact",
    "series_list",
    "series_get",
    "series_create",
    "series_create_episode",
    // Brand / channel / playbook scopes
    "scope_create",
    "scope_override",
    "scope_remove_override",
    "scope_validate",
    // Library
    "asset_delete",
    // Projects and file operations
    "project_list",
    "project_create",
    "project_files",
    "file_read",
    "file_write",
    // Threads and turns
    "thread_list",
    "thread_create",
    "thread_set_model",
    "thread_events",
    "budget_get",
    "budget_set",
    "turn_start",
    "turn_cancel",
    "turn_undo",
    "permission_respond",
    "question_respond",
    "permission_pending",
    // Checkpoints
    "checkpoint_list",
    "checkpoint_create",
    "checkpoint_restore",
    // Engine models and catalog
    "engines_probe",
    "engine_models",
    "engine_settings_get",
    "engine_settings_set",
    "agy_connect",
    "agy_disconnect",
    "catalog_kits",
    "catalog_modules",
    "catalog_suggest",
    "project_modules_set",
    // Scopes and assets
    "scope_list",
    "asset_list",
    // Jobs and disk usage
    "jobs_snapshot",
    "disk_usage",
    // Artifact rendering and checks
    "artifact_list",
    "artifact_render",
    "artifact_check",
    "artifact_channel_matrix",
    // Secrets status and configuration (write-only / sudo-gated)
    "secret_status",
    "secret_set",
    "secret_delete",
    // Provider endpoints and pricing
    "provider_endpoint_get",
    "provider_endpoint_set",
    "provider_pricing_get",
    "provider_pricing_set",
    // Tools & environment probes
    "env_probe",
    "tool_install_preview",
    "tool_install_confirm",
    "tool_list_installed",
    "tool_uninstall",
    // Text-to-Speech (TTS)
    "tts_status",
    "tts_model_preview",
    "tts_model_install",
    "tts_synthesize_preview",
    "tts_get_config",
    "tts_set_config",
    // Publishing workflow
    "publish_prepare",
    "publish_list",
    "publish_get",
    "publish_update_metadata",
    "publish_confirm",
    "publish_cancel",
    // YouTube publishing integration
    "youtube_client_import",
    "youtube_client_status",
    "youtube_channels_list",
    "youtube_channel_disconnect",
    // Web status and configuration
    "web_status",
    "web_configure",
];

static ALLOWED_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
static SUDO_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();

/// Check if a command is in the web invoke allowlist.
pub fn is_command_allowed(command: &str) -> bool {
    let set = ALLOWED_SET.get_or_init(|| ALLOWED_WEB_COMMANDS.iter().copied().collect());
    set.contains(command)
}

/// Check if a command requires recent sudo re-authentication.
pub fn requires_sudo(command: &str) -> bool {
    let set = SUDO_SET.get_or_init(|| SUDO_COMMANDS.iter().copied().collect());
    set.contains(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowlist_membership() {
        assert!(is_command_allowed("project_list"));
        assert!(is_command_allowed("turn_start"));
        assert!(is_command_allowed("publish_confirm"));
        assert!(is_command_allowed("secret_set"));
        assert!(is_command_allowed("engine_settings_set"));
        assert!(is_command_allowed("agy_connect"));
        assert!(is_command_allowed("agy_disconnect"));
        // The web workspace picker browses server folders; listing and sub-dir creation.
        assert!(is_command_allowed("fs_list_dirs"));
        assert!(!requires_sudo("fs_list_dirs"));
        assert!(is_command_allowed("fs_create_dir"));
        assert!(!requires_sudo("fs_create_dir"));
        for c in [
            "web_configure",
            "workspace_open",
            "provider_endpoint_set",
            "publish_confirm",
        ] {
            assert!(requires_sudo(c), "{c} must require sudo");
        }

        // Arbitrary / OS-level or unknown commands must be refused
        assert!(!is_command_allowed("os_exec"));
        assert!(!is_command_allowed("system_format"));
        assert!(!is_command_allowed("random_nonexistent_command"));
    }

    #[test]
    fn test_sudo_classification() {
        assert!(requires_sudo("publish_confirm"));
        assert!(requires_sudo("tool_install_confirm"));
        assert!(requires_sudo("secret_set"));
        assert!(requires_sudo("secret_delete"));
        assert!(requires_sudo("checkpoint_restore"));
        assert!(requires_sudo("turn_undo"));

        // Read-only or safe commands do not require sudo
        assert!(!requires_sudo("project_list"));
        assert!(!requires_sudo("file_read"));
        assert!(!requires_sudo("turn_start"));
        assert!(!requires_sudo("secret_status"));
    }

    #[test]
    fn test_tts_commands_allowlist_and_sudo() {
        for cmd in [
            "tts_status",
            "tts_model_preview",
            "tts_model_install",
            "tts_synthesize_preview",
            "tts_get_config",
            "tts_set_config",
        ] {
            assert!(
                is_command_allowed(cmd),
                "{cmd} must be in allowed web commands"
            );
        }

        assert!(
            requires_sudo("tts_model_install"),
            "tts_model_install downloads executables and must require sudo"
        );

        for cmd in [
            "tts_status",
            "tts_model_preview",
            "tts_synthesize_preview",
            "tts_get_config",
            "tts_set_config",
        ] {
            assert!(!requires_sudo(cmd), "{cmd} must not require sudo");
        }
    }
}
