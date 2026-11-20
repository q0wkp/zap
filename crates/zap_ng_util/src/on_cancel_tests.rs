// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::on_cancel::OnCancelFutureExt;
use futures_util::future::{AbortHandle, Abortable, Aborted};
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn test_ready_future_doesnt_call_callback() {
    let callback_called = AtomicBool::new(false);

    let future = async {}.on_cancel(|| callback_called.store(true, Ordering::SeqCst));
    zap_ng_ui::r#async::block_on(future);

    assert!(!callback_called.load(Ordering::Relaxed));
}

#[test]
fn test_aborted_future_calls_callback() {
    let callback_called = AtomicBool::new(false);

    let (handle, registration) = AbortHandle::new_pair();
    let future = Abortable::new(
        async {}.on_cancel(|| callback_called.store(true, Ordering::SeqCst)),
        registration,
    );

    // Abort the future before it is ever polled.
    handle.abort();
    let future_result = zap_ng_ui::r#async::block_on(future);

    // The future should be aborted and the callback should have been called.
    assert_eq!(future_result, Err(Aborted));
    assert!(callback_called.load(Ordering::Relaxed));
}
