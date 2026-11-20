//! A plugin's manifest, `plugin.toml`, and finding the plugins in a directory.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use serde::Deserialize;

/// The name of the manifest file in a plugin's directory.
pub const MANIFEST_FILE_NAME: &str = "plugin.toml";

/// The version of the plugin interface this build offers (`wit/plugin.wit`).
pub const API_VERSION: &str = "0.1";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Unique among plugins; also the name of the plugin's table in `zap-ng.toml`.
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    /// The version of the plugin interface the plugin was built for.
    pub api: String,
    /// The component, relative to the plugin's directory. A plugin that only provides
    /// something to other plugins may have none.
    #[serde(default)]
    pub wasm: Option<PathBuf>,
    /// The hooks the plugin is called for.
    #[serde(default)]
    pub hooks: Vec<String>,
    /// Names other plugins can require, such as `runtime:python`.
    #[serde(default)]
    pub provides: Vec<String>,
    /// Names that some enabled plugin must provide for this one to load.
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub capabilities: Capabilities,
    /// Whether the plugin runs when `zap-ng.toml` doesn't say.
    #[serde(default = "enabled_by_default")]
    pub enabled_by_default: bool,
}

fn enabled_by_default() -> bool {
    true
}

/// What a plugin asks to reach beyond its own working directory.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    /// Directories to read, relative to the user's home directory.
    #[serde(default)]
    pub read: Vec<String>,
    /// Directories to read and write, relative to the user's home directory.
    #[serde(default)]
    pub write: Vec<String>,
    /// The system keychain, through the host's secret functions.
    #[serde(default)]
    pub secrets: bool,
}

impl Capabilities {
    pub fn is_empty(&self) -> bool {
        self.read.is_empty() && self.write.is_empty() && !self.secrets
    }
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self> {
        let manifest: Manifest = toml_edit::de::from_str(text)?;
        if manifest.id.is_empty()
            || !manifest
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            bail!(
                "the id {:?} must be lowercase letters, digits and hyphens",
                manifest.id
            );
        }
        if manifest.api != API_VERSION {
            bail!(
                "built for plugin interface {}, this ZapNG offers {API_VERSION}",
                manifest.api
            );
        }
        if let Some(wasm) = &manifest.wasm {
            if !is_inside(wasm) {
                bail!("wasm {wasm:?} must be a path inside the plugin's directory");
            }
        } else if !manifest.hooks.is_empty() {
            bail!("hooks need a wasm component to call");
        }
        for grant in manifest
            .capabilities
            .read
            .iter()
            .chain(&manifest.capabilities.write)
        {
            if grant.is_empty() || !is_inside(Path::new(grant)) {
                bail!("the directory {grant:?} must be a path inside the home directory");
            }
        }
        Ok(manifest)
    }
}

/// Whether a relative path stays inside the directory it is joined to.
fn is_inside(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, std::path::Component::Normal(_)))
        && path.components().next().is_some()
}

/// Where a plugin was found, which decides how far it is trusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Shipped with ZapNG.
    Bundled,
    /// Put into the plugin directory by the user.
    User,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plugin {
    pub manifest: Manifest,
    pub dir: PathBuf,
    pub origin: Origin,
}

impl Plugin {
    pub fn wasm_path(&self) -> Option<PathBuf> {
        self.manifest.wasm.as_ref().map(|wasm| self.dir.join(wasm))
    }
}

/// A directory that looked like a plugin but could not be used, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejected {
    pub dir: PathBuf,
    pub reason: String,
}

/// Reads every plugin directly under `dir`: each subdirectory holding a manifest. A missing
/// `dir` holds no plugins.
pub fn discover(dir: &Path, origin: Origin) -> (Vec<Plugin>, Vec<Rejected>) {
    let mut plugins = Vec::new();
    let mut rejected = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (plugins, rejected);
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join(MANIFEST_FILE_NAME).is_file())
        .collect();
    dirs.sort();
    for plugin_dir in dirs {
        let manifest = std::fs::read_to_string(plugin_dir.join(MANIFEST_FILE_NAME))
            .context("can't read the manifest")
            .and_then(|text| Manifest::parse(&text));
        match manifest {
            Ok(manifest) => plugins.push(Plugin {
                manifest,
                dir: plugin_dir,
                origin,
            }),
            Err(error) => rejected.push(Rejected {
                dir: plugin_dir,
                reason: format!("{error:#}"),
            }),
        }
    }
    (plugins, rejected)
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
