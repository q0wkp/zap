// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: removed the unsupported-session state that only Windows sessions used; renamed the Warp and Zap names to ZapNG.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CodingPanelEnablementState {
    Enabled,
    /// An SSH command has been detected at preexec time but the remote
    /// session has not finished bootstrapping yet. The file tree should
    /// show a loading state immediately to avoid flickering the stale
    /// local tree.
    PendingRemoteSession,
    /// The active session is on a remote host.
    ///
    /// `has_remote_server` is `true` when the session is registered with
    /// `RemoteServerManager` (i.e. Auto SSH ZapNGification / mode 1). When
    /// `true`, remote repo metadata may arrive and the file tree should show
    /// a loading state. When `false` (tmux or subshell SSH), no data will
    /// arrive and the file tree should show a disabled message.
    RemoteSession {
        has_remote_server: bool,
    },
    Disabled,
}

impl CodingPanelEnablementState {
    pub(crate) fn from_session_env(
        is_enabled: bool,
        is_remote: bool,
        has_remote_server: bool,
    ) -> Self {
        if is_remote {
            Self::RemoteSession { has_remote_server }
        } else if is_enabled {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}
