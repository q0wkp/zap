// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::crash_reporting::VirtualEnvironment;

/// Returns what virtualized environment ZapNG is running in, if any.
pub fn get_virtualized_environment() -> Option<VirtualEnvironment> {
    if let Ok(output) = command::blocking::Command::new("systemd-detect-virt").output() {
        if !output.status.success() {
            return None;
        } else {
            let value = std::str::from_utf8(&output.stdout).ok()?;
            if value == "none" {
                return None;
            }
            return Some(VirtualEnvironment {
                name: value.to_owned(),
            });
        }
    };

    // Test specifically for WSL based on existence of a particular file under
    // /proc.
    //
    // See: https://superuser.com/questions/1749781/how-can-i-check-if-the-environment-is-wsl-from-a-shell-script
    if std::path::Path::new("/proc/sys/fs/binfmt_misc/WSLInterop").exists() {
        return Some(VirtualEnvironment {
            name: "wsl".to_owned(),
        });
    }

    None
}
