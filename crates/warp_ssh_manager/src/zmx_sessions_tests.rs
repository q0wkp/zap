use super::*;

const M: &str = "ZMXmarker";

fn list_output(body: &str) -> String {
    format!("{M}:begin\n{body}{M}:end\n")
}

#[test]
fn list_command_is_one_single_quoted_sh_script() {
    let command = build_list_command(M);
    let script = command
        .strip_prefix("sh -lc '")
        .and_then(|rest| rest.strip_suffix('\''))
        .expect("command must be `sh -lc '<script>'`");
    // 登录 shell 只看到一个单引号字符串,脚本内部不能再有单引号或换行。
    assert!(!script.contains('\''), "script: {script}");
    assert!(!script.contains('\n'), "script: {script}");
    assert!(script.contains(&format!("echo {M}:begin")));
    assert!(script.contains(&format!("echo {M}:end")));
    assert!(script.contains("zmx history \"$n\""));
}

#[test]
fn parses_sessions_with_their_previews() {
    let output = list_output(&format!(
        "{M}:session\n\
         \u{20}\u{20}name=dev\tpid=10\tclients=1\tcreated=1794486674\tcwd=file://host/home/me/my%20repo\n\
         $ cargo build\n\
         \u{20}\u{20}\u{20}Compiling zap\n\
         \n\
         {M}:session\n\
         → name=build\tpid=11\tclients=0\tcreated=1794486700\tcwd=file:///srv\tcmd=make -j8 V=1\tfg=cc1\n\
         \n"
    ));

    let sessions = parse_list_output(&output, M).unwrap();

    assert_eq!(
        sessions,
        vec![
            ZmxSession {
                name: "dev".into(),
                clients: Some(1),
                cwd: Some("/home/me/my repo".into()),
                command: None,
                foreground: None,
                preview: Some("$ cargo build\n   Compiling zap".into()),
            },
            ZmxSession {
                name: "build".into(),
                clients: Some(0),
                cwd: Some("/srv".into()),
                command: Some("make -j8 V=1".into()),
                foreground: Some("cc1".into()),
                preview: None,
            },
        ]
    );
}

#[test]
fn ignores_what_the_login_shell_prints_before_the_script() {
    let output = format!(
        "Welcome to host\n{M}:session\n{}",
        list_output(&format!(
            "{M}:session\nname=a\tpid=1\tclients=0\tcreated=0\n"
        ))
    );

    let sessions = parse_list_output(&output, M).unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "a");
}

#[test]
fn output_without_the_begin_marker_is_an_error() {
    assert_eq!(
        parse_list_output("name=a\tpid=1\n", M),
        Err(ZmxSessionsError::Failed("Unexpected output".into()))
    );
}

#[test]
fn missing_zmx_is_reported() {
    let output = format!("{M}:begin\n{M}:missing\n");
    assert_eq!(
        parse_list_output(&output, M),
        Err(ZmxSessionsError::NotInstalled)
    );
}

#[test]
fn no_sessions_is_an_empty_list() {
    assert_eq!(parse_list_output(&list_output(""), M), Ok(Vec::new()));
}

#[test]
fn unreachable_sessions_are_skipped() {
    let output = list_output(&format!(
        "{M}:session\n\
         name=stale\terr=ConnectionRefused\tstatus=cleaning up\n\
         {M}:session\n\
         name=live\tpid=2\tclients=0\tcreated=0\n\
         hello\n"
    ));

    let sessions = parse_list_output(&output, M).unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "live");
    assert_eq!(sessions[0].preview.as_deref(), Some("hello"));
}

#[test]
fn truncated_output_keeps_the_sessions_read_so_far() {
    let output = format!("{M}:begin\n{M}:session\nname=a\tpid=1\tclients=0\tcreated=0\nline\n");

    let sessions = parse_list_output(&output, M).unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].preview.as_deref(), Some("line"));
}

#[test]
fn crlf_output_is_parsed() {
    let output = list_output(&format!(
        "{M}:session\nname=a\tpid=1\tclients=2\tcreated=0\nhi\n"
    ))
    .replace('\n', "\r\n");

    let sessions = parse_list_output(&output, M).unwrap();

    assert_eq!(sessions[0].clients, Some(2));
    assert_eq!(sessions[0].preview.as_deref(), Some("hi"));
}

#[test]
fn session_line_keeps_equals_signs_in_the_command() {
    let session =
        parse_session_line("name=a\tpid=1\tclients=0\tcreated=0\tcmd=env A=b sh").unwrap();
    assert_eq!(session.command.as_deref(), Some("env A=b sh"));
}

#[test]
fn session_line_reads_the_legacy_start_dir() {
    let session =
        parse_session_line("name=a\tpid=1\tclients=0\tcreated=0\tstart_dir=/tmp/x").unwrap();
    assert_eq!(session.cwd.as_deref(), Some("/tmp/x"));
}

#[test]
fn session_line_ignores_labels_and_task_state() {
    let session = parse_session_line(
        "name=a\tpid=1\tclients=0\tcreated=5\tended=9\texit_code=1\tproject=zap",
    )
    .unwrap();
    assert_eq!(session.clients, Some(0));
    assert_eq!(session.cwd, None);
    assert_eq!(session.command, None);
    assert_eq!(session.foreground, None);
}

