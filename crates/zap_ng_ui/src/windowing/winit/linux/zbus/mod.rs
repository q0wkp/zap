// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

mod desktop_settings;
mod network_status;
mod suspend_resume;

pub use desktop_settings::*;
pub use network_status::watch_network_status_changed;
pub use suspend_resume::watch_suspend_resume_changes;
