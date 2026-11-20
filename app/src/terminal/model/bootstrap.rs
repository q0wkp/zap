// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

/// Stages during the course of bootstrapping the shell.  
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BootstrapStage {
    /// ZapNG is re-parsing historical blocks for this session. We haven't yet started
    /// bootstrapping.
    RestoreBlocks,
    /// ZapNG is writing the bootstrap script into the running shell.
    ZapNGInput,
    /// Execution of any shell startup scripts such as .rc or .profile files.
    ScriptExecution,
    /// Model is fully bootstrapped (i.e the `Bootstrap` message was successfully received by ZapNG).   
    Bootstrapped,
    /// Model is fully bootstrapped and we've received the precmd that results from bootstrapping itself
    PostBootstrapPrecmd,
}

impl BootstrapStage {
    pub fn next_stage(&self) -> Self {
        match self {
            BootstrapStage::RestoreBlocks => BootstrapStage::ZapNGInput,
            BootstrapStage::ZapNGInput => BootstrapStage::ScriptExecution,
            BootstrapStage::ScriptExecution => {
                log::error!("calling next_stage on a block that should be bootstrapped");
                BootstrapStage::ScriptExecution
            }
            BootstrapStage::Bootstrapped => BootstrapStage::PostBootstrapPrecmd,
            BootstrapStage::PostBootstrapPrecmd => {
                log::error!(
                    "calling next_stage on an already bootstrapped block that has seen precmd"
                );
                BootstrapStage::PostBootstrapPrecmd
            }
        }
    }

    pub fn is_bootstrapped(&self) -> bool {
        matches!(
            self,
            BootstrapStage::Bootstrapped | BootstrapStage::PostBootstrapPrecmd
        )
    }

    pub fn is_done(&self) -> bool {
        matches!(self, BootstrapStage::PostBootstrapPrecmd)
    }

    /// ZapNGInput is the one block that is hidden by default (unless debug mode is on).
    pub fn is_hidden(&self) -> bool {
        matches!(self, BootstrapStage::ZapNGInput)
    }

    /// We only can have an empty block that's shown if it's a block a user created by hitting enter, or if it's
    /// a restored block that was created by the user hitting enter.
    pub fn is_empty_block_allowed(&self) -> bool {
        matches!(
            self,
            BootstrapStage::RestoreBlocks | BootstrapStage::PostBootstrapPrecmd
        )
    }
}
