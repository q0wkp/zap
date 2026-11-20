// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

pub mod conversations;
mod data_sources;
mod files;
mod filter_chip_renderer;
pub mod launch_config;
pub mod mixer;
pub mod navigation;
#[cfg_attr(not(feature = "local_tty"), allow(dead_code))]
pub mod new_session;
pub mod render_util;
mod selected_items;
pub mod separator_search_item;
pub mod ssh_servers;
pub mod view;
pub mod zap_ng_drive;
mod zero_state;

use filter_chip_renderer::FilterChipRenderer;
pub use mixer::{CommandPaletteMixer, ItemSummary};
pub use selected_items::SelectedItems;
pub use view::View;

pub mod styles {
    pub const SEARCH_ITEM_TEXT_PADDING: f32 = 4.;
}
