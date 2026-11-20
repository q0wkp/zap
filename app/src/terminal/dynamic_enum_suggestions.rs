// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_completer::completer::{CommandOutput, GeneratorContext};

use crate::completer::SessionContext;

/// Given a command and a session context, run the command and parse it as a dynamic command.
/// Returns a vector of dynamic enum completion suggestions.
pub(crate) async fn run_dynamic_enum_command(
    command: &str,
    ctx: &SessionContext,
) -> anyhow::Result<Vec<String>> {
    ctx.execute_command_at_pwd(command, None)
        .await
        .and_then(|output| parse_dynamic_command(&output))
}

/// Parse the output of a dynamic enum suggestion command. Right now, this function
/// trims each row and splits on newlines.
fn parse_dynamic_command(output: &CommandOutput) -> anyhow::Result<Vec<String>> {
    match output.status {
        zap_ng_completer::completer::CommandExitStatus::Success => {
            let output = String::from_utf8_lossy(output.output());
            let cleaned_output: Vec<String> = output
                .split("\n")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(cleaned_output)
        }
        zap_ng_completer::completer::CommandExitStatus::Failure => {
            Err(anyhow::anyhow!("Command exited with failure code"))
        }
    }
}
