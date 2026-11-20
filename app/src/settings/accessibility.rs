// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use settings::{
    macros::define_settings_group, RespectUserSyncSetting, SupportedPlatforms, SyncToCloud,
};
use zap_ng_ui::accessibility::AccessibilityVerbosity;

define_settings_group!(AccessibilitySettings, settings: [
    a11y_verbosity: AccessibilityVerbosityState {
        type: AccessibilityVerbosity,
        default: AccessibilityVerbosity::default(),
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        storage_key: "AccessibilityVerbosity",
        toml_path: "accessibility.accessibility_verbosity",
        description: "The verbosity level for screen reader announcements.",
    }
]);
