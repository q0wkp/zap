// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};
use zap_ng_ui::platform::linux;

define_settings_group!(LinuxAppConfiguration,
    settings: [
        force_x11: ForceX11 {
            type: bool,
            // Default to true on WSL and false on all other platforms.
            default: linux::is_wsl(),
            supported_platforms: SupportedPlatforms::LINUX,
            sync_to_cloud: SyncToCloud::Never,
            private: false,
            toml_path: "system.force_x11",
            description: "Whether to force X11 instead of Wayland on Linux.",
        },
    ]
);
