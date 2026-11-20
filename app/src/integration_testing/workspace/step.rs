// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::{async_assert, integration::TestStep, SingletonEntity};

use crate::{
    integration_testing::view_getters::workspace_view, undo_close::UndoCloseStack,
    workspace::Workspace,
};

/// Mock pressing a button on the ZapNG-native quit modal. Note that this modal is currently only
/// used on Linux, not macOS.
pub fn press_native_modal_button(button_index: usize) -> TestStep {
    TestStep::new("Press a native modal button")
        .with_action(move |app, _, _data| {
            let active_window = app
                .read(|ctx| ctx.windows().active_window())
                .expect("no active window");
            let workspace = workspace_view(app, active_window);
            app.update(|ctx| {
                assert!(
                    workspace.as_ref(ctx).is_native_quit_modal_open(ctx),
                    "Native modal should be open"
                );
                Workspace::press_native_modal_button(&workspace, button_index, ctx);
            });
        })
        .add_assertion(|app, window_id| {
            let workspace = workspace_view(app, window_id);
            workspace.read(app, |workspace, ctx| {
                async_assert!(
                    !workspace.is_native_quit_modal_open(ctx),
                    "Native modal is still open"
                )
            })
        })
}

/// Trigger undo close (restore closed pane/tab/window) action.
pub fn trigger_undo_close() -> TestStep {
    TestStep::new("Trigger undo close").with_action(move |app, _, _data| {
        app.update(|ctx| {
            UndoCloseStack::handle(ctx).update(ctx, |stack, model_ctx| {
                stack.undo_close(model_ctx);
            });
        });
    })
}
