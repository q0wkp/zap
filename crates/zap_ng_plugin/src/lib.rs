//! Plugins: WebAssembly components that ZapNG loads at run time and calls at fixed points
//! (hooks).
//!
//! A plugin is a directory holding a manifest, `plugin.toml`, and usually a component built
//! against `wit/plugin.wit`. It runs in a sandbox: it sees its own working directory, the
//! directories its manifest asks for, and the functions of the `host` interface, nothing else.
//! A plugin can also provide a name that other plugins require, such as a language runtime, and
//! is then loaded before them.

mod host;
mod load_order;
mod manifest;

pub use host::{HostDirs, HostServices, PluginHost, GUEST_HOME_DIR, GUEST_WORK_DIR};
pub use load_order::{load_order, Skipped};
pub use manifest::{
    discover, Capabilities, Manifest, Origin, Plugin, Rejected, API_VERSION, MANIFEST_FILE_NAME,
};
