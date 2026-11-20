// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::integration_testing::view_getters::workspace_view;
use zap_ng_ui::async_assert;
use zap_ng_ui::integration::AssertionCallback;

pub fn assert_workflow_modal_is_open() -> AssertionCallback {
    Box::new(move |app, window_id| {
        let workspace = workspace_view(app, window_id);

        workspace.read(app, |workspace, _| {
            async_assert!(
                workspace.is_workflow_modal_open(),
                "Expected workflow modal to be open, but it was closed"
            )
        })
    })
}

pub fn assert_workflow_modal_is_closed() -> AssertionCallback {
    Box::new(move |app, window_id| {
        let workspace = workspace_view(app, window_id);

        workspace.read(app, |workspace, _| {
            async_assert!(
                !workspace.is_workflow_modal_open(),
                "Expected workflow modal to be closed, but it was open"
            )
        })
    })
}

pub fn assert_zap_ng_drive_is_open() -> AssertionCallback {
    Box::new(move |app, window_id| {
        let workspace = workspace_view(app, window_id);

        workspace.read(app, |workspace, _| {
            async_assert!(
                workspace.is_zap_ng_drive_open(),
                "Expected ZapNG Drive to be open, but it was closed"
            )
        })
    })
}

pub fn assert_zap_ng_drive_is_closed() -> AssertionCallback {
    Box::new(move |app, window_id| {
        let workspace = workspace_view(app, window_id);

        workspace.read(app, |workspace, _| {
            async_assert!(
                !workspace.is_zap_ng_drive_open(),
                "Expected ZapNG Drive to be closed, but it was open"
            )
        })
    })
}

pub fn assert_is_left_panel_open() -> AssertionCallback {
    Box::new(move |app, window_id| {
        let workspace = workspace_view(app, window_id);

        workspace.read(app, |workspace, ctx| {
            async_assert!(
                workspace.is_left_panel_open(ctx),
                "Expected left panel to be open, but it was closed"
            )
        })
    })
}
