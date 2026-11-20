// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Because it's hard to identify the contents of rich content blocks,
/// we register unique identifiers to make it easier to identify them.
pub enum RichContentType {
    AIBlock,
    EnterAgentView,
    ZapNGifySuccessBlock,
    InlineAgentViewHeader,
    AgentViewZeroState,
    TerminalViewZeroState,
    PluginInstructionsBlock,
}

impl RichContentType {
    pub fn is_ai_block(&self) -> bool {
        matches!(self, Self::AIBlock)
    }

    pub fn is_agent_view_block(&self) -> bool {
        matches!(self, Self::EnterAgentView)
    }

    pub fn is_inline_agent_view_header(&self) -> bool {
        matches!(self, Self::InlineAgentViewHeader)
    }
}
