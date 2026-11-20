// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::integration_testing::command_palette::open_command_palette_and_run_action;
use crate::integration_testing::view_getters::workspace_view;
use zap_ng_ui::async_assert;
use zap_ng_ui::integration::TestStep;

pub fn open_theme_picker() -> Vec<TestStep> {
    let mut steps = open_command_palette_and_run_action("Open Theme Picker");
    let last = steps.pop().expect("steps should not be empty");
    steps.push(last.add_assertion(|app, window_id| {
        let workspace = workspace_view(app, window_id);
        workspace.read(app, |workspace, _| {
            async_assert!(
                workspace.is_theme_chooser_open(),
                "Theme chooser should be open"
            )
        })
    }));
    steps
}
