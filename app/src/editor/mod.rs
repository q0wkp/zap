// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

pub mod accept_autosuggestion_keybinding_view;
pub mod autosuggestion_ignore_view;
mod soft_wrap;
mod view;

/// Consumers of the editor should only interface with the view.
/// They should _not_ be able to interface with the internal
/// details of the editor (e.g. the [`Buffer`]).
pub use view::*;
pub use zap_ng_ui::text::point::Point;

use std::{cmp, ops::Range};
use zap_ng_ui::AppContext;

pub fn init(app: &mut AppContext) {
    view::init(app);
}

trait RangeExt<T> {
    fn sorted(&self) -> (T, T);
}

impl<T: Ord + Clone> RangeExt<T> for Range<T> {
    fn sorted(&self) -> (T, T) {
        (
            cmp::min(&self.start, &self.end).clone(),
            cmp::max(&self.start, &self.end).clone(),
        )
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
pub mod tests;
