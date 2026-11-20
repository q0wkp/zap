// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

/// Adjusts resource limits applied to the ZapNG process (e.g.: the limit on open
/// file descriptors) to ensure proper behavior.
pub fn adjust_resource_limits() {
    #[cfg(target_os = "macos")]
    {
        /// Our desired limit on the maximum number of file descriptors we can open.
        ///
        /// MacOS sets this value to 256, by default, which can interfere with
        /// normal usage of ZapNG for users who open lots of tabs.
        const TARGET_FD_LIMIT: u64 = 2560;

        use nix::sys::resource::{getrlimit, setrlimit, Resource::RLIMIT_NOFILE};

        let (cur_limit, hard_limit) = match getrlimit(RLIMIT_NOFILE) {
            Ok(val) => val,
            Err(err) => {
                log::error!("Failed to retrieve resource limit for number of files: {err:#}");
                return;
            }
        };

        log::info!(
            "Initial open file descriptor limit is {cur_limit}, with a hard limit of {hard_limit}"
        );
        let new_limit = TARGET_FD_LIMIT.min(hard_limit);
        if cur_limit < new_limit {
            match setrlimit(RLIMIT_NOFILE, new_limit, hard_limit) {
                Ok(_) => log::info!("Increased open file descriptor limit to {new_limit}"),
                Err(err) => log::error!("Failed to increase open file descriptor limit: {err:#}"),
            }
        }
    }
}
