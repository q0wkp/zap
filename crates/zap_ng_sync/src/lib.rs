// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

//! zap_ng_sync - 云同步通用基础设施
//!
// author: logic
// date: 2026-05-24

pub mod crypto;
pub mod gist_client;
pub mod sync_engine;
pub mod types;

pub use crypto::CryptoError;
pub use gist_client::{GistClient, GistClientError, GistOps};
pub use sync_engine::{SyncDataProvider, SyncEngine};
pub use types::*;
