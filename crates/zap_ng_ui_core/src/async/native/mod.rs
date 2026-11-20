// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

pub mod executor;

pub use async_io::{block_on, Timer};
use futures::Future;

pub use futures_util::future::BoxFuture;

trait_set::trait_set! {
    /// A trait representing a task which can be run in the background.
    pub trait Spawnable = 'static + Future + Send;
    /// A trait representing a stream which can be polled in the background.
    pub trait Stream = 'static + futures::Stream + Send;
    /// A trait representing a value which can be returned from a background
    /// task.
    pub trait SpawnableOutput = Send;
    /// Bounds for async I/O streams passed to cross-platform networking code.
    /// On native, streams must be `Send` for the multi-threaded tokio runtime.
    pub trait TransportStream = Unpin + Send + 'static;
}
