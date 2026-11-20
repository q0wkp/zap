// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: limited the crash-recovery cfg alias to Linux, dropping Windows; renamed the Warp and Zap names to ZapNG.

use cfg_aliases::cfg_aliases;

fn main() {
    // This sets the same cfg aliases as the `zap-ng` crate, used to gate crash-recovery flags.
    cfg_aliases! {
        enable_crash_recovery: { target_os = "linux" },
    }
}
