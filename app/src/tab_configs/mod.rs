// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

pub(crate) mod action_sidecar;
pub mod branch_picker;
pub mod new_worktree_modal;
pub mod params_modal;
pub(crate) mod remove_confirmation_dialog;
pub mod repo_picker;
pub mod session_config;
pub mod session_config_modal;
pub mod session_config_rendering;
pub mod tab_config;
pub mod telemetry;

use zap_ng_core::ui::theme::Fill;

pub use new_worktree_modal::{NewWorktreeModal, NewWorktreeModalEvent};
pub use params_modal::{TabConfigParamsModal, TabConfigParamsModalEvent};
#[cfg(feature = "local_fs")]
pub(crate) use tab_config::build_worktree_config_toml;
pub use tab_config::{
    render_tab_config, TabConfig, TabConfigError, TabConfigParam, TabConfigParamType,
};

/// Optional visual overrides for BranchPicker / RepoPicker dropdowns.
pub struct PickerStyle {
    pub width: f32,
    pub background: Option<Fill>,
}
