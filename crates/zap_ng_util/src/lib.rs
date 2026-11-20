// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: removed the Windows-only module; renamed the Warp and Zap names to ZapNG.

//! This crate contains generic utilities and helpers available for use across all internal zap-ng
//! crates.
//!
//! Generally, if a given function/abstraction is useful outside of a single warp-internal crate
//! but isn't large/complex enough to warrant its own crate, it belongs here.
pub mod assets;
pub mod content_version;
pub mod file;
pub mod file_type;
pub mod on_cancel;
pub mod path;
pub mod standardized_path;
pub mod user_input;
pub mod worktree_names;
