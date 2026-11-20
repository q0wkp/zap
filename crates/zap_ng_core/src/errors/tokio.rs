// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::{register_error, ErrorExt};

impl ErrorExt for tokio::task::JoinError {
    fn is_actionable(&self) -> bool {
        // If the task was cancelled (aborted), this is expected behavior and not actionable.
        if self.is_cancelled() {
            return false;
        }

        // If the task panicked, this is actionable - we need to know about panics.
        if self.is_panic() {
            return true;
        }

        // Other join errors are actionable.
        true
    }
}
register_error!(tokio::task::JoinError);
