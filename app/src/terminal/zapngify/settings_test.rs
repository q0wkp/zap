// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: removed the Windows-only WSL tests and added a test that zmx attach commands are not treated as subshell commands; renamed the Warp and Zap names to ZapNG.

// ZapNGifying writes the bootstrap script to the PTY. A re-attached zmx session may be running any
// program, which would receive that script as input.
#[test]
fn test_zmx_attach_is_not_a_subshell_command() {
    use crate::test_util::settings::initialize_settings_for_tests;
    use zap_ng_util::path::ShellFamily;
    use zap_ng_ui::{App, SingletonEntity};

    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);

        app.read(|ctx| {
            let settings = super::ZapNGifySettings::as_ref(ctx);
            for command in ["zmx attach dev", "zmx a dev", "zmx attach dev bash"] {
                assert!(!settings.is_compatible_subshell_command(command, ShellFamily::Posix));
            }
        });
    });
}
