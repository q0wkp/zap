use super::{attach_command, list_sessions_command, parse_session_names, switch_session_command};

#[test]
fn finds_the_program_given_to_attach() {
    assert_eq!(attach_command("zmx attach dev claude"), Some("claude"));
    assert_eq!(
        attach_command("zmx a dev claude --resume"),
        Some("claude --resume")
    );
    assert_eq!(
        attach_command("  /usr/bin/zmx   a  dev   nvim  "),
        Some("nvim")
    );
    assert_eq!(
        attach_command("zmx attach --min-window dev claude"),
        Some("claude")
    );
}

#[test]
fn ignores_attach_without_a_program() {
    assert_eq!(attach_command("zmx attach dev"), None);
    assert_eq!(attach_command("zmx a"), None);
    assert_eq!(attach_command("zmx"), None);
}

#[test]
fn ignores_other_commands() {
    assert_eq!(attach_command("zmx run dev claude"), None);
    assert_eq!(attach_command("zmxx attach dev claude"), None);
    assert_eq!(attach_command("echo zmx attach dev claude"), None);
    assert_eq!(attach_command(""), None);
}

#[test]
fn list_command_is_one_single_quoted_sh_script() {
    let command = list_sessions_command();
    let script = command
        .strip_prefix("sh -lc '")
        .and_then(|rest| rest.strip_suffix("' sh"))
        .expect("command must be `sh -lc '<script>' sh`");
    assert!(!script.contains('\''), "script: {script}");
    assert!(!script.contains('\n'), "script: {script}");
    assert!(script.ends_with("zmx list --short 2>/dev/null"));
}

#[test]
fn session_names_are_read_one_a_line() {
    assert_eq!(
        parse_session_names("dev\r\nmy build\n\n"),
        vec!["dev".to_string(), "my build".to_string()]
    );
    assert!(parse_session_names("").is_empty());
}

#[test]
fn switch_command_passes_the_names_as_quoted_arguments() {
    let command = switch_session_command("dev", "it's mine");
    // The names never enter the script: it reads them as $1 and $2.
    assert!(command.contains("ZMX_SESSION=\"$1\" ZMX_SESSION_PREFIX= zmx attach \"$2\""));
    assert!(command.ends_with("' sh dev 'it'\\''s mine'"), "{command}");
}

/// The switch command, run by a real `sh` against a fake `zmx` that records how it was called.
#[cfg(unix)]
#[test]
fn switch_command_runs_zmx_attach_inside_the_current_session() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("zmx");
    let record = dir.path().join("record");
    std::fs::write(
        &fake,
        format!(
            "#!/bin/sh\nprintf '%s|%s|%s|%s\\n' \"$ZMX_SESSION\" \"${{ZMX_SESSION_PREFIX-unset}}\" \"$1\" \"$2\" > '{}'\n",
            record.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

    // `-l` would read the user's own profile; the test only needs the script.
    let command = switch_session_command("dev", "my build").replacen("sh -lc", "sh -c", 1);
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .env("PATH", format!("{}:/usr/bin:/bin", dir.path().display()))
        .env("HOME", dir.path())
        .env("ZMX_SESSION_PREFIX", "pfx-")
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(
        std::fs::read_to_string(&record).unwrap(),
        "dev||attach|my build\n"
    );
}
