// We can use `std::process:Command` here because this is invoked within a build script,
// _not_ within the Zap binary.
#![allow(clippy::disallowed_types)]

use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        macos: { target_os = "macos" },
        native: { not(target_family = "wasm") },
    }
}
