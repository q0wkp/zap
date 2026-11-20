// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: added the split TOML preferences module, made the preferences file be replaced atomically, and removed the Windows registry-backed storage; renamed the Warp and Zap names to ZapNG.

//! Storage for user preferences.

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

pub mod file_backed;
pub mod in_memory;
#[cfg(target_family = "wasm")]
pub mod local_storage;
#[cfg(feature = "user_preferences-toml")]
pub mod split_toml;
#[cfg(feature = "user_preferences-toml")]
pub mod toml_backed;
#[cfg(target_os = "macos")]
pub mod user_defaults;

/// A type alias for a boxed user preferences backend.
pub type Model = Box<dyn UserPreferences>;

/// Replaces the contents of the preferences file at `file_path` with `data`, creating the file
/// and its parent directory if they don't exist.
///
/// The data is written to a temporary file next to the target, which is then renamed over it.
/// Writing the file in place truncates it first, so a crash, a kill or a full disk in the middle
/// of the write would leave a truncated file behind and lose every preference in it; with the
/// rename, the file holds either the previous contents or the new ones. The data is not synced
/// to disk, so this does not cover a power loss: preferences are written often, and a sync on
/// every write would be too slow.
///
/// A symbolic link at `file_path` stays in place and the file it points to is replaced, and an
/// existing file keeps its permissions. A read-only file is not replaced. Where the temporary
/// file can't be created or renamed (a directory that isn't writable, a file that is itself a
/// mount point), the file is written in place as a fallback.
fn write_file_atomically(file_path: &Path, data: &[u8]) -> std::io::Result<()> {
    let target = std::fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf());
    let parent_dir = target
        .parent()
        .expect("absolute path to file should have parent");
    std::fs::create_dir_all(parent_dir)?;

    let existing_permissions = std::fs::metadata(&target)
        .ok()
        .map(|metadata| metadata.permissions());
    if existing_permissions
        .as_ref()
        .is_some_and(|permissions| permissions.readonly())
    {
        // A rename would replace a file that was made read-only to keep it from being written.
        // Write in place, which fails as it should.
        return std::fs::write(&target, data);
    }

    let temporary_path = temporary_sibling_path(&target);
    let Ok(mut temporary_file) = File::create(&temporary_path) else {
        return std::fs::write(&target, data);
    };

    let written = temporary_file
        .write_all(data)
        .and_then(|()| match existing_permissions {
            Some(permissions) => temporary_file.set_permissions(permissions),
            None => Ok(()),
        });
    drop(temporary_file);
    if let Err(error) = written {
        // The previous contents are still in place, so report the failure instead of risking
        // them with a write in place.
        let _ = std::fs::remove_file(&temporary_path);
        return Err(error);
    }

    if std::fs::rename(&temporary_path, &target).is_err() {
        let _ = std::fs::remove_file(&temporary_path);
        return std::fs::write(&target, data);
    }
    Ok(())
}

/// Returns a path in the directory of `target` that no other write uses, in this process or in
/// another one writing the same file.
fn temporary_sibling_path(target: &Path) -> PathBuf {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    let mut file_name = target
        .file_name()
        .expect("path to file should have a file name")
        .to_owned();
    file_name.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ));
    target.with_file_name(file_name)
}

/// A trait representing storage for user preferences.
pub trait UserPreferences {
    /// Writes a value at the given key.
    fn write_value(&self, key: &str, value: String) -> Result<(), Error>;

    /// Reads the value stored at the given key.
    ///
    /// Returns Ok(None) if no value was found.
    fn read_value(&self, key: &str) -> Result<Option<String>, Error>;

    /// Removes the value stored at the given key, if any.
    fn remove_value(&self, key: &str) -> Result<(), Error>;

    /// Writes a value at the given key, with optional hierarchy context.
    ///
    /// Hierarchy-aware backends (like TOML) use the hierarchy to place the
    /// value in the correct section. The default implementation ignores
    /// the hierarchy and delegates to [`write_value`](Self::write_value).
    ///
    /// `max_table_depth` controls how deeply nested objects are rendered as
    /// section tables before switching to inline tables:
    /// - `None` — unlimited depth (all section tables)
    /// - `Some(0)` — fully inline (`key = { ... }`)
    /// - `Some(n)` — `n` levels of section tables, then inline
    fn write_value_with_hierarchy(
        &self,
        key: &str,
        value: String,
        hierarchy: Option<&str>,
        max_table_depth: Option<u32>,
    ) -> Result<(), Error> {
        let _ = (hierarchy, max_table_depth);
        self.write_value(key, value)
    }

    /// Reads the value stored at the given key, with optional hierarchy context.
    ///
    /// The default implementation ignores the hierarchy and delegates to
    /// [`read_value`](Self::read_value).
    fn read_value_with_hierarchy(
        &self,
        key: &str,
        hierarchy: Option<&str>,
    ) -> Result<Option<String>, Error> {
        let _ = hierarchy;
        self.read_value(key)
    }

    /// Removes the value stored at the given key, with optional hierarchy context.
    ///
    /// The default implementation ignores the hierarchy and delegates to
    /// [`remove_value`](Self::remove_value).
    fn remove_value_with_hierarchy(&self, key: &str, hierarchy: Option<&str>) -> Result<(), Error> {
        let _ = hierarchy;
        self.remove_value(key)
    }

    /// Returns whether this backend is the user-visible settings file.
    ///
    /// When true, settings that define custom file serialization (via
    /// `file_serialize` / `file_deserialize`) will use their custom format
    /// instead of the standard serde representation. This produces a more
    /// human-readable settings file.
    ///
    /// Other backends (NSUserDefaults, in-memory, etc.) return `false` and
    /// always use the standard serde format.
    fn is_settings_file(&self) -> bool {
        false
    }

    /// Reloads the backing store from disk.
    ///
    /// File-backed backends re-read their file and replace the in-memory
    /// contents. Non-file backends do nothing. On parse failure the
    /// implementation should keep the previous state and return an error.
    fn reload_from_disk(&self) -> Result<(), Error> {
        Ok(())
    }

    /// Marks a key as write-inhibited so that subsequent writes and removes
    /// for this key are silently skipped.
    ///
    /// This is used to protect individual setting values that exist in the
    /// backing store but could not be deserialized into the expected type.
    /// The user's broken-but-fixable value is preserved until they correct
    /// it in the file.
    ///
    /// The default implementation is a no-op (non-file backends don't need
    /// per-key inhibition).
    fn inhibit_writes_for_key(&self, key: &str, hierarchy: Option<&str>) {
        let _ = (key, hierarchy);
    }

    /// Clears all per-key write inhibitions.
    ///
    /// Called after a successful reload from disk so that inhibitions can
    /// be re-derived from the freshly loaded values.
    fn clear_all_write_inhibitions(&self) {}
}

/// Enumerates the various errors that can occur when interacting with user
/// preferences.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to decode the stored bytes into a UTF-8 string.
    #[error("failed to decode UTF-8 string from bytes")]
    DecodeError(#[from] std::str::Utf8Error),

    /// Generic I/O error.
    #[error("i/o error")]
    IoError(#[from] std::io::Error),

    /// Catch-all for unclassifiable errors.
    #[error("unknown error")]
    Unknown(#[from] anyhow::Error),
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
