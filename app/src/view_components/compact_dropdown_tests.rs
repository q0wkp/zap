// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_core::ui::appearance::Appearance;
use zap_ng_ui::{platform::WindowStyle, App, View};

use crate::{menu::MenuVariant, ui_components::icons::Icon};

use super::{CompactDropdown, CompactDropdownItem};

#[derive(Debug, Clone)]
struct TestAction;

/// Baseline test that the view can render.
#[test]
fn test_render() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());
        let (_, view) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            CompactDropdown::<TestAction>::new(MenuVariant::Fixed, ctx)
        });

        // This should not panic.
        view.read(&app, |view, ctx| view.render(ctx));

        // After adding some items, rendering should still not panic.
        view.update(&mut app, |view, ctx| {
            view.set_items(
                [
                    CompactDropdownItem::new(Icon::Folder, "Folder", TestAction),
                    CompactDropdownItem::new(Icon::Gear, "Gear", TestAction),
                ],
                ctx,
            );
        });
        view.read(&app, |view, ctx| view.render(ctx));
    })
}
