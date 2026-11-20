//! The application's side of plugins (`crates/zap_ng_plugin`): where they are found, what
//! `zap-ng.toml` says about them, and the services the host can't provide by itself.
//!
//! Plugins shipped with ZapNG are in the bundled resources, the user's own in the `plugins`
//! directory of the configuration directory in the home directory. `zap-ng.toml` can turn one on
//! or off and configure it:
//!
//! ```toml
//! [plugins.import-zap]
//! enabled = false
//!
//! [plugins.import-zap.config]
//! key = "value"
//! ```

mod zap_import;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context as _, Result};
use serde::{Deserialize, Serialize};
use zap_ng_core::channel::ChannelState;
use zap_ng_plugin::{discover, load_order, HostDirs, HostServices, Origin, Plugin, PluginHost};
use zap_ng_ui_extras::secure_storage;

pub use zap_import::offer_zap_import;

/// The file, in the plugins' working directory, of the calls to make on the next start.
const DEFERRED_CALLS_FILE_NAME: &str = "deferred-calls.json";

/// How long a call made at startup may take. Importing copies whole directories.
const DEFERRED_CALL_TIMEOUT: Duration = Duration::from_secs(600);

/// The directory holding one working directory per plugin.
fn work_dir() -> PathBuf {
    zap_ng_core::paths::state_dir().join("plugins")
}

/// What `zap-ng.toml` says about plugins.
#[derive(Default)]
struct PluginSettings {
    enabled: HashMap<String, bool>,
    /// Each plugin's `config` table, as TOML text.
    config: HashMap<String, String>,
}

impl PluginSettings {
    fn read() -> Self {
        let mut settings = Self::default();
        let path = crate::settings::zap_ng_file::zap_ng_settings_file_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return settings;
        };
        let table: toml::Table = match text.parse() {
            Ok(table) => table,
            Err(error) => {
                log::warn!(
                    "Can't read the plugin settings of {}: {error}",
                    path.display()
                );
                return settings;
            }
        };
        let Some(plugins) = table.get("plugins").and_then(toml::Value::as_table) else {
            return settings;
        };
        for (id, plugin) in plugins {
            if let Some(enabled) = plugin.get("enabled").and_then(toml::Value::as_bool) {
                settings.enabled.insert(id.clone(), enabled);
            }
            if let Some(config) = plugin.get("config").and_then(toml::Value::as_table) {
                settings.config.insert(id.clone(), config.to_string());
            }
        }
        settings
    }
}

/// The keychain, for plugins that were given it.
struct AppHostServices;

impl AppHostServices {
    /// Where the application named by a keychain service keeps secrets on Linux when there is
    /// no secret service. Zap's is known; no other application's is.
    fn fallback_dir(service: &str) -> Option<PathBuf> {
        if !cfg!(any(target_os = "linux", target_os = "freebsd")) {
            return None;
        }
        if service == ChannelState::data_domain() {
            Some(zap_ng_core::paths::state_dir())
        } else if service == zap_import::ZAP_KEYCHAIN_SERVICE {
            dirs::home_dir().map(|home| home.join(".local/state/zap"))
        } else {
            None
        }
    }
}

