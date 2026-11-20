// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

pub mod app_id;
pub mod assertions;
pub mod channel;
pub mod command;
pub mod context_flag;
pub mod errors;
pub mod execution_mode;
pub mod features;
pub mod interval_timer;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod operating_system_info;
pub mod paths;
pub mod platform;
pub mod safe_log;
pub mod semantic_selection;
pub use settings;
// Re-export settings macros for backward compatibility
pub use settings::{
    define_setting, define_settings_group, implement_setting_for_enum, maybe_define_setting,
};
pub mod host_id;
pub mod session_id;
pub mod sync_queue;
pub mod telemetry;
pub mod ui;
pub mod user_preferences;

pub use app_id::AppId;
pub use host_id::HostId;
pub use session_id::SessionId;
