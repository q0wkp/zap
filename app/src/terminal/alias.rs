// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::terminal::model::session::Session;
use smol_str::SmolStr;
use std::sync::Arc;
use zap_ng_completer::parsers::simple::all_parsed_commands;

/// Contains information about an aliased command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AliasedCommand {
    /// The alias itself.
    pub alias: SmolStr,
    /// The value that the alias is mapped to.
    pub alias_value: String,
}

/// Returns whether the alias can be expanded by ZapNG given its value.
///
/// We don't expand on any alias that starts with itself, as it leads to
/// cases where the alias is expanded twice: once as the user types in the
/// editor and again by the shell when the command is entered.
// TODO: CORE-240 Don't expand if any command in the alias value is equal
// to the alias itself.
pub fn is_expandable_alias(alias: &str, alias_value: &str) -> bool {
    if let Some(command_token) = alias_value.split_whitespace().next() {
        return alias != command_token;
    }
    // If the alias value is empty, we don't expand.
    false
}

/// Searches the source text for any aliases in a command position. Returns
/// information about the alias if found.
fn check_for_alias(source: &str, session: Arc<Session>) -> Option<AliasedCommand> {
    let mut all_commands_iterator =
        all_parsed_commands(source, session.shell_family().escape_char());
    all_commands_iterator.find_map(|command| {
        let first_token = command.parts.first()?;
        let alias_value = session.alias_value(&first_token.item)?;
        is_expandable_alias(&first_token.item, alias_value).then(|| AliasedCommand {
            alias: first_token.item.as_str().into(),
            alias_value: alias_value.into(),
        })
    })
}

pub async fn check_for_alias_async(source: &str, session: Arc<Session>) -> Option<AliasedCommand> {
    check_for_alias(source, session)
}

#[cfg(test)]
#[path = "alias_tests.rs"]
pub mod tests;
