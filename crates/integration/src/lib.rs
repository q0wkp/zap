// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

mod builder;
mod step;

pub mod test;
pub mod user_defaults;
pub mod util;

pub use builder::Builder;
pub use zap_ng::integration_testing::view_getters;
pub use zap_ng_ui::integration::TestStep;
