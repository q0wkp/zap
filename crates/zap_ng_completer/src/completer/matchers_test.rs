// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::completer::matchers::match_type_for_case_insensitive;

use super::{Match, MatchStrategy};

#[test]
fn test_match_type_for_case_insensitive() {
    assert_eq!(
        match_type_for_case_insensitive("git", "git"),
        Some(Match::Exact {
            is_case_sensitive: true
        })
    );
    assert_eq!(
        match_type_for_case_insensitive("gIt", "git"),
        Some(Match::Exact {
            is_case_sensitive: false
        })
    );
    assert_eq!(
        match_type_for_case_insensitive("abc", "abcdef"),
        Some(Match::Prefix {
            is_case_sensitive: true
        })
    );
    assert_eq!(
        match_type_for_case_insensitive("aBc", "abcdef"),
        Some(Match::Prefix {
            is_case_sensitive: false
        })
    );
    assert_eq!(match_type_for_case_insensitive("abc", "def"), None);
}

#[test]
fn test_get_match_type_case_sensitive() {
    let matcher = MatchStrategy::CaseSensitive;

    assert_eq!(matcher.get_match_type("git", "GIT"), None);
    assert_eq!(
        matcher.get_match_type("git", "git"),
        Some(Match::Exact {
            is_case_sensitive: true
        })
    );
    assert_eq!(
        matcher.get_match_type("AsDs", "AsDss"),
        Some(Match::Prefix {
            is_case_sensitive: true
        })
    );
    assert_eq!(matcher.get_match_type("Asds", "asds"), None);
}

#[test]
fn test_get_match_type_case_insensitive() {
    let matcher = MatchStrategy::CaseInsensitive;

    assert_eq!(
        matcher.get_match_type("git", "GIT"),
        Some(Match::Exact {
            is_case_sensitive: false
        })
    );
    assert_eq!(
        matcher.get_match_type("AsDs", "asdss"),
        Some(Match::Prefix {
            is_case_sensitive: false
        })
    );
    assert_eq!(matcher.get_match_type("Asd", "ads"), None);
}

#[test]
fn test_get_match_type_fuzzy() {
    let matcher = MatchStrategy::Fuzzy;

    assert_eq!(
        matcher.get_match_type("git", "GIT"),
        Some(Match::Exact {
            is_case_sensitive: false
        })
    );
    assert_eq!(
        matcher.get_match_type("AsDs", "asdss"),
        Some(Match::Prefix {
            is_case_sensitive: false
        })
    );
    assert!(matches!(
        matcher.get_match_type("abc", "aabac"),
        Some(Match::Fuzzy { .. })
    ));
    assert_eq!(matcher.get_match_type("abc", "xyz"), None);
}
