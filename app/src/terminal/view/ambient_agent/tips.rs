// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

//! Local fallback tips for the ambient-agent loading screen.

use crate::ai::agent_tips::AITip;
use zap_ng_ui::keymap::Keystroke;
use zap_ng_ui::AppContext;

/// A local fallback tip with text and optional link.
#[derive(Clone, Debug)]
pub struct AmbientAgentTip {
    text: String,
    link: Option<String>,
}

impl AmbientAgentTip {
    pub fn new(text: impl Into<String>, link: Option<impl Into<String>>) -> Self {
        Self {
            text: text.into(),
            link: link.map(|l| l.into()),
        }
    }
}

impl AITip for AmbientAgentTip {
    fn keystroke(&self, _app: &AppContext) -> Option<Keystroke> {
        None
    }

    fn link(&self) -> Option<String> {
        self.link.clone()
    }

    fn description(&self) -> &str {
        &self.text
    }

    // Uses the default implementation which adds "Tip: " prefix and parses backticks as inline code
}

/// Returns a collection of tips for the ambient-agent loading screen.
pub fn get_ambient_agent_tips() -> Vec<AmbientAgentTip> {
    Vec::new()
}
