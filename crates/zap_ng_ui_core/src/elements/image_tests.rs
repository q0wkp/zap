// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::*;

#[test]
fn image_rect_returns_none_for_nan_origin() {
    assert!(image_rect(
        vec2f(164.0, 164.0),
        vec2f(f32::NAN, 874.725),
        vec2f(163.75, 163.75),
        false,
        false,
    )
    .is_none());
}
