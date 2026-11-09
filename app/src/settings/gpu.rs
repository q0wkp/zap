use settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};
use warpui::platform::GraphicsBackend;

define_settings_group!(GPUSettings, settings: [
    prefer_low_power_gpu: PreferLowPowerGPU {
        type: bool,
        default: cfg!(any(target_os = "linux", target_os = "freebsd")),
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        toml_path: "system.prefer_low_power_gpu",
        description: "Whether to prefer the integrated (low-power) GPU.",
    },
    preferred_backend: PreferredGraphicsBackend {
        type: Option<GraphicsBackend>,
        default: None,
        supported_platforms: SupportedPlatforms::WINDOWS,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
       toml_path: "system.preferred_graphics_backend",
       description: "The preferred graphics backend on Windows.",
   },
]);
