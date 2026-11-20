// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: removed the Windows platform module; renamed the Warp and Zap names to ZapNG.

pub mod app;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod mac;
#[cfg(target_family = "wasm")]
pub mod wasm;

pub mod headless;

pub mod current {
    cfg_if::cfg_if! {
        if #[cfg(target_family = "wasm")] {
            pub use super::wasm::*;
        } else if #[cfg(any(target_os = "linux", target_os = "freebsd"))] {
            pub use super::linux::*;
        } else if #[cfg(target_os = "macos")] {
            pub use super::mac::*;
        } else {
            pub use zap_ng_ui_core::platform::test::*;
        }
    }
}

pub use zap_ng_ui_core::platform::*;

pub use app::AppBuilder;

/// Returns whether the current device is a mobile device with touch input.
///
/// This is a cross-platform wrapper around the platform-specific implementation.
pub fn is_mobile_device() -> bool {
    #[cfg(target_family = "wasm")]
    {
        wasm::is_mobile_device()
    }
    #[cfg(not(target_family = "wasm"))]
    {
        false
    }
}

/// A trait for accessing internal per-platform concrete implementations
/// through a wrapper type.
#[allow(dead_code)]
trait AsInnerMut<Inner: ?Sized> {
    fn as_inner_mut(&mut self) -> &mut Inner;
}
