// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: dropped the Windows-only code; renamed the Warp and Zap names to ZapNG.

// We can use `std::process:Command` here because this is invoked within a build script,
// _not_ within the ZapNG binary.
#![allow(clippy::disallowed_types)]

use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        macos: { target_os = "macos" },
        native: { not(target_family = "wasm") },
    }
}
