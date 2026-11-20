// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub(super) struct GlyphIndex<T>(pub(super) T);

impl GlyphIndex<usize> {
    pub(super) fn as_f32(&self) -> GlyphIndex<f32> {
        GlyphIndex(self.0 as f32)
    }
}
