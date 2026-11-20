//! The settings ZapNG has and Warp doesn't are kept in a file of their own, `zap-ng.toml`, next
//! to `settings.toml`, which so holds only what Warp's own settings file holds.
//!
//! A setting is still defined like any other, with a `toml_path`. Listing that path in
//! [`ZAP_NG_SETTINGS`] moves the setting to `zap-ng.toml`, under the path given beside it.

use std::path::PathBuf;

use zap_ng_ui_extras::user_preferences::split_toml::Route;

/// The settings kept in `zap-ng.toml`: the `toml_path` each is defined with, and its path in
/// that file.
pub const ZAP_NG_SETTINGS: &[Route] = &[
    // Model providers and the requests sent to them.
    ("agents.warp_agent.providers", "providers"),
    ("agents.byop.last_used_model_id", "byop.last_used_model_id"),
    (
        "agents.byop.last_used_reasoning",
        "byop.last_used_reasoning",
    ),
    (
        "agents.byop.models_dev.download",
        "byop.models_dev.download",
    ),
    ("agents.byop.models_dev.url", "byop.models_dev.url"),
    ("agents.byop_compaction.auto", "byop.compaction.auto"),
    (
        "agents.byop_compaction.model.model_id",
        "byop.compaction.model.model_id",
    ),
    (
        "agents.byop_compaction.model.provider_id",
        "byop.compaction.model.provider_id",
    ),
    (
        "agents.byop_compaction.preserve_recent_tokens",
        "byop.compaction.preserve_recent_tokens",
    ),
    ("agents.byop_compaction.prune", "byop.compaction.prune"),
    (
        "agents.byop_compaction.reserved",
        "byop.compaction.reserved",
    ),
    (
        "agents.byop_compaction.tail_turns",
        "byop.compaction.tail_turns",
    ),
    // The agent's interface.
    (
        "agents.warp_agent.appearance.hide_completed_tool_cards",
        "agent.appearance.hide_completed_tool_cards",
    ),
    (
        "agents.warp_agent.input.show_zero_state_hints",
        "agent.input.show_zero_state_hints",
    ),
    (
        "general.persist_conversations",
        "agent.persist_conversations",
    ),
    // Other vendors' CLI agents.
    ("agents.third_party.default_cli_agent", "cli_agents.default"),
    ("agents.third_party.per_agent", "cli_agents.per_agent"),
    // Appearance.
    ("appearance.language", "appearance.language"),
    (
        "appearance.text.ui_font_name",
        "appearance.text.ui_font_name",
    ),
    (
        "appearance.text.ui_font_size",
        "appearance.text.ui_font_size",
    ),
    (
        "appearance.text.fallback_font_name",
        "appearance.text.fallback_font_name",
    ),
    (
        "appearance.text.markdown_heading_h1_scale",
        "appearance.markdown.heading_h1_scale",
    ),
    (
        "appearance.text.markdown_heading_h2_scale",
        "appearance.markdown.heading_h2_scale",
    ),
    (
        "appearance.text.markdown_heading_h3_scale",
        "appearance.markdown.heading_h3_scale",
    ),
    (
        "appearance.text.markdown_heading_h4_scale",
        "appearance.markdown.heading_h4_scale",
    ),
    (
        "appearance.text.markdown_heading_h5_scale",
        "appearance.markdown.heading_h5_scale",
    ),
    (
        "appearance.text.markdown_heading_h6_scale",
        "appearance.markdown.heading_h6_scale",
    ),
    (
        "appearance.vertical_tabs.show_title_bar_search_bar",
        "appearance.vertical_tabs.show_title_bar_search_bar",
    ),
    // The terminal.
    (
        "terminal.input.multiline_command_confirmation_threshold",
        "terminal.input.multiline_command_confirmation_threshold",
    ),
    // The network proxy.
    ("network.proxy_mode", "network.proxy.mode"),
    ("network.proxy_url", "network.proxy.url"),
    ("network.proxy_username", "network.proxy.username"),
    ("network.proxy_no_proxy", "network.proxy.no_proxy"),
    // The SSH manager.
    ("warpify.ssh.default_keepalive", "ssh.default_keepalive"),
    (
        "warpify.ssh.enable_ssh_auto_discovery",
        "ssh.auto_discovery",
    ),
    (
        "warpify.ssh.split_pane_on_same_host",
        "ssh.split_pane_on_same_host",
    ),
    // Settings sync and updates.
    ("cloud_sync.auto_sync", "sync.auto_sync"),
    ("cloud_sync.sync_platform", "sync.platform"),
    (
        "updates.automatic_updates_enabled",
        "updates.automatic_updates_enabled",
    ),
];

/// The file the settings in [`ZAP_NG_SETTINGS`] are kept in.
pub fn zap_ng_settings_file_path() -> PathBuf {
    zap_ng_core::paths::config_local_dir().join("zap-ng.toml")
}
