use super::*;

const MINIMAL: &str = "id = \"demo\"\nname = \"Demo\"\nversion = \"1.0.0\"\napi = \"0.1\"\n";

#[test]
fn minimal_manifest_gets_the_defaults() {
    let manifest = Manifest::parse(MINIMAL).unwrap();
    assert_eq!(manifest.id, "demo");
    assert!(manifest.enabled_by_default);
    assert!(manifest.capabilities.is_empty());
    assert_eq!(manifest.wasm, None);
}

#[test]
fn manifest_with_everything() {
    let manifest = Manifest::parse(&format!(
        "{MINIMAL}wasm = \"plugin.wasm\"\nhooks = [\"first-run\"]\nprovides = [\"runtime:python\"]\n\
         requires = [\"runtime:wasi\"]\nenabled_by_default = false\n\
         [capabilities]\nread = [\".zap\"]\nwrite = [\".zap-ng\"]\nsecrets = true\n"
    ))
    .unwrap();
    assert_eq!(manifest.hooks, ["first-run"]);
    assert_eq!(manifest.capabilities.read, [".zap"]);
    assert!(manifest.capabilities.secrets && !manifest.enabled_by_default);
}

#[test]
fn bad_manifests_are_refused() {
    for (text, expected) in [
        (MINIMAL.replace("demo", "De mo"), "lowercase"),
        (MINIMAL.replace("0.1", "9.0"), "plugin interface"),
        (
            format!("{MINIMAL}wasm = \"../x.wasm\"\n"),
            "inside the plugin",
        ),
        (format!("{MINIMAL}hooks = [\"first-run\"]\n"), "need a wasm"),
        (
            format!("{MINIMAL}[capabilities]\nread = [\"/etc\"]\n"),
            "inside the home",
        ),
        (
            format!("{MINIMAL}[capabilities]\nwrite = [\"a/../../b\"]\n"),
            "inside the home",
        ),
        (format!("{MINIMAL}surprise = 1\n"), "unknown field"),
    ] {
        let error = format!("{:#}", Manifest::parse(&text).unwrap_err());
        assert!(error.contains(expected), "{text}: {error}");
    }
}

#[test]
fn discover_reads_the_plugins_of_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in [
        ("b", MINIMAL.to_owned()),
        ("a", "not toml at all [".to_owned()),
    ] {
        std::fs::create_dir(dir.path().join(name)).unwrap();
        std::fs::write(dir.path().join(name).join(MANIFEST_FILE_NAME), text).unwrap();
    }
    std::fs::create_dir(dir.path().join("no-manifest")).unwrap();

    let (plugins, rejected) = discover(dir.path(), Origin::User);
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].dir, dir.path().join("b"));
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].dir, dir.path().join("a"));

    assert_eq!(
        discover(&dir.path().join("missing"), Origin::User),
        (vec![], vec![])
    );
}
