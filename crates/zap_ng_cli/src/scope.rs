// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use clap::Args;

/// Common args for scoping objects to team or personal drives.
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = false)]
pub struct ObjectScope {
    /// Create at the team level.
    #[arg(long, group = "scope")]
    pub team: bool,
    /// Create as private to your account.
    #[arg(long, conflicts_with = "team", group = "scope")]
    pub personal: bool,
}
