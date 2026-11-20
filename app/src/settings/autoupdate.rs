// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};

define_settings_group!(AutoupdateSettings, settings: [
    automatic_updates_enabled: AutomaticUpdatesEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        storage_key: "AutomaticUpdatesEnabled",
        toml_path: "updates.automatic_updates_enabled",
        description: "Whether ZapNG automatically checks for and downloads updates in the background.",
    },
]);
