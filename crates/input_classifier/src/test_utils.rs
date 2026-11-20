// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use std::{collections::HashSet, sync::Arc};

use smol_str::SmolStr;
use zap_ng_completer::{
    completer::{GeneratorContext, PathCompletionContext},
    signatures::CommandRegistry,
};

/// An implementation of `CompletionContext` for testing purposes.
pub struct CompletionContext {
    command_registry: Arc<CommandRegistry>,
}

impl CompletionContext {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            command_registry: CommandRegistry::global_instance(),
        }
    }
}

impl zap_ng_completer::completer::CompletionContext for CompletionContext {
    fn top_level_commands(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.command_registry.registered_commands())
    }

    fn command_registry(&self) -> &CommandRegistry {
        &self.command_registry
    }

    fn environment_variable_names(&self) -> Option<&HashSet<SmolStr>> {
        None
    }

    fn shell_supports_autocd(&self) -> Option<bool> {
        None
    }

    fn path_completion_context(&self) -> Option<&dyn PathCompletionContext> {
        None
    }

    fn generator_context(&self) -> Option<&dyn GeneratorContext> {
        None
    }
}
