// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::{View, ViewContext};

pub trait OnboardingSlide: View {
    fn on_up(&mut self, _ctx: &mut ViewContext<Self>) {}
    fn on_down(&mut self, _ctx: &mut ViewContext<Self>) {}
    fn on_left(&mut self, _ctx: &mut ViewContext<Self>) {}
    fn on_right(&mut self, _ctx: &mut ViewContext<Self>) {}
    fn on_tab(&mut self, _ctx: &mut ViewContext<Self>) {}
    fn on_enter(&mut self, _ctx: &mut ViewContext<Self>) {}
    fn on_cmd_or_ctrl_enter(&mut self, _ctx: &mut ViewContext<Self>) {}
    fn on_escape(&mut self, _ctx: &mut ViewContext<Self>) {}
}
