// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

pub mod ansi;
mod block_id;
mod block_index;
pub mod char_or_str;
pub mod escape_sequences;
pub mod grid;
mod indexing;
mod mode;
pub mod mouse;

pub use block_id::BlockId;
pub use block_index::BlockIndex;
pub use indexing::*;
pub use mode::{KeyboardModes, KeyboardModesApplyBehavior, TermMode};
