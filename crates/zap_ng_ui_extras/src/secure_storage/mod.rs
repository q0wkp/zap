// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: added opening a service's secure storage on its own, with an optional Linux fallback directory, and removed the Windows-only code; renamed the Warp and Zap names to ZapNG.

//! Secure storage for passwords and other application secrets.
//!
//! This defines an API for interacting with an underlying secure storage
//! system, implementations of the API for various platforms, testing
//! utilities, and extension traits to improve ergonomics of using the APIs.

#[cfg(not(target_family = "wasm"))]
#[cfg_attr(target_os = "macos", path = "mac.rs")]
#[cfg_attr(any(target_os = "linux", target_os = "freebsd"), path = "linux.rs")]
mod imp;
mod noop;

// Treat this as a noop on web, as there is no backing storage which is "secure".
#[cfg(target_family = "wasm")]
use noop as imp;

/// A type alias for the concrete type stored within a zap_ng_ui
/// app context, enabling usage such as:
///
/// ```
/// use zap_ng_ui::{App, SingletonEntity};
/// use zap_ng_ui_extras::secure_storage;
///
/// App::test((), |mut app| async move {
///     app.update(|ctx| {
///         secure_storage::register("service_name", ctx);
///
///         let _ = secure_storage::Model::handle(ctx).as_ref(ctx).read_value("some_key");
///     });
/// });
/// ```
pub type Model = Box<dyn SecureStorage>;

/// Registers a platform-native Secure Storage provider with the application.
///
/// The service name is used as a namespace for the application's secrets.  It
/// is recommended that this be a unique identifier for the application; one
/// common scheme is reverse-DNS notation (e.g.: "dev.warp.ZapNG").
pub fn register(service_name: &str, ctx: &mut zap_ng_ui::AppContext) {
    ctx.add_singleton_model(|_| -> Model { Box::new(imp::SecureStorage::new(service_name)) });
}

/// Registers a no-op Secure Storage provider with the application.
pub fn register_noop(service_name: &str, ctx: &mut zap_ng_ui::AppContext) {
    ctx.add_singleton_model(|_| -> Model { Box::new(noop::SecureStorage::new(service_name)) });
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn register_with_fallback(
    service_name: &str,
    fallback_dir: std::path::PathBuf,
    ctx: &mut zap_ng_ui::AppContext,
) {
    ctx.add_singleton_model(|_| -> Model {
        Box::new(imp::SecureStorage::new_with_fallback(
            service_name,
            fallback_dir,
        ))
    });
}

/// Opens the secure storage of `service_name` on its own, without registering it with an
/// application: before the application is set up, or for another application's secrets.
///
/// On Linux, `fallback_dir` is where secrets are kept when there is no secret service, as
/// [`register_with_fallback`] takes it.
#[cfg(not(target_family = "wasm"))]
pub fn open(service_name: &str, fallback_dir: Option<std::path::PathBuf>) -> Model {
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    if let Some(fallback_dir) = fallback_dir {
        return Box::new(imp::SecureStorage::new_with_fallback(
            service_name,
            fallback_dir,
        ));
    }
    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    let _ = fallback_dir;
    Box::new(imp::SecureStorage::new(service_name))
}

/// A trait representing a secure store for key-value pairs.
///
/// This is typically backed by an OS-provided secure storage system.
pub trait SecureStorage {
    /// Writes a value at the given key.
    fn write_value(&self, key: &str, value: &str) -> Result<(), Error>;

    /// Reads the value stored at the given key.
    fn read_value(&self, key: &str) -> Result<String, Error>;

    /// Removes the value stored at the given key, if any.
    fn remove_value(&self, key: &str) -> Result<(), Error>;
}

impl zap_ng_ui::Entity for Model {
    type Event = ();
}

impl zap_ng_ui::SingletonEntity for Model {}

/// Enumerates the various errors that can occur when interacting with secure
/// storage.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The item with the given key was not found in secure storage.
    ///
    /// This is not guaranteed to be returned in all cases where the item is
    /// not found; if we are not able to interpret the error returned by the
    /// underlying implementation, [`SecureStorageError::Unknown`] may be
    /// returned.
    #[error("item not found")]
    NotFound,

    /// Failed to decode the stored bytes into a UTF-8 string.
    #[error("failed to decode UTF-8 string from bytes")]
    DecodeError(#[from] std::str::Utf8Error),

    /// Catch-all for unclassifiable errors.
    #[error("unknown error")]
    Unknown(#[from] anyhow::Error),
}

/// An extension trait to make secure storage easier to use.
///
/// ```
/// use zap_ng_ui::{App, SingletonEntity};
/// use zap_ng_ui_extras::secure_storage;
///
/// App::test((), |mut app| async move {
///     app.update(|ctx| {
///         secure_storage::register("service_name", ctx);
///
///         use secure_storage::AppContextExt;
///         let _ = ctx.secure_storage().read_value("some_key");
///     });
/// });
/// ```
pub trait AppContextExt {
    fn secure_storage(&self) -> &dyn SecureStorage;
}

impl AppContextExt for zap_ng_ui::AppContext {
    fn secure_storage(&self) -> &dyn SecureStorage {
        use zap_ng_ui::SingletonEntity;

        <Model as SingletonEntity>::as_ref(self).as_ref()
    }
}
