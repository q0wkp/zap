//! Interface for working with processes that ensures that commands are spawned with the correct set
//! of arguments.
//!
//! [`blocking::Command`] can be used as a drop-in replacement for [`std::process::Command`],
//! and [`r#async::Command`] can be used as a drop-in replacement for [`async_process::Command`].
#[cfg(not(target_family = "wasm"))]
pub mod r#async;
pub mod blocking;
#[cfg(unix)]
pub mod unix;

pub use std::process::{ExitStatus, Output, Stdio};
