// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

// ZapNG: telemetry sending has been physically removed. These macros remain as
// compatibility shims for call sites that still describe local UI/business events. They
// type-check the event expression in an unreachable branch without evaluating it, so
// telemetry-only payload construction has no runtime cost while callers keep their existing
// type inference and imports. Keep the context/executor operands referenced to avoid churn
// in callers while the remaining event-type shell is removed incrementally.

#[macro_export]
macro_rules! send_telemetry_from_ctx {
    ($event:expr, $ctx:expr) => {{
        if false {
            let _ = &$event;
        }
        let _ = &$ctx;
    }};
}

#[macro_export]
macro_rules! send_telemetry_from_app_ctx {
    ($event:expr, $app_ctx:expr) => {{
        if false {
            let _ = &$event;
        }
        let _ = &$app_ctx;
    }};
}

#[macro_export]
macro_rules! send_telemetry_sync_from_ctx {
    ($event:expr, $ctx:expr) => {{
        if false {
            let _ = &$event;
        }
        let _ = &$ctx;
    }};
}

#[macro_export]
macro_rules! send_telemetry_sync_from_app_ctx {
    ($event:expr, $app_ctx:expr) => {{
        if false {
            let _ = &$event;
        }
        let _ = &$app_ctx;
    }};
}

#[macro_export]
macro_rules! send_telemetry_on_executor {
    ($auth_state:expr, $event:expr, $executor:expr) => {{
        if false {
            let _ = &$event;
        }
        let _ = &$auth_state;
        let _ = &$executor;
    }};
}
