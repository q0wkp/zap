// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::search::result_renderer::ItemHighlightState;
use zap_ng_core::ui::appearance::Appearance;
use zap_ng_ui::{elements::Icon, Element};

/// Assumes the path is a file, not a folder
pub fn icon_from_file_path(
    path: &str,
    appearance: &Appearance,
    highlight_state: ItemHighlightState,
) -> Box<dyn Element> {
    let icon = crate::code::icon_from_file_path(path, appearance);
    match icon {
        Some(icon) => icon,
        None => Icon::new(
            "bundled/svg/completion-file.svg",
            highlight_state.icon_fill(appearance).into_solid(),
        )
        .finish(),
    }
}
