// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: removed the Windows high-performance GPU defaults; renamed the Warp and Zap names to ZapNG.

use settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};
use zap_ng_ui::platform::GraphicsBackend;

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
