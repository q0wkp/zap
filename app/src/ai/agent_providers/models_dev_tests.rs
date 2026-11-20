// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

use std::time::{Duration, SystemTime};

use super::*;

#[test]
fn bundled_catalog_parses() {
    let catalog = parse_bundled().expect("the built-in catalog should parse");
    assert!(
        catalog.len() > 50,
        "the built-in catalog looks truncated: {} providers",
        catalog.len()
    );
    let models: usize = catalog.values().map(|p| p.models.len()).sum();
    assert!(
        models > 1000,
        "the built-in catalog looks truncated: {models} models"
    );
    assert!(
        catalog.values().any(|p| p.api.is_some()),
        "provider base URLs should survive the trimming"
    );
    assert!(
        catalog
            .values()
            .flat_map(|p| p.models.values())
            .any(|m| m.limit.context > 0),
        "context windows should survive the trimming"
    );
}

#[test]
fn bundled_catalog_has_a_date() {
    assert!(
        bundled_taken_at().is_some(),
        "catalog.txt should hold taken_at"
    );
}

#[test]
fn parse_catalog_rejects_what_is_not_a_catalog() {
    assert!(parse_catalog(b"<html>captive portal</html>").is_err());
    assert!(
        parse_catalog(b"{}").is_err(),
        "an empty catalog must not replace a real one"
    );
    assert!(parse_catalog(b"[]").is_err());

    let catalog = parse_catalog(
        br#"{"deepseek": {"id": "deepseek", "name": "DeepSeek",
             "api": "https://api.deepseek.com",
             "models": {"deepseek-chat": {"id": "deepseek-chat",
                                          "limit": {"context": 128000, "output": 8192}}}}}"#,
    )
    .expect("a minimal catalog should parse");
    assert_eq!(
        catalog["deepseek"].models["deepseek-chat"].limit.context,
        128000
    );
}

#[test]
fn newer_of_cache_and_bundle_wins() {
    let bundled = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    let older = bundled - Duration::from_secs(1);
    let newer = bundled + Duration::from_secs(1);

    assert!(prefer_bundled(None, Some(bundled)), "no cache");
    assert!(prefer_bundled(Some(older), Some(bundled)));
    assert!(!prefer_bundled(Some(newer), Some(bundled)));
    assert!(
        !prefer_bundled(Some(older), None),
        "an undated bundle never wins"
    );
    assert!(prefer_bundled(None, None));
}

#[test]
fn staleness_follows_the_ttl() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10 * CACHE_TTL.as_secs());
    assert!(is_stale_at(None, now), "nothing loaded");
    assert!(!is_stale_at(Some(now - Duration::from_secs(60)), now));
    assert!(is_stale_at(
        Some(now - CACHE_TTL - Duration::from_secs(1)),
        now
    ));
}

#[test]
fn downloads_stay_off_when_turned_off() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10 * CACHE_TTL.as_secs());
    assert!(!download_due(false, None, now));
    assert!(!download_due(false, Some(SystemTime::UNIX_EPOCH), now));
    assert!(download_due(true, None, now));
    assert!(!download_due(true, Some(now), now));
}

#[test]
fn blank_url_setting_falls_back_to_models_dev() {
    assert_eq!(catalog_url(""), DEFAULT_CATALOG_URL);
    assert_eq!(catalog_url("   "), DEFAULT_CATALOG_URL);
    assert_eq!(
        catalog_url(" https://mirror.example/api.json "),
        "https://mirror.example/api.json"
    );
}
