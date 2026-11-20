// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

#[cfg(native)]
#[cfg_attr(not(macos), allow(dead_code))]
pub mod font_kit;

#[cfg(test)]
#[path = "text_layout_test.rs"]
mod text_layout_tests;

pub use zap_ng_ui_core::fonts::*;

#[cfg(test)]
pub(crate) use text_layout_tests::{collect_glyph_indices, init_fonts};

#[cfg(all(test, target_os = "macos"))]
pub(crate) use text_layout_tests::collect_line_caret_position_starts;
