// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: removed the Windows-only module path; renamed the Warp and Zap names to ZapNG.

//! Module containing the definition of a platform agnostic audible bell.

use anyhow::Result;
use zap_ng_ui::{Entity, SingletonEntity};

#[cfg_attr(any(target_os = "linux", target_os = "freebsd"), path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
// TODO(WASM): Replace this with a functional implementation for the web.
#[cfg_attr(target_family = "wasm", path = "noop.rs")]
mod imp;

/// A singleton model that provides a way convenient way to make a "beep" when rung (via a call to
/// [`AudibleBell::ring`]).
pub struct AudibleBell(imp::AudibleBell);

impl AudibleBell {
    pub fn new() -> Self {
        Self(imp::AudibleBell::new())
    }

    /// Rings the audible bell. Returns an [`Err`] if the bell was unable to be rung for any reason.
    pub fn ring(&self) -> Result<()> {
        self.0.ring()
    }
}

impl Default for AudibleBell {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for AudibleBell {
    type Event = ();
}

impl SingletonEntity for AudibleBell {}
