// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

//! This module contains all code relevant to Agent Predict within ZapNG.
//!
//! Agent Predict attempts to predict the next action the user will take in ZapNG.

pub(crate) mod generate_ai_input_suggestions;
pub(crate) mod generate_am_query_suggestions;
pub mod next_command_model;
// ZapNG(Wave 3-2):`predict_am_queries` API 模块已物理删 — 原 `ServerApi::predict_am_queries`
// 0 外部消费已同步删除；FeatureFlag::PredictAMQueries / terminal/input.rs 中
// `predict_am_queries_future_handle` 仅作为控制开关/句柄代号保留，不再需要该模块。
pub mod prompt_suggestions;
