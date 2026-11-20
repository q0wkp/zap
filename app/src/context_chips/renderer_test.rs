// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use pathfinder_color::ColorU;
use zap_ng_ui::fonts::Properties;

use crate::context_chips::{ChipAvailability, ChipDisabledReason, ContextChipKind};

use super::{Renderer, RendererStyles};

#[test]
fn test_constructor_availability_updates_disabled_state_and_tooltip_override() {
    let kind = ContextChipKind::ShellGitBranch;
    let chip = kind.to_chip().expect("chip definition should exist");
    let renderer = Renderer::new(
        kind,
        chip,
        crate::context_chips::ChipValue::Text("main".to_string()),
        RendererStyles::new(
            ColorU {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            Properties::default(),
        ),
        ChipAvailability::Disabled(ChipDisabledReason::RequiresExecutable {
            command: "gh".to_string(),
        }),
    );

    assert!(renderer.is_disabled);
    assert_eq!(
        renderer.tooltip_override_text.as_deref(),
        Some("Requires the GitHub CLI")
    );
}

#[test]
fn test_constructor_availability_enabled_has_no_disabled_state_or_tooltip_override() {
    let kind = ContextChipKind::ShellGitBranch;
    let chip = kind.to_chip().expect("chip definition should exist");
    let renderer = Renderer::new(
        kind,
        chip,
        crate::context_chips::ChipValue::Text("main".to_string()),
        RendererStyles::new(
            ColorU {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            Properties::default(),
        ),
        ChipAvailability::Enabled,
    );

    assert!(!renderer.is_disabled);
    assert_eq!(renderer.tooltip_override_text, None);
}
