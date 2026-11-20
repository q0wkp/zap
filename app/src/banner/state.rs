// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use serde::{Deserialize, Serialize};
use zap_ng_ui::Entity;

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(rename_all = "snake_case")]
pub enum BannerState {
    /// The banner is not currently visible and has not yet been seen by the user.
    #[default]
    NotDismissed,

    // The banner is open.
    Open,

    /// The banner was dismissed by the user.
    Dismissed,
}

impl Entity for BannerState {
    type Event = ();
}
