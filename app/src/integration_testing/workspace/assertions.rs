// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::{async_assert_eq, integration::AssertionCallback};

use crate::integration_testing::view_getters::workspace_view;

pub fn assert_focused_tab_index(tab_index: usize) -> AssertionCallback {
    Box::new(move |app, window_id| {
        let workspace = workspace_view(app, window_id);
        workspace.read(app, |view, _ctx| {
            async_assert_eq!(view.active_tab_index(), tab_index)
        })
    })
}

/// Assert that there are a particular number of tabs in the workspace.
pub fn assert_tab_count(tab_count: usize) -> AssertionCallback {
    Box::new(move |app, window_id| {
        let workspace = workspace_view(app, window_id);
        workspace.read(app, |view, _ctx| {
            let actual_tab_count = view.tab_count();
            async_assert_eq!(
                actual_tab_count,
                tab_count,
                "Expected {} tabs, but there were {}",
                tab_count,
                actual_tab_count
            )
        })
    })
}
