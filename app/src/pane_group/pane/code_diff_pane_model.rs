// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::{Entity, ModelContext, ViewHandle};

use crate::ai::blocklist::inline_action::code_diff_view::{CodeDiffView, CodeDiffViewEvent};

/// Intermediate model between CodeDiffPane and CodeDiffView.
/// This model is needed because if the PaneGroup subscribes directly to the CodeDiffView,
/// then the PaneGroup could not unsubscribe during detach.
/// This is because unsubscribing does not work when handling an event from the subscribed view.
pub struct CodeDiffPaneModel {}

impl CodeDiffPaneModel {
    pub fn new(view: ViewHandle<CodeDiffView>, ctx: &mut ModelContext<Self>) -> Self {
        // Subscribe to the CodeDiffView events
        ctx.subscribe_to_view(&view, |_model, event, ctx| ctx.emit(event.clone()));

        Self {}
    }
}

impl Entity for CodeDiffPaneModel {
    type Event = CodeDiffViewEvent;
}
