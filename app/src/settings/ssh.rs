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
    ]
);
