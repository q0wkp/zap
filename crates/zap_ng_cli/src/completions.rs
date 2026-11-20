// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use std::io;

use clap_complete::aot::{Shell, generate};

use crate::{Args, binary_name};
use zap_ng_core::channel::ChannelState;

/// Generate shell completions for the ZapNG CLI and write them to stdout.
pub fn generate_to_stdout(shell: Option<Shell>) -> anyhow::Result<()> {
    let shell = match shell.or_else(Shell::from_env) {
        Some(s) => s,
        None => anyhow::bail!(
            "Could not determine shell from environment. Please provide a shell argument."
        ),
    };

    let mut cmd = Args::clap_command();
    let bin_name =
        binary_name().unwrap_or_else(|| ChannelState::channel().cli_command_name().to_string());

    generate(shell, &mut cmd, bin_name, &mut io::stdout());
    Ok(())
}
