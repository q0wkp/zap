// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_core::ui::appearance::Appearance;
use zap_ng_ui::{
    fonts::Weight,
    ui_components::components::{Coords, UiComponentStyles},
};

pub const ICON_MARGIN: f32 = 8.;
pub const PAGE_SPACING: f32 = 16.;
pub const PAGE_PADDING: f32 = 28.;
pub const ITEM_BOTTOM_MARGIN: f32 = 12.;
pub const EDITOR_VERTICAL_PADDING: f32 = 10.;
pub const INSTALLATION_MODAL_PADDING: f32 = 16.;
pub const INSTALLATION_MODAL_BUTTON_GAP: f32 = 12.;
pub const INSTALLATION_MODAL_BUTTON_TOP_MARGIN: f32 = 16.;
pub const INSTALLATION_MODAL_INPUT_VERTICAL_SPACING: f32 = 12.;
pub const INSTALLATION_MODAL_BUTTON_PADDING: Coords = Coords {
    left: 8.,
    right: 8.,
    top: 6.,
    bottom: 6.,
};
pub const INSTALLATION_MODAL_LABEL_VERTICAL_SPACING: f32 = 4.;
pub const INSTALLATION_MODAL_TITLE_VERTICAL_SPACING: f32 = 16.;
pub const SECTION_MARGIN: f32 = 16.;
pub const EMPTY_STATE_HEIGHT: f32 = 400.;
pub const CORNER_RADIUS: f32 = 4.;
pub const SERVER_CARD_LIST_SPACING: f32 = 8.;
pub const SERVER_CARD_INTERIOR_SPACING: f32 = 4.;
pub const SERVER_CARD_ACTIONS_STANDARD_WIDTH: f32 = 180.;
pub const SERVER_CARD_ACTIONS_WIDE_WIDTH: f32 = 240.;
pub const EDIT_PAGE_BUTTON_SPACING: f32 = 4.;
pub const UPDATE_AVAILABLE_DOT_WIDTH: f32 = 6.;

pub fn header_text(appearance: &Appearance) -> UiComponentStyles {
    UiComponentStyles {
        font_size: Some(appearance.ui_font_heading_2()),
        font_weight: Some(Weight::Bold),
        ..Default::default()
    }
}

pub fn description_text(appearance: &Appearance) -> UiComponentStyles {
    UiComponentStyles {
        font_size: Some(appearance.ui_font_subheading()),
        font_color: Some(
            appearance
                .theme()
                .sub_text_color(appearance.theme().background())
                .into(),
        ),
        margin: Some(Coords {
            bottom: 8.,
            ..Default::default()
        }),
        ..Default::default()
    }
}
