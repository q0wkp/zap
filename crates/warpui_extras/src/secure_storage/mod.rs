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

/// A type alias for the concrete type stored within a warpui
/// app context, enabling usage such as:
///
/// ```
/// use warpui::{App, SingletonEntity};
/// use warpui_extras::secure_storage;
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
/// common scheme is reverse-DNS notation (e.g.: "dev.warp.Zap").
pub fn register(service_name: &str, ctx: &mut warpui::AppContext) {
    ctx.add_singleton_model(|_| -> Model { Box::new(imp::SecureStorage::new(service_name)) });
}

/// Registers a no-op Secure Storage provider with the application.
pub fn register_noop(service_name: &str, ctx: &mut warpui::AppContext) {
    ctx.add_singleton_model(|_| -> Model { Box::new(noop::SecureStorage::new(service_name)) });
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn register_with_fallback(
    service_name: &str,
    fallback_dir: std::path::PathBuf,
    ctx: &mut warpui::AppContext,
) {
    ctx.add_singleton_model(|_| -> Model {
        Box::new(imp::SecureStorage::new_with_fallback(
            service_name,
            fallback_dir,
        ))
    });
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

impl warpui::Entity for Model {
    type Event = ();
}

impl warpui::SingletonEntity for Model {}

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
/// use warpui::{App, SingletonEntity};
/// use warpui_extras::secure_storage;
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

impl AppContextExt for warpui::AppContext {
    fn secure_storage(&self) -> &dyn SecureStorage {
        use warpui::SingletonEntity;

        <Model as SingletonEntity>::as_ref(self).as_ref()
    }
}
