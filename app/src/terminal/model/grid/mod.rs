// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

mod displayed_output;
pub mod grid_handler;
mod grid_storage;
mod indexing;
mod selection_cursor;
mod storage;

pub(super) mod grapheme_cursor;
#[cfg(test)]
mod tests;

pub use zap_ng_terminal::model::grid::row;

pub use displayed_output::RespectDisplayedOutput;
pub use grid_storage::*;
pub(super) use indexing::ConvertToAbsolute;
pub use indexing::IndexRegion;
pub use selection_cursor::SelectionCursor;

enum CursorDirection {
    Up,
    Down,
    Left,
    Right,
}

enum CursorState {
    Valid,
    Exhausted(CursorDirection),
    Invalid,
}
