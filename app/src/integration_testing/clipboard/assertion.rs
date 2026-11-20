// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::{async_assert_eq, integration::AssertionCallback};

pub fn assert_clipboard_contains_string(string: String) -> AssertionCallback {
    Box::new(move |app, _window_id| {
        let clipboard = app.update(|ctx| ctx.clipboard().read());
        let content = match clipboard.paths {
            Some(paths) => paths.join(" "),
            None => clipboard.plain_text,
        };

        async_assert_eq!(content, string)
    })
}
