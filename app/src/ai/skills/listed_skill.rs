// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use ai::skills::{ParsedSkill, SkillProvider, SkillReference, SkillScope};
use serde::{Deserialize, Serialize};
use zap_ng_core::ui::icons::Icon;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct SkillDescriptor {
    pub reference: SkillReference,
    pub name: String,
    pub description: String,
    /// The scope of the skill.
    pub scope: SkillScope,
    /// The provider/origin of the skill (Claude, Codex, or ZapNG).
    /// None if the skill path didn't match a known provider directory.
    pub provider: SkillProvider,
    /// Override icon for this skill. When set, rendering code should use this
    /// instead of deriving the icon from the provider.
    #[serde(skip)]
    pub icon_override: Option<Icon>,
}

impl SkillDescriptor {
    /// Returns whether this skill is from a project directory (vs home directory).
    pub fn is_project_skill(&self) -> bool {
        self.scope == SkillScope::Project
    }

    pub fn new_bundled(id: String, skill: ParsedSkill, icon: Icon) -> Self {
        Self {
            provider: SkillProvider::ZapNG,
            scope: SkillScope::Bundled,
            reference: SkillReference::BundledSkillId(id),
            name: skill.name,
            description: skill.description,
            icon_override: Some(icon),
        }
    }
}

impl From<ParsedSkill> for SkillDescriptor {
    fn from(skill: ParsedSkill) -> Self {
        Self {
            provider: skill.provider,
            scope: skill.scope,
            reference: SkillReference::Path(skill.path),
            name: skill.name,
            description: skill.description,
            icon_override: None,
        }
    }
}
