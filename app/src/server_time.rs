// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use anyhow::{anyhow, Result};
use chrono::{DateTime, FixedOffset, Utc};
use instant::Instant;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct ServerTimestamp(DateTime<Utc>);

impl ServerTimestamp {
    pub fn new(time: DateTime<Utc>) -> Self {
        Self(time)
    }

    pub fn from_unix_timestamp_micros(ms_since_epoch: i64) -> Result<Self> {
        let date_time = DateTime::from_timestamp_micros(ms_since_epoch)
            .ok_or_else(|| anyhow!("Unable to convert microseconds into NaiveDateTime"))?;
        Ok(ServerTimestamp::new(date_time))
    }

    pub fn timestamp_micros(&self) -> i64 {
        self.0.timestamp_micros()
    }

    pub fn utc(&self) -> DateTime<Utc> {
        self.0
    }
}

impl From<DateTime<Utc>> for ServerTimestamp {
    fn from(value: DateTime<Utc>) -> Self {
        ServerTimestamp::new(value)
    }
}

/// 本地估算的服务端时间。
///
/// ZapNG 不再请求云端 `/current_time`;启动路径用本地当前时间初始化,调用方仍可
/// 通过该类型获得随单调时钟推进的 wall-clock 时间。
#[derive(Debug, Clone)]
pub struct ServerTime {
    time_at_fetch: DateTime<FixedOffset>,
    fetched_at: Instant,
}

impl ServerTime {
    pub(crate) fn local_now() -> Self {
        Self {
            time_at_fetch: chrono::Utc::now().into(),
            fetched_at: Instant::now(),
        }
    }

    pub(crate) fn current_time(&self) -> DateTime<FixedOffset> {
        let elapsed = chrono::Duration::from_std(self.fetched_at.elapsed())
            .expect("duration should not be bigger than limit");
        self.time_at_fetch + elapsed
    }
}
