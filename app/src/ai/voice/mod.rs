// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

//! This module contains all code relevant to Voice within ZapNG.
//!
//! Voice is used for voice input within ZapNG.

// ZapNG Wave 6-1:`pub(crate) mod transcribe` 随 `ServerApi::transcribe` 一同物理删。
// 原子模块 `transcribe/api/{request,response}` 仅为已删除的云端 `/ai/transcribe` 端点
// 的 wire 类型。本地语音走 `voice/transcriber.rs::Transcriber` trait + `TranscribeError`。
