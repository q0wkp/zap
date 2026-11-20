// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

//! Deserialization types for ripgrep JSON output.
use string_offset::ByteOffset;

#[derive(serde::Deserialize)]
pub(crate) struct RipgrepPath {
    pub text: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct RipgrepLines {
    pub text: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct RipgrepSubmatch {
    pub start: ByteOffset,
    pub end: ByteOffset,
}

#[derive(serde::Deserialize)]
pub(crate) struct RipgrepMatchData {
    pub path: RipgrepPath,
    pub lines: RipgrepLines,
    pub line_number: u32,
    #[serde(default)]
    pub submatches: Vec<RipgrepSubmatch>,
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum RipgrepMessage {
    #[serde(rename = "begin")]
    Begin,
    #[serde(rename = "match")]
    Match { data: RipgrepMatchData },
    #[serde(rename = "end")]
    End,
}
