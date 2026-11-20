// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: removed the Windows-only module; renamed the Warp and Zap names to ZapNG.

pub(crate) mod app;
pub mod delegate;
mod event_loop;
pub(crate) mod fonts;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub mod linux;

mod notifications;
#[cfg(target_family = "wasm")]
pub mod wasm;

mod window;

use app::CustomEvent;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub use app::WindowingSystem;
use event_loop::EventLoop;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub use window::get_os_window_manager_name;
use window::Window;
