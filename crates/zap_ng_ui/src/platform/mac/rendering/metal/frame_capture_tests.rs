// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::convert_bgra_to_rgba;

#[test]
fn test_convert_bgra_to_rgba() {
    let mut data = vec![
        0xBB, 0xCC, 0xFF, 0xAA, // BGRA pixel (Blue, Green, Red, Alpha)
        0x11, 0x22, 0x33, 0x44, // Another BGRA pixel
    ];

    convert_bgra_to_rgba(&mut data);

    // After conversion, should be RGBA (Red, Green, Blue, Alpha)
    assert_eq!(
        data,
        vec![
            0xFF, 0xCC, 0xBB, 0xAA, // RGBA pixel
            0x33, 0x22, 0x11, 0x44, // Another RGBA pixel
        ]
    );
}
