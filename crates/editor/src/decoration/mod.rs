// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::ModelContext;

use crate::content::{buffer::BufferSnapshot, edit::PreciseDelta, version::BufferVersion};

pub trait DecorationLayer {
    fn update_internal_state_with_delta(
        &mut self,
        deltas: &[PreciseDelta],
        content_version: BufferVersion,
        content: BufferSnapshot,
        ctx: &mut ModelContext<Self>,
    );
}
