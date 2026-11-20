// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::assert_clipboard_contains_string;
use zap_ng_ui::{clipboard::ClipboardContent, integration::TestStep};

pub fn write_to_clipboard(text: String) -> TestStep {
    let expected = text.clone();
    TestStep::new("Write text to the clipboard")
        .with_action(move |app, _, _| {
            app.update(|app| {
                app.clipboard()
                    .write(ClipboardContent::plain_text(text.clone()))
            })
        })
        .add_named_assertion(
            "Ensure the clipboard contains the text",
            assert_clipboard_contains_string(expected),
        )
}
