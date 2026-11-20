// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

//! Running a plugin's component.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context as _, Result};
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::manifest::Plugin;

mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "plugin",
    });
}

use bindings::zap_ng::plugin::host::{Host, LogLevel};

/// The directory a plugin's own working directory appears at, inside the plugin.
pub const GUEST_WORK_DIR: &str = "/work";
/// The directory the user's home directory appears at, inside the plugin. Only the directories
/// the plugin was granted exist under it.
pub const GUEST_HOME_DIR: &str = "/home";

/// What the application does for plugins that this crate can't do itself.
pub trait HostServices: Send + Sync {
    /// Reads a secret another application stored in the system keychain.
    fn read_secret(&self, service: &str, key: &str) -> Result<Option<String>, String>;
    /// Stores a secret in the application's own keychain entry.
    fn write_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

/// Where a plugin's directories are on this machine.
#[derive(Clone, Debug)]
pub struct HostDirs {
    /// The user's home directory, which a manifest's directory grants are relative to.
    pub home: PathBuf,
    /// The directory holding one working directory per plugin.
    pub work: PathBuf,
}

pub struct PluginHost {
    engine: Engine,
    linker: Linker<State>,
    dirs: HostDirs,
    services: Arc<dyn HostServices>,
}

struct State {
    wasi: WasiCtx,
    table: ResourceTable,
    plugin_id: String,
    config: String,
    may_use_secrets: bool,
    services: Arc<dyn HostServices>,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl Host for State {
    fn log(&mut self, level: LogLevel, message: String) {
        let level = match level {
            LogLevel::Debug => log::Level::Debug,
            LogLevel::Info => log::Level::Info,
            LogLevel::Warn => log::Level::Warn,
            LogLevel::Error => log::Level::Error,
        };
        log::log!(level, "[plugin {}] {message}", self.plugin_id);
    }

    fn plugin_config(&mut self) -> String {
        self.config.clone()
    }

    fn read_secret(&mut self, service: String, key: String) -> Result<Option<String>, String> {
        self.check_secrets()?;
        self.services.read_secret(&service, &key)
    }

    fn write_secret(&mut self, key: String, value: String) -> Result<(), String> {
        self.check_secrets()?;
        self.services.write_secret(&key, &value)
    }
}

impl State {
    fn check_secrets(&self) -> Result<(), String> {
        if self.may_use_secrets {
            Ok(())
        } else {
            Err("the plugin's manifest doesn't ask for the secrets capability".to_owned())
        }
    }
}

impl PluginHost {
    pub fn new(dirs: HostDirs, services: Arc<dyn HostServices>) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        // Lets a call be cut short once its time is up.
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(runtime_error)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(runtime_error)?;
        bindings::Plugin::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)
            .map_err(runtime_error)?;
        Ok(Self {
            engine,
            linker,
            dirs,
            services,
        })
    }

    /// Calls the plugin for one hook and returns what it answered. Each call runs a fresh
    /// instance, so a plugin keeps nothing in memory between calls; what it wants to keep goes
    /// into its working directory.
    ///
    /// `config` is the plugin's own table of `zap-ng.toml` as TOML text. A call still running
    /// after `timeout` is stopped and fails.
    pub fn call(
        &self,
        plugin: &Plugin,
        hook: &str,
        payload: &str,
        config: &str,
        timeout: Duration,
    ) -> Result<String> {
        let id = &plugin.manifest.id;
        let wasm_path = plugin
            .wasm_path()
            .ok_or_else(|| anyhow!("the plugin {id} has no component to call"))?;
        let component = Component::from_file(&self.engine, &wasm_path)
            .map_err(runtime_error)
            .with_context(|| format!("can't load {}", wasm_path.display()))?;

        let state = State {
            wasi: self.wasi_ctx(plugin)?,
            table: ResourceTable::new(),
            plugin_id: id.clone(),
            config: config.to_owned(),
            may_use_secrets: plugin.manifest.capabilities.secrets,
            services: self.services.clone(),
        };
        let mut store = Store::new(&self.engine, state);
        store.set_epoch_deadline(1);
        store.epoch_deadline_trap();

        // The call ends, at the latest, when the epoch moves on. Should the call end first, the
        // thread still moves the epoch once it wakes, which stops nothing: every call sets its
        // deadline one epoch ahead of the engine's current one.
        let (done_sender, done_receiver) = std::sync::mpsc::channel::<()>();
        let engine = self.engine.clone();
        std::thread::spawn(move || {
            if done_receiver.recv_timeout(timeout).is_err() {
                engine.increment_epoch();
            }
        });

        let result = bindings::Plugin::instantiate(&mut store, &component, &self.linker)
            .and_then(|instance| instance.call_handle(&mut store, hook, payload));
        drop(done_sender);
        match result {
            Ok(Ok(answer)) => Ok(answer),
            Ok(Err(message)) => Err(anyhow!("the plugin {id} failed on {hook}: {message}")),
            Err(error) => {
                Err(runtime_error(error).context(format!("the plugin {id} stopped on {hook}")))
            }
        }
    }

    fn wasi_ctx(&self, plugin: &Plugin) -> Result<WasiCtx> {
        let work_dir = self.dirs.work.join(&plugin.manifest.id);
        std::fs::create_dir_all(&work_dir)
            .with_context(|| format!("can't create {}", work_dir.display()))?;

        let mut builder = WasiCtxBuilder::new();
        builder.env("HOME", GUEST_HOME_DIR);
        builder
            .preopened_dir(&work_dir, GUEST_WORK_DIR, DirPerms::all(), FilePerms::all())
            .map_err(runtime_error)?;
        let capabilities = &plugin.manifest.capabilities;
        // A directory granted for writing is created; one granted for reading is offered only
        // if it exists.
        for grant in &capabilities.write {
            let host_dir = self.dirs.home.join(grant);
            std::fs::create_dir_all(&host_dir)
                .with_context(|| format!("can't create {}", host_dir.display()))?;
            builder
                .preopened_dir(
                    &host_dir,
                    guest_path(grant),
                    DirPerms::all(),
                    FilePerms::all(),
                )
                .map_err(runtime_error)?;
        }
        for grant in &capabilities.read {
            let host_dir = self.dirs.home.join(grant);
            if host_dir.is_dir() && !capabilities.write.contains(grant) {
                builder
                    .preopened_dir(
                        &host_dir,
                        guest_path(grant),
                        DirPerms::READ,
                        FilePerms::READ,
                    )
                    .map_err(runtime_error)?;
            }
        }
        Ok(builder.build())
    }
}

/// Where a directory granted relative to the home directory appears inside the plugin.
fn guest_path(grant: &str) -> String {
    Path::new(GUEST_HOME_DIR)
        .join(grant)
        .to_string_lossy()
        .into_owned()
}

/// The runtime has an error type of its own; the rest of the application reads this one.
fn runtime_error(error: wasmtime::Error) -> anyhow::Error {
    anyhow!("{error:#}")
}
