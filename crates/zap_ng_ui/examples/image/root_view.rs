// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use pathfinder_color::ColorU;
use zap_ng_ui::{
    elements::{
        CacheOption, ConstrainedBox, Flex, Icon, Image, MainAxisAlignment, MainAxisSize,
        ParentElement, Rect, Stack,
    },
    AppContext, Element, Entity, TypedActionView, View,
};

pub struct RootView {}

impl RootView {
    pub fn new() -> Self {
        RootView {}
    }
}

impl Default for RootView {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for RootView {
    type Event = ();
}
impl View for RootView {
    fn ui_name() -> &'static str {
        "RootView"
    }

    fn render(&self, _: &AppContext) -> Box<dyn Element> {
        let asset_source = ::asset_cache::url_source(
            "https://i.ebayimg.com/images/g/B~gAAOSwhNthhdjn/s-l1600.jpg",
        );

        Stack::new()
            .with_child(Rect::new().with_background_color(ColorU::white()).finish())
            .with_child(
                Flex::column()
                    .with_main_axis_alignment(MainAxisAlignment::Center)
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_child(
                        Flex::row()
                            .with_main_axis_alignment(MainAxisAlignment::Center)
                            .with_main_axis_size(MainAxisSize::Max)
                            .with_child(
                                ConstrainedBox::new(
                                    Image::new(asset_source, CacheOption::BySize)
                                        .before_load(
                                            Icon::new(
                                                "ui/examples/image/loading.svg",
                                                ColorU::black(),
                                            )
                                            .finish(),
                                        )
                                        .finish(),
                                )
                                .with_height(500.)
                                .with_width(500.)
                                .finish(),
                            )
                            .finish(),
                    )
                    .finish(),
            )
            .finish()
    }
}

impl TypedActionView for RootView {
    type Action = ();
}
