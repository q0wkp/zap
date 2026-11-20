// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

//! A headless implementation of the UI framework's platform abstraction.
//!
//! This provides enough functionality to run an app, but no GUI or visible output.

mod app;
mod delegate;
mod event_loop;
mod windowing;

pub use app::App;
pub use delegate::AppDelegate;

#[cfg(target_os = "macos")]
pub(crate) use windowing::Window;
