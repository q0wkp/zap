// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::*;

#[test]
fn ui_element_style_merge_test() {
    let style1 = UiComponentStyles {
        width: Some(24.),
        ..Default::default()
    };
    let style2 = UiComponentStyles {
        width: Some(25.),
        font_size: Some(14.),
        ..Default::default()
    };
    assert_eq!(style2, style1.merge(style2));

    let style3 = UiComponentStyles {
        font_size: Some(14.),
        ..Default::default()
    };
    let style4 = UiComponentStyles {
        width: Some(24.),
        font_size: Some(14.),
        ..Default::default()
    };
    assert_eq!(style4, style1.merge(style3));
}
