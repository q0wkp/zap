// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

pub mod ai_agent_tasks;
pub mod blockgrid;
pub mod settings;
pub mod terminal;
mod virtual_fs;

pub use blockgrid::mock_blockgrid;
pub use terminal::add_window_with_terminal;
pub use virtual_fs::{Stub, VirtualFS};

macro_rules! assert_eventually {
    ($cond:expr, $($arg:tt)+) => {
        $crate::test_util::assert_eventually!(20 => $cond, $($arg)+);
    };
    // Run the condition up to ticks times, yielding to the executor in between.  If it does
    // not become true, this panics with the provided format string + args.
    ($ticks:literal => $cond:expr, $($arg:tt)+) => {{
        let mut pass = false;
        for _ in 0..$ticks {
            if $cond {
                pass = true;
                break;
            }
            zap_ng_ui::r#async::Timer::after(std::time::Duration::from_millis(5)).await;
        }
        if !pass {
            panic!("{}", format_args!($($arg)+));
        }
    }};
}
pub(crate) use assert_eventually;
