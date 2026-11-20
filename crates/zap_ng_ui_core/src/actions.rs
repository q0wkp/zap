// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

/// A StandardAction is one that corresponds to an action that
/// must be dispatched and handled natively by NSApp (e.g. terminate:)
/// Use CustomActions for handling ZapNG specific actions.
///
/// Set a 'repr' here as we store these values as tags in menu items.
#[derive(Copy, Clone, Debug, PartialEq, Eq, FromPrimitive, ToPrimitive, Hash)]
#[repr(isize)]
pub enum StandardAction {
    Close,
    Hide,
    HideOtherApps,
    ShowAllApps,
    Quit,
    Zoom,
    Minimize,
    BringAllToFront,
    ToggleFullScreen,
    Paste,
}
