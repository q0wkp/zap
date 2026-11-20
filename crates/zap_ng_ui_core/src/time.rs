// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

#[cfg(test)]
use chrono::TimeZone;
use chrono::{DateTime, Utc};
#[cfg(test)]
use std::sync::atomic::{AtomicI64, Ordering};

#[cfg(not(test))]
pub fn get_current_time() -> DateTime<Utc> {
    Utc::now()
}

thread_local! {
    #[cfg(test)]
    static CURRENT_SECS: AtomicI64 = const { AtomicI64::new(0) };
}
#[cfg(test)]
pub fn get_current_time() -> DateTime<Utc> {
    CURRENT_SECS.with(|current_secs| {
        Utc.timestamp_opt(current_secs.load(Ordering::SeqCst), 0)
            .unwrap()
    })
}
#[cfg(test)]
pub fn test_offset_time(offset_secs: i64) {
    CURRENT_SECS.with(|current_secs| {
        current_secs.fetch_add(offset_secs, Ordering::SeqCst);
    })
}
