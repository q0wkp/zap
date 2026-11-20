// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

pub use zap_ng_features::*;

use zap_ng_ui::platform::menu::{CustomMenuItem, MenuItem, MenuItemPropertyChanges};
fn feature_flag_menu_item(flag: FeatureFlag) -> MenuItem {
    MenuItem::Custom(CustomMenuItem::new(
        &format!("{flag:?}"),
        move |_| {
            // toggling the flag
            flag.set_enabled(!flag.is_enabled())
        },
        move |_props, _ctx| MenuItemPropertyChanges {
            checked: Some(flag.is_enabled()),
            ..Default::default()
        },
        None,
    ))
}

pub fn runtime_flags_menu_items() -> Vec<MenuItem> {
    if !FeatureFlag::RuntimeFeatureFlags.is_enabled() {
        return Vec::new();
    }

    RUNTIME_FEATURE_FLAGS
        .iter()
        .map(|flag| feature_flag_menu_item(*flag))
        .collect()
}
