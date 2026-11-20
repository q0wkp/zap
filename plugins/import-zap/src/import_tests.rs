use super::*;

fn routes() -> Vec<(String, String)> {
    [
        ("agents.warp_agent.providers", "providers"),
        ("network.proxy_url", "network.proxy.url"),
        ("network.proxy_mode", "network.proxy.mode"),
        ("appearance.language", "appearance.language"),
        (
            "updates.automatic_updates_enabled",
            "updates.automatic_updates_enabled",
        ),
    ]
    .map(|(from, to)| (from.to_owned(), to.to_owned()))
    .to_vec()
}

const ZAP_SETTINGS: &str = r#"# my settings
[appearance]
language = "zh-CN"

[appearance.themes]
theme = "Dracula" # keep

[network]
proxy_url = "http://proxy:8080"
proxy_mode = "manual"

[agents.warp_agent]
is_any_ai_enabled = true

[[agents.warp_agent.providers]]
id = "p1"
name = "Local"

[[agents.warp_agent.providers]]
id = "p2"
name = "Other"
"#;

#[test]
fn routed_settings_move_and_the_rest_stays() {
    let (settings, zap_ng, moved) = split_settings(ZAP_SETTINGS, "", &routes()).unwrap();
    assert_eq!(moved, 4);

    let settings: DocumentMut = settings.parse().unwrap();
    assert_eq!(
        settings["appearance"]["themes"]["theme"].as_str(),
        Some("Dracula")
    );
    assert_eq!(
        settings["agents"]["warp_agent"]["is_any_ai_enabled"].as_bool(),
        Some(true)
    );
    assert!(settings["appearance"].get("language").is_none());
    assert!(settings["agents"]["warp_agent"].get("providers").is_none());
    // The table the move emptied is gone, comments on what stayed are kept.
    assert!(settings.get("network").is_none());
    assert!(settings.to_string().contains("# my settings"));
    assert!(settings.to_string().contains("# keep"));

    let zap_ng: DocumentMut = zap_ng.parse().unwrap();
    assert_eq!(zap_ng["appearance"]["language"].as_str(), Some("zh-CN"));
    assert_eq!(
        zap_ng["network"]["proxy"]["url"].as_str(),
        Some("http://proxy:8080")
    );
    assert_eq!(zap_ng["network"]["proxy"]["mode"].as_str(), Some("manual"));
    let providers = zap_ng["providers"].as_array_of_tables().unwrap();
    assert_eq!(providers.len(), 2);
    assert_eq!(providers.get(1).unwrap()["name"].as_str(), Some("Other"));
}

#[test]
fn what_zap_ng_toml_already_holds_is_kept() {
    let existing =
        "[plugins.import-zap]\nenabled = true\n\n[network.proxy]\nno_proxy = \"localhost\"\n";
    let (_, zap_ng, moved) = split_settings(ZAP_SETTINGS, existing, &routes()).unwrap();
    assert_eq!(moved, 4);
    let zap_ng: DocumentMut = zap_ng.parse().unwrap();
    assert_eq!(
        zap_ng["plugins"]["import-zap"]["enabled"].as_bool(),
        Some(true)
    );
    assert_eq!(
        zap_ng["network"]["proxy"]["no_proxy"].as_str(),
        Some("localhost")
    );
    assert_eq!(
        zap_ng["network"]["proxy"]["url"].as_str(),
        Some("http://proxy:8080")
    );
}

#[test]
fn settings_without_routed_ones_are_left_alone() {
    let text = "[appearance.themes]\ntheme = \"Dark\"\n";
    assert_eq!(
        split_settings(text, "", &routes()).unwrap(),
        (text.to_owned(), String::new(), 0)
    );
    assert!(split_settings("not [toml", "", &routes()).is_err());
}

#[test]
fn the_directories_of_a_zap_install_are_copied() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(detect(home), [""; 0]);

    let write = |path: &str, text: &str| {
        let path = home.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(".zap/prompts/system.j2", "prompt");
    write(".zap/remote-server/zap-oss", "binary");
    write(".config/zap/settings.toml", ZAP_SETTINGS);
    write(".config/zap/user_preferences.json", "{}");
    write(".local/state/zap/warp.sqlite", "db");
    write(".local/state/zap/warp.sqlite-wal", "wal");
    write(".local/state/zap/zap.log", "log");
    write(".local/state/zap/logs/old.log", "log");
    // What ZapNG created on its first start is replaced.
    write(".local/state/zap-ng/warp.sqlite", "fresh");
    write(
        ".config/zap-ng/zap-ng.toml",
        "[plugins.import-zap]\nenabled = true\n",
    );

    assert_eq!(detect(home), [".zap", ".config/zap", ".local/state/zap"]);
    let report = import_files(home, &routes());
    assert_eq!(report.errors, [""; 0]);
    assert_eq!((report.files, report.settings_moved), (5, 4));

    let read = |path: &str| std::fs::read_to_string(home.join(path)).unwrap();
    assert_eq!(read(".zap-ng/prompts/system.j2"), "prompt");
    assert_eq!(read(".local/state/zap-ng/warp.sqlite"), "db");
    assert_eq!(read(".local/state/zap-ng/warp.sqlite-wal"), "wal");
    assert_eq!(read(".config/zap-ng/user_preferences.json"), "{}");
    assert!(!home.join(".zap-ng/remote-server").exists());
    assert!(!home.join(".local/state/zap-ng/zap.log").exists());
    assert!(!home.join(".local/state/zap-ng/logs").exists());

    assert!(!read(".config/zap-ng/settings.toml").contains("proxy_url"));
    let zap_ng = read(".config/zap-ng/zap-ng.toml");
    assert!(
        zap_ng.contains("[plugins.import-zap]") && zap_ng.contains("http://proxy:8080"),
        "{zap_ng}"
    );
    // Zap's own files are as they were.
    assert_eq!(read(".config/zap/settings.toml"), ZAP_SETTINGS);
}
