// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use std::path::PathBuf;

/// Shared CLI args for loading command configuration from a file.
#[derive(Debug, Default, Clone, clap::Args)]
pub struct ConfigFileArgs {
    /// Path to a YAML or JSON configuration file.
    #[arg(
        short = 'f',
        long = "file",
        value_name = "PATH",
        env = "WARP_AGENT_CONFIG_FILE"
    )]
    pub file: Option<PathBuf>,
}
