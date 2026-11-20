// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

//! Module containing helper functions for opening links within the terminal.

use zap_ng_ui::event::ModifiersState;
use zap_ng_ui::platform::OperatingSystem;

/// Returns a string denoting the keybinding to directly open a link.
pub fn directly_open_link_keybinding_string() -> &'static str {
    match OperatingSystem::get() {
        OperatingSystem::Mac => "Cmd +",
        OperatingSystem::Linux | OperatingSystem::Windows => "Ctrl +",
        OperatingSystem::Other(_) => "Middle",
    }
}

/// Returns true if a link should directly be opened (instead of showing a tooltip) given the
/// current [`ModifiersState`].
///
/// NOTE this is platform dependent: On MacOS links can be directly opened via `cmd+click`, on
/// Linux/Windows they are opened via `ctrl+click`.
pub fn should_directly_open_link(modifiers: &ModifiersState) -> bool {
    match OperatingSystem::get() {
        OperatingSystem::Mac => modifiers.cmd,
        OperatingSystem::Linux | OperatingSystem::Windows => modifiers.ctrl,
        // On platforms other than MacOS, Linux, and Windows, accept both cmd and ctrl.
        OperatingSystem::Other(_) => modifiers.cmd || modifiers.ctrl,
    }
}
