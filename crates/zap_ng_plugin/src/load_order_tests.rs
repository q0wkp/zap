// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;

use super::*;
use crate::manifest::Manifest;

fn plugin(id: &str, extra: &str, origin: Origin) -> Plugin {
    Plugin {
        manifest: Manifest::parse(&format!(
            "id = \"{id}\"\nname = \"{id}\"\nversion = \"1\"\napi = \"0.1\"\n{extra}"
        ))
        .unwrap(),
        dir: PathBuf::from(id),
        origin,
    }
}

fn ids(plugins: &[Plugin]) -> Vec<&str> {
    plugins
        .iter()
        .map(|plugin| plugin.manifest.id.as_str())
        .collect()
}

#[test]
fn a_plugin_comes_after_what_it_requires() {
    let (ordered, skipped) = load_order(
        vec![
            plugin("script", "requires = [\"runtime:python\"]\n", Origin::User),
            plugin(
                "python",
                "provides = [\"runtime:python\"]\nrequires = [\"base\"]\n",
                Origin::User,
            ),
            plugin("base", "provides = [\"base\"]\n", Origin::User),
        ],
        &HashMap::new(),
    );
    assert_eq!(ids(&ordered), ["base", "python", "script"]);
    assert_eq!(skipped, []);
}

#[test]
fn plugins_that_cannot_load_are_skipped_with_the_reason() {
    let enabled = HashMap::from([("off".to_owned(), false), ("opt-in".to_owned(), true)]);
    let (ordered, skipped) = load_order(
        vec![
            plugin(
                "importer",
                "[capabilities]\nsecrets = true\n",
                Origin::Bundled,
            ),
            plugin("off", "", Origin::User),
            plugin("opt-in", "enabled_by_default = false\n", Origin::User),
            plugin("opt-out", "enabled_by_default = false\n", Origin::User),
            plugin(
                "greedy",
                "[capabilities]\nread = [\".ssh\"]\n",
                Origin::User,
            ),
            plugin("script", "requires = [\"runtime:python\"]\n", Origin::User),
            plugin("importer", "", Origin::User),
            // What a disabled plugin provides is not there for others.
            plugin("needs-off", "requires = [\"x\"]\n", Origin::User),
        ],
        &enabled,
    );
    assert_eq!(ids(&ordered), ["importer", "opt-in"]);
    assert_eq!(ordered[0].origin, Origin::Bundled);
    let reasons: Vec<(&str, &str)> = skipped
        .iter()
        .map(|skipped| (skipped.id.as_str(), skipped.reason.as_str()))
        .collect();
    assert_eq!(reasons[0], ("off", "disabled"));
    assert_eq!(reasons[1], ("opt-out", "disabled"));
    assert!(reasons[2].0 == "greedy" && reasons[2].1.contains("only bundled"));
    assert_eq!(reasons[3], ("importer", "another plugin has the same id"));
    assert_eq!(
        reasons[4],
        ("script", "no enabled plugin provides runtime:python")
    );
    assert_eq!(reasons[5], ("needs-off", "no enabled plugin provides x"));
}
