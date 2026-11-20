// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use anyhow::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <pattern> [paths...]", args[0]);
        std::process::exit(1);
    }

    let pattern = args[1].clone();
    let paths: Vec<PathBuf> = args[2..].iter().map(PathBuf::from).collect();

    println!("Searching for pattern: {pattern}");
    println!("In paths: {paths:?}");

    zap_ng_ripgrep::search::run_search_subprocess(&[pattern], paths, false, false, None)?;

    Ok(())
}
