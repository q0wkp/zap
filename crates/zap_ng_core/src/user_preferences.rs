// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: dropped the Windows-only code; renamed the Warp and Zap names to ZapNG.

//! Extension trait for accessing the private user preferences backend.
//!
//! Public settings are accessed exclusively through the settings macros
//! (`define_settings_group!`) and the `settings::PublicPreferences` wrapper,
//! which restricts direct access at compile time.

use std::ops::Deref;

use settings::PrivatePreferences;
use zap_ng_ui::SingletonEntity;
use zap_ng_ui_extras::user_preferences::UserPreferences;

/// An extension trait on [`zap_ng_ui::AppContext`] for accessing private user
/// preferences.
///
/// Private settings are always stored in the platform-native store (e.g.
/// UserDefaults on macOS, JSON file on Linux) and never
/// appear in the user-visible settings file.
pub trait GetUserPreferences {
    /// Returns the preferences backend for private settings.
    ///
    /// Private settings are always stored in the platform-native store and never
    /// appear in the user-visible settings file.
    fn private_user_preferences(&self) -> &dyn UserPreferences;
}

impl GetUserPreferences for zap_ng_ui::AppContext {
    fn private_user_preferences(&self) -> &dyn UserPreferences {
        <PrivatePreferences as SingletonEntity>::as_ref(self).deref()
    }
}
