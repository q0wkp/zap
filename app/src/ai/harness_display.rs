// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

//! Shared display metadata for [`Harness`] variants.
//!
//! Any UI surface that shows a harness to the user — the harness selector
//! dropdown, the conversation details sidebar, etc. — should source its label,
//! icon, and brand color from here so the two surfaces cannot drift.

use pathfinder_color::ColorU;
use zap_ng_cli::agent::Harness;

use crate::ai::agent::conversation::AIAgentHarness;
use crate::ai::blocklist::CLAUDE_ORANGE;
use crate::terminal::cli_agent::GEMINI_BLUE;
use crate::ui_components::icons::Icon;

/// User-visible display name for a [`Harness`].
pub fn display_name(harness: Harness) -> &'static str {
    match harness {
        Harness::Oz => "ZapNG Agent",
        Harness::Claude => "Claude Code",
        Harness::OpenCode => "OpenCode",
        Harness::Gemini => "Gemini CLI",
        Harness::Unknown => "Unknown",
    }
}

/// Leading icon for a [`Harness`].
pub fn icon_for(harness: Harness) -> Icon {
    match harness {
        Harness::Oz => Icon::ZapNG,
        Harness::Claude => Icon::ClaudeLogo,
        Harness::OpenCode => Icon::OpenCodeLogo,
        Harness::Gemini => Icon::GeminiLogo,
        Harness::Unknown => Icon::HelpCircle,
    }
}

/// Brand tint for a [`Harness`]'s icon. `None` means "use the surface's
/// default foreground color".
pub fn brand_color(harness: Harness) -> Option<ColorU> {
    match harness {
        Harness::Oz => None,
        Harness::Claude => Some(CLAUDE_ORANGE),
        Harness::OpenCode => None,
        Harness::Gemini => Some(GEMINI_BLUE),
        Harness::Unknown => None,
    }
}

/// Map [`AIAgentHarness`] (from `ServerAIConversationMetadata`) to the
/// canonical [`Harness`].
impl From<AIAgentHarness> for Harness {
    fn from(harness: AIAgentHarness) -> Self {
        match harness {
            AIAgentHarness::Oz => Harness::Oz,
            AIAgentHarness::ClaudeCode => Harness::Claude,
            AIAgentHarness::Gemini => Harness::Gemini,
            AIAgentHarness::Unknown => Harness::Unknown,
        }
    }
}

impl PartialEq<Harness> for AIAgentHarness {
    fn eq(&self, other: &Harness) -> bool {
        Harness::from(*self) == *other
    }
}
