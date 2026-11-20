// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::{completer::TopLevelCommandCaseSensitivity, signatures::CommandRegistry};

/// Returns the name of the argument that should be given at `idx` for the given command.
pub(super) fn argument_name_at_index_for_command(
    command: &str,
    idx: usize,
    command_registry: &CommandRegistry,
    command_case_sensitivity: TopLevelCommandCaseSensitivity,
) -> Option<String> {
    command_registry
        .signature_from_line(command, command_case_sensitivity)
        .and_then(|found_signature| {
            let arguments = found_signature.signature.arguments();
            arguments
                .get(idx)
                .map(|arg| arg.name().unwrap_or_default().to_string())
        })
}
