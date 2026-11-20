// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::keymap::{BindingDescription, PerPlatformKeystroke};

use super::StaticCommand;

pub enum DefaultSlashCommandBinding {
    None,
    Single(&'static str),
    PerPlatform(PerPlatformKeystroke),
}

pub fn default_binding_for_command(name: &'static str) -> DefaultSlashCommandBinding {
    match name {
        "/agent" => DefaultSlashCommandBinding::PerPlatform(PerPlatformKeystroke {
            mac: "cmd-enter",
            linux_and_windows: "ctrl-shift-enter",
        }),
        "/conversations" => DefaultSlashCommandBinding::PerPlatform(PerPlatformKeystroke {
            mac: "cmd-y",
            linux_and_windows: "ctrl-shift-Y",
        }),
        "/open-repo" => DefaultSlashCommandBinding::PerPlatform(PerPlatformKeystroke {
            mac: "alt-cmd-o",
            linux_and_windows: "ctrl-alt-o",
        }),
        "/model" => DefaultSlashCommandBinding::Single("alt-shift-/"),
        _ => DefaultSlashCommandBinding::None,
    }
}

pub fn binding_description(command: &StaticCommand) -> BindingDescription {
    BindingDescription::new_preserve_case(crate::t!(
        "keybinding-desc-slash-command",
        name = command.name
    ))
}
