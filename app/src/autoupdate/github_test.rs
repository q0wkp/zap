use super::*;

#[test]
fn test_dmg_name() {
    assert_eq!(
        dmg_name("2026.07.09.2-mod.1.1", true),
        "zap-2026.07.09.2-mod.1.1-macos-aarch64.dmg"
    );
    assert_eq!(
        dmg_name("2026.07.09.2-mod.1.1", false),
        "zap-2026.07.09.2-mod.1.1-macos-x86_64.dmg"
    );
}

#[test]
fn test_appimage_name() {
    assert_eq!(
        appimage_name("2026.07.09.2-mod.1.1"),
        "zap-2026.07.09.2-mod.1.1-linux-x86_64.AppImage"
    );
}

/// The updater looks a release's file up by the name it builds from the release's version.
#[test]
fn test_release_file_found_by_its_version() {
    let release: GithubRelease = serde_json::from_value(serde_json::json!({
        "tag_name": "v2026.07.09.2-mod.1.1",
        "html_url": "https://github.com/q0wkp/zap/releases/tag/v2026.07.09.2-mod.1.1",
        "assets": [
            {
                "name": "zap-2026.07.09.2-mod.1.1-linux-x86_64.AppImage",
                "browser_download_url": "https://github.com/q0wkp/zap/releases/download/v2026.07.09.2-mod.1.1/zap-2026.07.09.2-mod.1.1-linux-x86_64.AppImage",
                "size": 1,
            },
            {
                "name": "zap-2026.07.09.2-mod.1.1-macos-aarch64.dmg",
                "browser_download_url": "https://github.com/q0wkp/zap/releases/download/v2026.07.09.2-mod.1.1/zap-2026.07.09.2-mod.1.1-macos-aarch64.dmg",
                "size": 1,
            },
        ],
    }))
    .unwrap();

    assert!(release
        .find_asset(&appimage_name(release.version()))
        .is_some());
    assert!(release
        .find_asset(&dmg_name(release.version(), true))
        .is_some());
    assert!(release.find_asset("Zap-x86_64.AppImage").is_none());
}