#[test]
fn session_line_reads_the_foreground_process_before_the_labels() {
    let session =
        parse_session_line("name=a\tpid=1\tclients=0\tcreated=0\tfg=vim\tfg=mine").unwrap();
    assert_eq!(session.foreground.as_deref(), Some("vim"));
}

#[test]
fn session_line_rejects_other_output() {
    assert_eq!(parse_session_line(""), None);
    assert_eq!(parse_session_line("no sessions found"), None);
    assert_eq!(parse_session_line("name=\tpid=1"), None);
    assert_eq!(parse_session_line("name=a"), None);
}

#[test]
fn cwd_keeps_non_ascii_and_malformed_escapes() {
    assert_eq!(
        decode_cwd("file://h/%E4%B8%AD%E6%96%87").as_deref(),
        Some("/中文")
    );
    assert_eq!(decode_cwd("file://h/a%zzb%4").as_deref(), Some("/a%zzb%4"));
    assert_eq!(decode_cwd("file://host"), None);
    assert_eq!(decode_cwd(""), None);
}

#[test]
fn preview_keeps_the_last_lines_and_drops_blank_edges() {
    let mut lines: Vec<String> = (0..PREVIEW_LINES + 5)
        .map(|i| format!("line {i}  "))
        .collect();
    lines.push(String::new());
    lines.push("   ".into());
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();

    let preview = trim_preview(&lines).unwrap();

    let kept: Vec<&str> = preview.lines().collect();
    assert_eq!(kept.len(), PREVIEW_LINES);
    assert_eq!(kept[0], "line 5");
    assert_eq!(
        kept[PREVIEW_LINES - 1],
        format!("line {}", PREVIEW_LINES + 4)
    );
}

#[test]
fn preview_of_blank_lines_is_none() {
    assert_eq!(trim_preview(&["", "  ", ""]), None);
    assert_eq!(trim_preview(&[]), None);
}

#[test]
fn preview_truncates_long_lines() {
    let long = "x".repeat(PREVIEW_LINE_CHARS + 50);
    let preview = trim_preview(&[long.as_str()]).unwrap();
    assert_eq!(preview.chars().count(), PREVIEW_LINE_CHARS);
}

#[test]
fn ssh_failure_message_falls_back_and_truncates() {
    assert_eq!(ssh_failure_message("  \n"), "Unexpected output");
    assert_eq!(
        ssh_failure_message("Permission denied (publickey).\n"),
        "Permission denied (publickey)."
    );
    let long = "e".repeat(300);
    assert_eq!(
        ssh_failure_message(&long),
        format!("{}...", "e".repeat(200))
    );
}

/// 脚本真的在 `sh` 下跑一遍:用一个假的 `zmx` 顶替,验证取名、预览和标记的衔接。
#[cfg(unix)]
#[test]
fn list_command_runs_under_sh_against_a_fake_zmx() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("zmx");
    // history 的输出故意不以换行结尾,和真的 zmx 一样。
    std::fs::write(
        &fake,
        "#!/bin/sh\n\
         case \"$1\" in\n\
         list) printf '  name=my dev\\tpid=1\\tclients=1\\tcreated=7\\tcwd=file://h/tmp\\n';\n\
               printf '  name=gone\\terr=ConnectionRefused\\tstatus=cleaning up\\n';\n\
               printf '→ name=b\\tpid=2\\tclients=0\\tcreated=8\\n';;\n\
         history) printf 'screen of %s\\nlast line' \"$2\";;\n\
         esac\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

    let command = build_list_command(M);
    let script = command
        .strip_prefix("sh -lc '")
        .and_then(|rest| rest.strip_suffix('\''))
        .unwrap();
    // 不走登录 shell(-l 会读用户自己的 profile),PATH 里只放假的 zmx 和系统目录。
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(script)
        .env("PATH", format!("{}:/usr/bin:/bin", dir.path().display()))
        .env("HOME", dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    let sessions = parse_list_output(&stdout, M).unwrap();

    assert_eq!(sessions.len(), 2, "stdout: {stdout}");
    assert_eq!(sessions[0].name, "my dev");
    assert_eq!(sessions[0].cwd.as_deref(), Some("/tmp"));
    assert_eq!(
        sessions[0].preview.as_deref(),
        Some("screen of my dev\nlast line")
    );
    assert_eq!(sessions[1].name, "b");
    assert_eq!(
        sessions[1].preview.as_deref(),
        Some("screen of b\nlast line")
    );
}

#[cfg(unix)]
#[test]
fn list_command_reports_a_missing_zmx() {
    let command = build_list_command(M);
    let script = command
        .strip_prefix("sh -lc '")
        .and_then(|rest| rest.strip_suffix('\''))
        .unwrap();
    // 把脚本开头补 PATH 的那一句换掉,保证哪里都找不到 zmx。
    let (_, rest) = script.split_once("; ").unwrap();
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("PATH=/nonexistent; {rest}"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(
        parse_list_output(&stdout, M),
        Err(ZmxSessionsError::NotInstalled),
        "stdout: {stdout}"
    );
}