impl HostServices for AppHostServices {
    fn read_secret(&self, service: &str, key: &str) -> Result<Option<String>, String> {
        match secure_storage::open(service, Self::fallback_dir(service)).read_value(key) {
            Ok(value) => Ok(Some(value)),
            Err(secure_storage::Error::NotFound) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn write_secret(&self, key: &str, value: &str) -> Result<(), String> {
        let service = ChannelState::data_domain();
        secure_storage::open(&service, Self::fallback_dir(&service))
            .write_value(key, value)
            .map_err(|error| error.to_string())
    }
}

/// The plugins that are loaded, and the host that runs them.
pub struct Plugins {
    host: PluginHost,
    plugins: Vec<Plugin>,
    settings: PluginSettings,
}

impl Plugins {
    /// Finds the plugins and decides which are loaded. Nothing of a plugin runs yet.
    pub fn load() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("no home directory"))?;
        let settings = PluginSettings::read();

        // Bundled plugins first: of two with the same id, the first one found is kept.
        let mut found = Vec::new();
        let sources = [
            (
                zap_ng_core::paths::bundled_resources_dir()
                    .map(|dir| dir.join("bundled").join("plugins")),
                Origin::Bundled,
            ),
            (
                zap_ng_core::paths::zap_ng_home_config_dir().map(|dir| dir.join("plugins")),
                Origin::User,
            ),
        ];
        for (dir, origin) in sources {
            let Some(dir) = dir else { continue };
            let (plugins, rejected) = discover(&dir, origin);
            for rejected in rejected {
                log::warn!(
                    "Not a usable plugin: {}: {}",
                    rejected.dir.display(),
                    rejected.reason
                );
            }
            found.extend(plugins);
        }
        let (plugins, skipped) = load_order(found, &settings.enabled);
        for skipped in skipped {
            log::info!("Plugin {} not loaded: {}", skipped.id, skipped.reason);
        }

        let host = PluginHost::new(
            HostDirs {
                home,
                work: work_dir(),
            },
            Arc::new(AppHostServices),
        )?;
        Ok(Self {
            host,
            plugins,
            settings,
        })
    }

    /// Calls a loaded plugin for a hook its manifest subscribes to.
    pub fn call(&self, id: &str, hook: &str, payload: &str, timeout: Duration) -> Result<String> {
        let plugin = self
            .plugins
            .iter()
            .find(|plugin| plugin.manifest.id == id)
            .ok_or_else(|| anyhow!("the plugin {id} is not loaded"))?;
        if !plugin
            .manifest
            .hooks
            .iter()
            .any(|subscribed| subscribed == hook)
        {
            return Err(anyhow!("the plugin {id} does not take the hook {hook}"));
        }
        let config = self.settings.config.get(id).map_or("", String::as_str);
        self.host.call(plugin, hook, payload, config, timeout)
    }
}

/// A call to make on the next start, before the application reads its settings and data.
#[derive(Debug, Deserialize, Serialize)]
struct DeferredCall {
    plugin: String,
    hook: String,
    payload: String,
}

/// Leaves a call for the next start. A plugin that replaces the application's files, as an
/// import does, can't run while the application has them open.
fn defer_call(call: DeferredCall) -> Result<()> {
    let dir = work_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("can't create {}", dir.display()))?;
    let path = dir.join(DEFERRED_CALLS_FILE_NAME);
    let mut calls: Vec<DeferredCall> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    calls.push(call);
    std::fs::write(&path, serde_json::to_string(&calls)?)
        .with_context(|| format!("can't write {}", path.display()))
}

/// Makes the calls left for this start. To be called before the settings and the database are
/// opened. A failed call is logged and not tried again.
pub fn run_deferred_calls() {
    let path = work_dir().join(DEFERRED_CALLS_FILE_NAME);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    // Removed first, so that a call that fails, or never returns, is not made on every start.
    if let Err(error) = std::fs::remove_file(&path) {
        log::warn!("Can't remove {}: {error}", path.display());
        return;
    }
    let calls: Vec<DeferredCall> = match serde_json::from_str(&text) {
        Ok(calls) => calls,
        Err(error) => {
            log::warn!("Can't read the plugin calls left for this start: {error}");
            return;
        }
    };
    let plugins = match Plugins::load() {
        Ok(plugins) => plugins,
        Err(error) => {
            log::warn!("Can't load the plugins: {error:#}");
            return;
        }
    };
    for call in calls {
        match plugins.call(
            &call.plugin,
            &call.hook,
            &call.payload,
            DEFERRED_CALL_TIMEOUT,
        ) {
            Ok(answer) => log::info!("Plugin {} finished {}: {answer}", call.plugin, call.hook),
            Err(error) => log::warn!("{error:#}"),
        }
    }
}
