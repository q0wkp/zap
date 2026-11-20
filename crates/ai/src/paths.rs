// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: removed the WSL and MSYS2 path conversions and the shell argument from the path helpers; renamed the Warp and Zap names to ZapNG.

use typed_path::{TypedPath, TypedPathBuf};
use zap_ng_ui::platform::OperatingSystem;

fn use_unix_paths() -> bool {
    OperatingSystem::get().is_linux() || OperatingSystem::get().is_mac()
}

pub fn join_paths(paths: &[&str]) -> String {
    let use_unix_paths = use_unix_paths();

    let base_path = if use_unix_paths {
        TypedPathBuf::unix()
    } else {
        TypedPathBuf::windows()
    };
    paths
        .iter()
        .fold(base_path, |acc, path| acc.join(path))
        .to_string_lossy()
        .into_owned()
}

fn shell_native_absolute_path_internal(
    file_path: &str,
    current_working_directory: &str,
) -> TypedPathBuf {
    let expanded_path = shellexpand::tilde(file_path).into_owned();

    let use_unix_paths = use_unix_paths();
    let (cwd, file_path) = if use_unix_paths {
        (
            TypedPathBuf::from_unix(current_working_directory),
            TypedPath::unix(&expanded_path),
        )
    } else {
        (
            TypedPathBuf::from_windows(current_working_directory),
            TypedPath::windows(&expanded_path),
        )
    };
    cwd.join(file_path).normalize()
}

/// Returns the absolute path of the path in the shell's native format.
///
/// On Unix systems, this will always be Unix encoded paths.
pub fn shell_native_absolute_path(
    file_path: &str,
    current_working_directory: Option<&String>,
) -> String {
    let Some(cwd) = current_working_directory else {
        return shellexpand::tilde(file_path).into_owned();
    };
    shell_native_absolute_path_internal(file_path, cwd)
        .to_string_lossy()
        .into_owned()
}

/// Returns the absolute path of the path in the host's native format.
///
/// This should be used over [`shell_native_absolute_path`] when we need an
/// absolute path in the format of the user's OS, regardless of what shell
/// they're using.
pub fn host_native_absolute_path(
    file_path: &str,
    current_working_directory: &Option<String>,
) -> String {
    let Some(cwd) = current_working_directory.as_ref() else {
        return shellexpand::tilde(file_path).into_owned();
    };
    shell_native_absolute_path_internal(file_path, cwd)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
