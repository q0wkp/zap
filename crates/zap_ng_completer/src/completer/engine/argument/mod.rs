// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

mod legacy;
pub use legacy::*;

use crate::{
    meta::{Span, SpannedItem},
    parsers::{
        hir::{Expression, ShellCommand},
        ParsedExpression, ParsedToken,
    },
};

/// Creates a new empty positional arg in a shell_command. This is useful before evaluating args
/// so that we don't include the extra whitespace at the end of the command (e.g. "cd ") within the
/// top level command.
fn add_extra_positional(shell_command: &mut ShellCommand, cursor: &Span) {
    let mut positional =
        vec![ParsedExpression::new(Expression::Literal, ParsedToken::empty()).spanned(*cursor)];

    match shell_command.args.positionals.take() {
        Some(mut positionals) => {
            positionals.append(&mut positional);
            shell_command.args.positionals = Some(positionals);
        }
        None => shell_command.args.positionals = Some(positional),
    };
}
