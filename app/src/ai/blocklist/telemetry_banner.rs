// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::AppContext;

/// ZapNG has no telemetry sender and must not collect user-generated AI data.
pub fn should_collect_ai_ugc_telemetry(_app: &AppContext, _is_telemetry_enabled: bool) -> bool {
    false
}
