// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: added settings for SSH keepalives and for opening splits of an SSH pane on the same host.

use settings::{
    macros::define_settings_group, RespectUserSyncSetting, SupportedPlatforms, SyncToCloud,
};

define_settings_group!(SshSettings,
    settings: [
        enable_legacy_ssh_wrapper: EnableSshWrapper {
            type: bool,
            default: true,
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
            private: false,
            storage_key: "EnableSSHWrapper",
            toml_path: "warpify.ssh.enable_legacy_ssh_wrapper",
            description: "Whether the legacy SSH wrapper is enabled for SSH sessions.",
        },
        enable_ssh_auto_discovery: EnableSshAutoDiscovery {
            type: bool,
            default: true,
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
            private: false,
            storage_key: "EnableSSHAutoDiscovery",
            toml_path: "warpify.ssh.enable_ssh_auto_discovery",
            description: "Whether to auto-discover SSH hosts from ~/.ssh/config.",
        },
        default_keepalive: DefaultSshKeepalive {
            type: bool,
            default: true,
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
            private: false,
            storage_key: "DefaultSshKeepalive",
            toml_path: "warpify.ssh.default_keepalive",
            description: "Whether the SSH wrapper sends keepalives (ServerAliveInterval=30, ServerAliveCountMax=6) on connections whose SSH configuration leaves ServerAliveInterval at 0, so that idle sessions survive NAT and firewall timeouts. A keepalive interval set in your SSH configuration or with -o is kept.",
        },
        split_pane_on_same_host: SplitPaneOnSameSshHost {
            type: bool,
            default: true,
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
            private: false,
            storage_key: "SplitPaneOnSameSshHost",
            toml_path: "warpify.ssh.split_pane_on_same_host",
            description: "Whether splitting a pane whose shell is in an SSH session opens the new pane on the same host, by running the same ssh command (or connecting through the SSH manager with the server's saved credentials) and then changing to the same directory there. Splitting with a chosen shell always opens a local shell.",
        },
    ]
);
