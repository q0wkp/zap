//! Runs the plugin in `plugins/import-zap`, as built for wasm32-wasip2, through the host. Where
//! that target isn't installed the test says so and passes.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use zap_ng_plugin::{discover, HostDirs, HostServices, Origin, PluginHost};

#[derive(Default)]
struct Keychain {
    written: Mutex<Vec<(String, String)>>,
}

impl HostServices for Keychain {
    fn read_secret(&self, service: &str, key: &str) -> Result<Option<String>, String> {
        Ok((service == "dev.zap.Zap" && key == "AiApiKeys").then(|| "secret".to_owned()))
    }

    fn write_secret(&self, key: &str, value: &str) -> Result<(), String> {
        self.written
            .lock()
            .unwrap()
            .push((key.to_owned(), value.to_owned()));
        Ok(())
    }
}

#[test]
#[allow(clippy::disallowed_methods, clippy::disallowed_types)]
fn a_zap_install_is_imported() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/import-zap");
    let target_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("import-zap");
    let output = std::process::Command::new(env!("CARGO"))
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-wasip2",
            "--target-dir",
        ])
        .arg(&target_dir)
        .current_dir(&source)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .expect("cargo should run");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("wasm32-wasip2") && stderr.contains("not installed") {
            eprintln!("skipped: the target wasm32-wasip2 is not installed");
            return;
        }
        panic!("building the plugin failed:\n{stderr}");
    }

    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let plugin_dir = home.join("bundled/import-zap");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::copy(source.join("plugin.toml"), plugin_dir.join("plugin.toml")).unwrap();
    std::fs::copy(
        target_dir.join("wasm32-wasip2/release/import_zap.wasm"),
        plugin_dir.join("import_zap.wasm"),
    )
    .unwrap();
    let (plugins, rejected) = discover(&home.join("bundled"), Origin::Bundled);
    assert_eq!(rejected, []);
    let plugin = &plugins[0];

    let keychain = Arc::new(Keychain::default());
    let host = PluginHost::new(
        HostDirs {
            home: home.to_owned(),
            work: home.join("work"),
        },
        keychain.clone(),
    )
    .unwrap();
    let call = |hook: &str, payload: &str| {
        host.call(plugin, hook, payload, "", Duration::from_secs(60))
            .map_err(|error| format!("{error:#}"))
    };

    assert_eq!(call("detect", "").unwrap(), "[]");

    std::fs::create_dir_all(home.join(".config/zap")).unwrap();
    std::fs::write(
        home.join(".config/zap/settings.toml"),
        "[network]\nproxy_url = \"http://proxy\"\n\n[appearance.themes]\ntheme = \"Dark\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(home.join(".local/state/zap")).unwrap();
    std::fs::write(home.join(".local/state/zap/warp.sqlite"), "db").unwrap();

    assert_eq!(
        call("detect", "").unwrap(),
        r#"[".config/zap",".local/state/zap"]"#
    );
    let report = call(
        "import",
        r#"{"settings_routes":[["network.proxy_url","network.proxy.url"]]}"#,
    )
    .unwrap();
    assert_eq!(
        report,
        r#"{"files":2,"settings_moved":1,"secrets":1,"errors":[]}"#
    );

    let read = |path: &str| std::fs::read_to_string(home.join(path)).unwrap();
    assert_eq!(read(".local/state/zap-ng/warp.sqlite"), "db");
    assert_eq!(
        read(".config/zap-ng/settings.toml"),
        "[appearance.themes]\ntheme = \"Dark\"\n"
    );
    assert_eq!(
        read(".config/zap-ng/zap-ng.toml"),
        "[network.proxy]\nurl = \"http://proxy\"\n"
    );
    assert_eq!(
        *keychain.written.lock().unwrap(),
        [("AiApiKeys".to_owned(), "secret".to_owned())]
    );
    // Zap's own files are as they were.
    assert!(read(".config/zap/settings.toml").contains("proxy_url"));
}
