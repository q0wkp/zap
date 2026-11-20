// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use std::path::PathBuf;
pub use virtual_fs::{Dirs, Stub, VirtualFS};

pub trait ZapNGDirs {
    #[allow(dead_code)]
    fn git_repository_fixture(&self) -> PathBuf {
        ZapNG::fixtures().join("git_repository")
    }
}

impl ZapNGDirs for Dirs {}

pub struct ZapNG;

impl ZapNG {
    #[allow(dead_code)]
    pub fn executable() -> PathBuf {
        let mut path = {
            let mut build = "debug";

            if !cfg!(debug_assertions) {
                build = "release";
            }

            std::env::var("CARGO_TARGET_DIR")
                .ok()
                .map(|directory| PathBuf::from(directory).join(build))
                .unwrap_or_else(|| Self::root().join(format!("target/{}", &build)))
        };

        path.push("warp");
        path
    }

    #[allow(dead_code)]
    pub fn fixtures() -> PathBuf {
        Self::root().join("tests/fixtures")
    }

    pub fn root() -> PathBuf {
        let manifest_dir = if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            PathBuf::from(manifest_dir)
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        };

        if manifest_dir.join("Cargo.lock").exists() {
            manifest_dir
        } else {
            manifest_dir
                .parent()
                .expect("Could not find the debug binaries directory")
                .parent()
                .expect("Could not find the debug binaries directory")
                .to_path_buf()
        }
    }
}
