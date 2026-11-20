//! Runs the test plugin in `tests/fixtures/probe` through the host. The plugin is built for
//! wasm32-wasip2 first; where that target isn't installed the tests say so and pass.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use zap_ng_plugin::{HostDirs, HostServices, Manifest, Origin, Plugin, PluginHost};

const TIMEOUT: Duration = Duration::from_secs(20);

/// Builds the test plugin once and returns its component, or `None` without the target.
#[allow(clippy::disallowed_methods, clippy::disallowed_types)]
fn probe_wasm() -> Option<&'static Path> {
    static WASM: OnceLock<Option<PathBuf>> = OnceLock::new();
    WASM.get_or_init(|| {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/probe");
        let target_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("probe");
        let output = std::process::Command::new(env!("CARGO"))
            .args([
                "build",
                "--release",
                "--target",
                "wasm32-wasip2",
                "--target-dir",
            ])
            .arg(&target_dir)
            .current_dir(&fixture)
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .output()
            .expect("cargo should run");
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("wasm32-wasip2") && stderr.contains("not installed") {
                eprintln!("skipped: the target wasm32-wasip2 is not installed");
                return None;
            }
            panic!("building the test plugin failed:\n{stderr}");
        }
        Some(target_dir.join("wasm32-wasip2/release/probe.wasm"))
    })
    .as_deref()
}

#[derive(Default)]
struct Secrets {
    written: Mutex<Vec<(String, String)>>,
}

impl HostServices for Secrets {
    fn read_secret(&self, service: &str, key: &str) -> Result<Option<String>, String> {
        Ok(Some(format!("{service}/{key}")))
    }

    fn write_secret(&self, key: &str, value: &str) -> Result<(), String> {
        self.written
            .lock()
            .unwrap()
            .push((key.to_owned(), value.to_owned()));
        Ok(())
    }
}

struct Fixture {
    home: tempfile::TempDir,
    host: PluginHost,
    secrets: Arc<Secrets>,
    plugin: Plugin,
}

fn fixture(capabilities: &str) -> Option<Fixture> {
    let wasm = probe_wasm()?;
    let home = tempfile::tempdir().unwrap();
    let plugin_dir = home.path().join("plugins/probe");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::copy(wasm, plugin_dir.join("probe.wasm")).unwrap();
    let manifest = Manifest::parse(&format!(
        "id = \"probe\"\nname = \"Probe\"\nversion = \"1\"\napi = \"0.1\"\nwasm = \"probe.wasm\"\n\
         hooks = [\"echo\"]\n{capabilities}"
    ))
    .unwrap();
    let secrets = Arc::new(Secrets::default());
    let host = PluginHost::new(
        HostDirs {
            home: home.path().to_owned(),
            work: home.path().join("work"),
        },
        secrets.clone(),
    )
    .unwrap();
    Some(Fixture {
        home,
        host,
        secrets,
        plugin: Plugin {
            manifest,
            dir: plugin_dir,
            origin: Origin::Bundled,
        },
    })
}

impl Fixture {
    fn call(&self, hook: &str, payload: &str) -> Result<String, String> {
        self.host
            .call(&self.plugin, hook, payload, "limit = 3\n", TIMEOUT)
            .map_err(|error| format!("{error:#}"))
    }
}

#[test]
fn a_call_gets_the_payload_and_the_plugins_config() {
    let Some(fixture) = fixture("") else { return };
    assert_eq!(fixture.call("echo", "hello").unwrap(), "hello|limit = 3\n");
    let error = fixture.call("nope", "").unwrap_err();
    assert!(
        error.contains("failed on nope: unknown hook nope"),
        "{error}"
    );
}

#[test]
fn a_plugin_reaches_its_working_directory_and_what_it_was_granted() {
    let Some(fixture) =
        fixture("[capabilities]\nread = [\".old\", \".absent\"]\nwrite = [\".new/data\"]\n")
    else {
        return;
    };
    let home = fixture.home.path();
    std::fs::create_dir(home.join(".old")).unwrap();
    std::fs::write(home.join(".old/settings.toml"), "old").unwrap();
    std::fs::write(home.join("private.txt"), "private").unwrap();

    // Its own working directory, kept between calls.
    fixture.call("write", "/work/state=kept").unwrap();
    assert_eq!(fixture.call("read", "/work/state").unwrap(), "kept");
    assert_eq!(
        std::fs::read_to_string(home.join("work/probe/state")).unwrap(),
        "kept"
    );

    // A directory granted for reading: read, but not written.
    assert_eq!(
        fixture.call("read", "/home/.old/settings.toml").unwrap(),
        "old"
    );
    fixture
        .call("write", "/home/.old/settings.toml=changed")
        .unwrap_err();
    assert_eq!(
        std::fs::read_to_string(home.join(".old/settings.toml")).unwrap(),
        "old"
    );

    // A directory granted for writing is created and written.
    fixture.call("write", "/home/.new/data/copied=new").unwrap();
    assert_eq!(
        std::fs::read_to_string(home.join(".new/data/copied")).unwrap(),
        "new"
    );

    // Nothing else of the home directory, nor of the machine, is there.
    fixture.call("read", "/home/private.txt").unwrap_err();
    fixture
        .call("read", "/home/.new/../private.txt")
        .unwrap_err();
    fixture.call("read", "/etc/hostname").unwrap_err();
    fixture.call("write", "/home/escaped=x").unwrap_err();
    assert!(!home.join("escaped").exists());
}

#[test]
fn the_keychain_needs_the_capability() {
    let Some(without) = fixture("") else { return };
    let error = without.call("secret", "value").unwrap_err();
    assert!(error.contains("secrets capability"), "{error}");
    assert!(without.secrets.written.lock().unwrap().is_empty());

    let with = fixture("[capabilities]\nsecrets = true\n").unwrap();
    assert_eq!(with.call("secret", "value").unwrap(), "old-service/key");
    assert_eq!(
        *with.secrets.written.lock().unwrap(),
        [("key".to_owned(), "value".to_owned())]
    );
}

#[test]
#[allow(clippy::disallowed_types)]
fn a_call_that_does_not_end_is_stopped() {
    let Some(fixture) = fixture("") else { return };
    let started = std::time::Instant::now();
    let error = fixture
        .host
        .call(&fixture.plugin, "spin", "", "", Duration::from_millis(300))
        .map_err(|error| format!("{error:#}"))
        .unwrap_err();
    assert!(error.contains("stopped on spin"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(10));
    // The host still works afterwards.
    assert_eq!(fixture.call("echo", "again").unwrap(), "again|limit = 3\n");
}
