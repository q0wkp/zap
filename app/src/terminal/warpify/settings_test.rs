// Warpifying writes the bootstrap script to the PTY. A re-attached zmx session may be running any
// program, which would receive that script as input.
#[test]
fn test_zmx_attach_is_not_a_subshell_command() {
    use crate::test_util::settings::initialize_settings_for_tests;
    use warp_util::path::ShellFamily;
    use warpui::{App, SingletonEntity};

    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);

        app.read(|ctx| {
            let settings = super::WarpifySettings::as_ref(ctx);
            for command in ["zmx attach dev", "zmx a dev", "zmx attach dev bash"] {
                assert!(!settings.is_compatible_subshell_command(command, ShellFamily::Posix));
            }
        });
    });
}
