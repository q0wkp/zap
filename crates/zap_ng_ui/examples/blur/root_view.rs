// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use pathfinder_color::ColorU;
use zap_ng_ui::{elements::Rect, AppContext, Element, Entity, TypedActionView, View};

pub struct BlurredView {}

impl Entity for BlurredView {
    type Event = ();
}
impl View for BlurredView {
    fn ui_name() -> &'static str {
        "RootView"
    }

    /// Renders a transparent red rectangle. The blur effect is applied on the window (see
    /// `open_new()`.
    fn render(&self, _: &AppContext) -> Box<dyn Element> {
        Rect::new()
            .with_background_color(ColorU::new(255, 0, 0, 50))
            .finish()
    }
}

impl TypedActionView for BlurredView {
    type Action = ();
}
