// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::*;

#[test]
fn test_parse_valid_app_id() {
    let app_id_string = "com.example.App";
    let app_id = AppId::parse(app_id_string).expect("should not fail to parse");
    assert_eq!(app_id.qualifier(), "com");
    assert_eq!(app_id.organization(), "example");
    assert_eq!(app_id.application_name(), "App");
    assert_eq!(app_id_string, &app_id.to_string());
}

#[test]
fn test_parse_invalid_app_id() {
    assert!(
        AppId::parse("com.example").is_err(),
        "should fail to parse two-part app ID string"
    );
    assert!(
        AppId::parse("com.example.App.Blah").is_err(),
        "should fail to parse four-part app ID string"
    );
}
