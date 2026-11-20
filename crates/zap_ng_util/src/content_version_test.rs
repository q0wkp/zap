// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::*;

#[test]
fn test_create_version() {
    ContentVersion::new();
}

#[test]
fn test_versions_equal() {
    let version1 = ContentVersion::new();
    let version2 = version1;

    assert_eq!(version1, version2);
}

#[test]
fn test_versions_not_equal() {
    let version1 = ContentVersion::new();
    let version2 = ContentVersion::new();

    assert_ne!(version1, version2);
}
