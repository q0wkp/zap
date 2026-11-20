use super::*;

const ROUTES: &[Route] = &[
    ("agents.warp_agent.providers", "providers"),
    ("network.proxy_url", "network.proxy.url"),
];

fn open(dir: &std::path::Path) -> SplitTomlUserPreferences {
    let (preferences, error) =
        SplitTomlUserPreferences::new(dir.join("settings.toml"), dir.join("zap-ng.toml"), ROUTES);
    assert!(error.is_none());
    preferences
}

#[test]
fn routed_settings_are_kept_in_the_second_file_under_their_own_paths() {
    let dir = tempfile::tempdir().unwrap();
    let preferences = open(dir.path());

    preferences
        .write_value_with_hierarchy(
            "proxy_url",
            "\"http://proxy\"".to_owned(),
            Some("network"),
            None,
        )
        .unwrap();
    preferences
        .write_value_with_hierarchy(
            "providers",
            "\"p\"".to_owned(),
            Some("agents.warp_agent"),
            None,
        )
        .unwrap();
    preferences
        .write_value_with_hierarchy("font_size", "13".to_owned(), Some("appearance.text"), None)
        .unwrap();

    let main = std::fs::read_to_string(dir.path().join("settings.toml")).unwrap();
    let routed = std::fs::read_to_string(dir.path().join("zap-ng.toml")).unwrap();
    assert!(
        main.contains("font_size") && !main.contains("proxy") && !main.contains("providers"),
        "{main}"
    );
    assert!(
        routed.contains("[network.proxy]") && routed.contains("url"),
        "{routed}"
    );
    assert!(
        routed.contains("providers") && !routed.contains("warp_agent"),
        "{routed}"
    );
    assert!(!routed.contains("font_size"), "{routed}");

    // Read back through the paths the settings are defined with, also from a fresh store.
    for preferences in [preferences, open(dir.path())] {
        assert_eq!(
            preferences
                .read_value_with_hierarchy("proxy_url", Some("network"))
                .unwrap()
                .as_deref(),
            Some("\"http://proxy\"")
        );
        assert_eq!(
            preferences
                .read_value_with_hierarchy("font_size", Some("appearance.text"))
                .unwrap()
                .as_deref(),
            Some("13")
        );
    }
}

#[test]
fn a_routed_setting_in_the_main_file_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.toml"),
        "[network]\nproxy_url = \"http://old\"\n",
    )
    .unwrap();
    let preferences = open(dir.path());
    assert_eq!(
        preferences
            .read_value_with_hierarchy("proxy_url", Some("network"))
            .unwrap(),
        None
    );
}

#[test]
fn removing_a_routed_setting_leaves_the_main_file_alone() {
    let dir = tempfile::tempdir().unwrap();
    let preferences = open(dir.path());
    preferences
        .write_value_with_hierarchy(
            "proxy_url",
            "\"http://proxy\"".to_owned(),
            Some("network"),
            None,
        )
        .unwrap();
    preferences
        .remove_value_with_hierarchy("proxy_url", Some("network"))
        .unwrap();
    assert_eq!(
        preferences
            .read_value_with_hierarchy("proxy_url", Some("network"))
            .unwrap(),
        None
    );
    assert!(!dir.path().join("settings.toml").exists());
}
