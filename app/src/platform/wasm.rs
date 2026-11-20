// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use js_sys::ReferenceError;
use thiserror::Error;
use wasm_bindgen::{JsCast, JsValue};

pub use zap_ng_web_event_bus::{emit_event, ZapNGEvent};

/// This function should be called early in application initialization to ensure that
/// static variables are initialized.
pub(super) fn init() {
    unsafe {
        extern "C" {
            /// __wasm_call_ctors is a function defined by the `wasm-ld` linker, and is used to
            /// initialize static variables.
            ///
            /// It should be called once at runtime before other code is executed.
            fn __wasm_call_ctors();
        }

        __wasm_call_ctors();
    }
}

mod ffi {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = "warpUserHandoff", catch)]
        pub fn user_handoff() -> Result<Option<String>, JsValue>;
    }
}

#[derive(Debug, Clone, Error)]
pub enum AuthHandoffError {
    #[error("The host page doesn't support user handoff")]
    Unsupported,
    #[error("Unexpected handoff error: {0:?}")]
    Unexpected(JsValue),
}

/// Fetch the host app's optional local user handoff token.
pub fn user_handoff() -> Result<Option<String>, AuthHandoffError> {
    ffi::user_handoff().map_err(|err| {
        if ReferenceError::instanceof(&err) {
            AuthHandoffError::Unsupported
        } else {
            AuthHandoffError::Unexpected(err)
        }
    })
}
