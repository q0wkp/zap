// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: added tests for the askpass password helper, host-key handling, endpoint validation and zmx sessions, and dropped the Windows askpass tests; renamed the Warp and Zap names to ZapNG.

//! `ssh_command` 单元测试。
//!
//! 按 `AGENTS.md §5.6` 拆出到独立文件,由 `ssh_command.rs` 末尾的 `#[path]` 引入。
//! 覆盖范围:
//! - `build_ssh_args` / `build_ssh_command_line` 参数构建
//! - `test_connection` 在缺密码 / 错认证类型时的错误路径
//! - the askpass helper that hands the password or passphrase to ssh
//!
//! 注意:实际派生 ssh 子进程的端到端测试在 `app/src/ssh_manager/server_view.rs` 的
//! 集成测试 / 手测里覆盖 — 单测不做网络连接。
//!
//! author: logic
//! date: 2026-06-01

use super::*;
use zeroize::Zeroizing;

fn server() -> SshServerInfo {
    SshServerInfo {
        node_id: "n".into(),
        host: "1.2.3.4".into(),
        port: 22,
        username: "alice".into(),
        auth_type: AuthType::Password,
        key_path: None,
        credential_id: None,
        startup_command: None,
        zmx_session: None,
        notes: None,
        last_connected_at: None,
    }
}

#[test]
fn default_port_omitted() {
    let s = server();
    assert_eq!(build_ssh_args(&s), vec!["ssh", "alice@1.2.3.4"]);
    // shell-escape 出于保守会把 user@host 用单引号引起来,这是合法且
    // shell-equivalent 的形式 — 不强求未引用版本。
    let line = build_ssh_command_line(&s);
    assert!(
        line == "ssh alice@1.2.3.4" || line == "ssh 'alice@1.2.3.4'",
        "unexpected: {line}"
    );
}

#[test]
fn custom_port_uses_dash_p() {
    let mut s = server();
    s.port = 2222;
    assert_eq!(
        build_ssh_args(&s),
        vec!["ssh", "-p", "2222", "alice@1.2.3.4"]
    );
}

#[test]
fn key_auth_emits_dash_i() {
    let mut s = server();
    s.auth_type = AuthType::Key;
    s.key_path = Some("/home/u/.ssh/id_ed25519".into());
    assert_eq!(
        build_ssh_args(&s),
        vec!["ssh", "-i", "/home/u/.ssh/id_ed25519", "alice@1.2.3.4"]
    );
}

#[test]
fn key_auth_without_path_is_skipped() {
    let mut s = server();
    s.auth_type = AuthType::Key;
    s.key_path = None;
    assert_eq!(build_ssh_args(&s), vec!["ssh", "alice@1.2.3.4"]);
}

#[test]
fn empty_username_yields_host_only() {
    let mut s = server();
    s.username = String::new();
    assert_eq!(build_ssh_args(&s), vec!["ssh", "1.2.3.4"]);
}

#[test]
fn shell_escapes_spaces_in_path() {
    let mut s = server();
    s.auth_type = AuthType::Key;
    s.key_path = Some("/path with spaces/id_rsa".into());
    let line = build_ssh_command_line(&s);
    assert!(
        line.contains("'/path with spaces/id_rsa'"),
        "actual: {line}"
    );
}

#[test]
fn test_connection_requires_password_for_password_auth() {
    let s = server();
    // test_connection 应该在没有密码时返回 Offline + 错误信息
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(test_connection(&s, None, None));
    assert_eq!(result.status, ConnectionStatus::Offline);
    assert!(result
        .error_message
        .unwrap()
        .contains("Password not provided"));
}

#[test]
fn test_connection_requires_password_for_onekey_auth() {
    let mut s = server();
    s.auth_type = AuthType::OneKey;
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(test_connection(&s, None, None));
    assert_eq!(result.status, ConnectionStatus::Offline);
    assert!(result
        .error_message
        .unwrap()
        .contains("Password not provided"));
}

#[test]
fn onekey_key_auth_emits_dash_i_when_key_path_is_resolved() {
    let mut s = server();
    s.auth_type = AuthType::OneKey;
    s.key_path = Some("/home/u/.ssh/shared_ed25519".into());

    assert_eq!(
        build_ssh_args(&s),
        vec!["ssh", "-i", "/home/u/.ssh/shared_ed25519", "alice@1.2.3.4"]
    );
}

/// `BatchMode=yes` makes ssh skip the passphrase prompt, so a key with a passphrase could never
/// be tested. The prompts that would need a password typed are turned off one by one instead.
#[test]
fn key_auth_args_leave_the_passphrase_prompt_on() {
    let mut s = server();
    s.auth_type = AuthType::Key;
    s.key_path = Some("/home/user/.ssh/id_rsa".into());
    let args = build_key_auth_cmd_args(&s, "echo ok");
    assert!(args.contains(&"-i".to_string()));
    assert!(args.contains(&"/home/user/.ssh/id_rsa".to_string()));
    assert!(args.contains(&"BatchMode=no".to_string()), "{args:?}");
    assert!(
        args.contains(&"PasswordAuthentication=no".to_string()),
        "{args:?}"
    );
    assert!(
        args.contains(&"KbdInteractiveAuthentication=no".to_string()),
        "{args:?}"
    );
}

#[test]
fn connection_status_equality() {
    assert_eq!(ConnectionStatus::Online, ConnectionStatus::Online);
    assert_eq!(ConnectionStatus::Offline, ConnectionStatus::Offline);
    assert_eq!(ConnectionStatus::Unknown, ConnectionStatus::Unknown);
    assert_ne!(ConnectionStatus::Online, ConnectionStatus::Offline);
    assert_ne!(ConnectionStatus::Online, ConnectionStatus::Unknown);
    assert_ne!(ConnectionStatus::Offline, ConnectionStatus::Unknown);
}

// -------- the askpass helper --------

/// Runs the askpass helper the way ssh does, with the prompt as its only argument, and returns
/// what it printed, or `None` if it gave no answer.
fn askpass_answer(prompt: &str, secret: Option<(AskpassSecretKind, &str)>) -> Option<String> {
    let output = run_askpass(prompt, secret, None);
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).unwrap())
}

/// Runs the askpass helper with the environment `run_ssh` gives ssh.
fn run_askpass(
    prompt: &str,
    secret: Option<(AskpassSecretKind, &str)>,
    trusted_fingerprint: Option<&str>,
) -> std::process::Output {
    let helper = AskpassHelper::create().unwrap();
    let mut command = command::blocking::Command::new(helper.path());
    command
        .arg(prompt)
        .env_remove(ASKPASS_SECRET_ENV)
        .env_remove(ASKPASS_SECRET_KIND_ENV)
        .env_remove(ASKPASS_HOST_KEY_ENV);
    if let Some((kind, secret)) = secret {
        command
            .env(ASKPASS_SECRET_ENV, secret)
            .env(ASKPASS_SECRET_KIND_ENV, kind.as_env_value());
    }
    if let Some(fingerprint) = trusted_fingerprint {
        command.env(ASKPASS_HOST_KEY_ENV, fingerprint);
    }
    // Another test thread forking at the wrong moment briefly holds the script open for
    // writing, which makes running it fail as "text file busy".
    loop {
        match command.output() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10));
            }
            other => break other.unwrap(),
        }
    }
}

#[test]
fn askpass_helper_is_private_to_the_user() {
    let helper = AskpassHelper::create().unwrap();
    let file_mode = std::fs::metadata(helper.path())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(file_mode & 0o777, 0o700);
    let dir_mode = std::fs::metadata(helper.path().parent().unwrap())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(dir_mode & 0o777, 0o700);

    let path = helper.path().to_path_buf();
    drop(helper);
    assert!(!path.exists());
}

#[test]
fn askpass_answers_the_password_prompt_with_the_password() {
    assert_eq!(
        askpass_answer(
            "alice@1.2.3.4's password: ",
            Some((AskpassSecretKind::Password, "s3cret-pass"))
        )
        .as_deref(),
        Some("s3cret-pass\n")
    );
}

/// The secret is printed as it is: nothing in it is expanded or split by the shell.
#[test]
fn askpass_passes_the_secret_through_unchanged() {
    let secret = "pa ss  $HOME `id` \\ 'q' \"d\" * 密码🔐";
    assert_eq!(
        askpass_answer(
            "alice@1.2.3.4's password: ",
            Some((AskpassSecretKind::Password, secret))
        ),
        Some(format!("{secret}\n"))
    );
}

#[test]
fn askpass_answers_the_passphrase_prompt_with_the_passphrase() {
    assert_eq!(
        askpass_answer(
            "Enter passphrase for key '/home/u/.ssh/id_ed25519': ",
            Some((AskpassSecretKind::Passphrase, "key-pass"))
        )
        .as_deref(),
        Some("key-pass\n")
    );
}

/// A server that rejects the key and falls back to asking for a password must not be sent
/// the key's passphrase, nor a key's passphrase prompt the account's password.
#[test]
fn askpass_keeps_a_secret_for_the_prompt_it_is_meant_for() {
    assert_eq!(
        askpass_answer(
            "alice@1.2.3.4's password: ",
            Some((AskpassSecretKind::Passphrase, "key-pass"))
        ),
        None
    );
    assert_eq!(
        askpass_answer(
            "Enter passphrase for key '/home/u/.ssh/id_ed25519': ",
            Some((AskpassSecretKind::Password, "s3cret-pass"))
        ),
        None
    );
}

#[test]
fn askpass_gives_no_answer_without_a_secret_or_to_other_prompts() {
    assert_eq!(askpass_answer("alice@1.2.3.4's password: ", None), None);
    assert_eq!(
        askpass_answer(
            "Verification code: ",
            Some((AskpassSecretKind::Password, "s3cret-pass"))
        ),
        None
    );
}

/// 回归:`build_ssh_args` 不应再带 `sshpass`,防止有人误把它加回 cmd_args
/// (macOS 默认无 sshpass,残留路径会立即 No such file or directory)。
#[test]
fn build_ssh_args_does_not_emit_sshpass() {
    let s = server();
    let args = build_ssh_args(&s);
    assert!(
        !args.iter().any(|a| a == "sshpass"),
        "build_ssh_args must not emit sshpass; got {args:?}"
    );
}

// -------- password auth cmd_args 回归保护 --------
//
// 这些测试守住"测试连接"password 路径不再 10s timeout 的关键开关。
// 任何 `test_password_auth` 内部的 -o 选项调整都得满足这三条:
// 1. 不再声明 keyboard-interactive(否则 server 端 PAM 会 fallback 到 kbd-int)
// 2. 显式禁掉 KbdInteractiveAuthentication(客户端能力开关,不是偏好)
// 3. 末尾仍是 `echo ok` 远端命令(否则成功判定匹配不到 stdout)
// author: logic
// date: 2026-06-01

/// 回归保护:`PreferredAuthentications` 必须只含 `password`,不能含
/// `keyboard-interactive`。否则 stdin pipe + EOF 会触发 kbd-int PAM
/// 重试链(`pam_faildelay` ~2s/次),把 10s `TEST_TIMEOUT` 顶满。
#[test]
fn password_auth_args_no_keyboard_interactive() {
    let s = server();
    let args = build_password_auth_cmd_args(&s, "echo ok");
    let joined = args.join(" ");
    assert!(
        !joined.contains("keyboard-interactive"),
        "test_password_auth must NOT use keyboard-interactive; got {args:?}"
    );
    assert!(
        joined.contains("PreferredAuthentications=password"),
        "expected PreferredAuthentications=password; got {args:?}"
    );
    // 即便 PreferredAuthentications=password 出现,后面也不能再列其它方法。
    // split 取 "=" 后第一段,若以 "password," 开头说明后面还有别的认证。
    let after_pref = joined
        .split("PreferredAuthentications=")
        .nth(1)
        .unwrap_or("");
    assert!(
        !after_pref.starts_with("password,"),
        "PreferredAuthentications should not list other methods after password; got {args:?}"
    );
}

/// 回归保护:必须显式禁 kbd-interactive(客户端能力开关),
/// 不只靠 `PreferredAuthentications` 列表顺序(后者只约束 password
/// 子方法)。OpenSSH 8.2+ 行为差异、与 server 端 `AuthenticationMethods`
/// 交互时尤其需要这层 defense in depth。
#[test]
fn password_auth_args_disable_kbd_interactive() {
    let s = server();
    let args = build_password_auth_cmd_args(&s, "echo ok");
    let joined = args.join(" ");
    assert!(
        joined.contains("KbdInteractiveAuthentication=no"),
        "missing KbdInteractiveAuthentication=no; got {args:?}"
    );
}

/// 回归保护:cmd_args 末尾的 `echo ok` 必须作为 remote command 出现。
/// ssh 解析规则下 destination 之后第一个非选项位置参数 = remote command,
/// 如果选项顺序错了导致 ssh 不把 `echo ok` 识别为命令,成功判定会失效。
#[test]
fn password_auth_args_ends_with_echo_ok_command() {
    let s = server();
    let args = build_password_auth_cmd_args(&s, "echo ok");
    assert!(!args.is_empty(), "cmd_args is empty: {args:?}");
    let last = args.last().unwrap();
    assert_eq!(
        last, "echo ok",
        "cmd_args must end with `echo ok` as remote command; got {args:?}"
    );
}

/// 回归保护:password 路径的 destination (`user@host`) 必须出现在所有 `-o`
/// 选项**之后**、`echo ok` **之前**。SSH 命令行解析规则为
/// `ssh [options] destination [command]`,第一个非选项参数 = destination,
/// 其后一切 = 远程命令。若 `-o` 选项跑到 destination 之后,SSH 会把它
/// 当作远程命令的一部分而非自身选项,导致 `PreferredAuthentications`、
/// `KbdInteractiveAuthentication` 等关键选项全部失效,触发 kbd-interactive
/// PAM 重试链卡满 10s `TEST_TIMEOUT`。
/// author: logic
/// date: 2026-06-01
#[test]
fn password_auth_args_destination_before_echo_ok_and_after_options() {
    let s = server();
    let args = build_password_auth_cmd_args(&s, "echo ok");
    let joined = args.join(" ");

    // destination "alice@1.2.3.4" 必须出现在 "echo ok" 之前
    let dest_pos = joined
        .find("alice@1.2.3.4")
        .expect("destination must appear in args");
    let echo_pos = joined
        .find("echo ok")
        .expect("`echo ok` must appear in args");

    assert!(
        dest_pos < echo_pos,
        "destination must come before `echo ok`; got joined: {joined}"
    );

    // destination 必须出现在所有 -o 选项之后
    // 找最后一个 -o 选项的位置
    let last_o_pos = joined
        .rfind("-o ")
        .expect("expected at least one -o option");
    assert!(
        last_o_pos < dest_pos,
        "all -o options must come before destination; got joined: {joined}"
    );
}

/// 回归保护:key auth 路径的 `build_ssh_args` 也需 destination 在 -o 选项之后。
/// author: logic
/// date: 2026-06-01
#[test]
fn key_auth_args_destination_comes_after_options() {
    let mut s = server();
    s.auth_type = AuthType::Key;
    s.key_path = Some("/home/user/.ssh/id_rsa".into());

    let args = build_key_auth_cmd_args(&s, "echo ok");

    let joined = args.join(" ");
    let dest_pos = joined
        .find("alice@1.2.3.4")
        .expect("destination must appear in args");
    let echo_pos = joined
        .find("echo ok")
        .expect("`echo ok` must appear in args");
    let last_o_pos = joined
        .rfind("-o ")
        .expect("expected at least one -o option");

    assert!(
        last_o_pos < dest_pos,
        "all -o options must come before destination; got joined: {joined}"
    );
    assert!(
        dest_pos < echo_pos,
        "destination must come before `echo ok`; got joined: {joined}"
    );
}

#[test]
fn startup_command_line_attaches_after_the_startup_command() {
    let mut s = server();
    assert_eq!(build_startup_command_line(&s), None);

    s.startup_command = Some("cd /srv".into());
    assert_eq!(build_startup_command_line(&s).as_deref(), Some("cd /srv"));

    s.zmx_session = Some("dev".into());
    assert_eq!(
        build_startup_command_line(&s).as_deref(),
        Some("cd /srv; zmx attach dev")
    );

    s.startup_command = Some("   ".into());
    assert_eq!(
        build_startup_command_line(&s).as_deref(),
        Some("zmx attach dev")
    );
}

#[test]
fn startup_command_line_quotes_the_session_name() {
    let mut s = server();
    s.zmx_session = Some("my session; rm -rf ~".into());
    assert_eq!(
        build_startup_command_line(&s).as_deref(),
        Some("zmx attach 'my session; rm -rf ~'")
    );
}

#[test]
fn endpoint_accepts_host_names_and_addresses() {
    for host in ["example.com", "10.0.0.1", "::1", "fe80::1%eth0", "my-alias"] {
        assert_eq!(validate_ssh_endpoint(host, "alice", 22), Ok(()), "{host}");
    }
    assert_eq!(validate_ssh_endpoint("example.com", "", 65535), Ok(()));
}

#[test]
fn endpoint_rejects_a_host_ssh_would_read_as_an_option() {
    assert_eq!(
        validate_ssh_endpoint("-oProxyCommand=touch /tmp/x", "", 22),
        Err(SshEndpointError::InvalidHost)
    );
    assert_eq!(
        validate_ssh_endpoint("-V", "alice", 22),
        Err(SshEndpointError::InvalidHost)
    );
}

#[test]
fn endpoint_rejects_whitespace_and_control_characters() {
    for host in [
        "exam ple.com",
        "example.com\n",
        "example\t.com",
        "a\u{0}b",
        "a\u{7f}b",
    ] {
        assert_eq!(
            validate_ssh_endpoint(host, "alice", 22),
            Err(SshEndpointError::InvalidHost),
            "{host:?}"
        );
    }
    assert_eq!(
        validate_ssh_endpoint("", "alice", 22),
        Err(SshEndpointError::EmptyHost)
    );
}

#[test]
fn endpoint_rejects_a_user_ssh_would_read_as_an_option() {
    // The destination is `user@host`, so the user name is what the argument starts with.
    assert_eq!(
        validate_ssh_endpoint("example.com", "-oProxyCommand=id", 22),
        Err(SshEndpointError::InvalidUsername)
    );
    assert_eq!(
        validate_ssh_endpoint("example.com", "al ice", 22),
        Err(SshEndpointError::InvalidUsername)
    );
}

#[test]
fn endpoint_rejects_port_zero() {
    assert_eq!(
        validate_ssh_endpoint("example.com", "alice", 0),
        Err(SshEndpointError::InvalidPort)
    );
}

#[test]
fn test_connection_refuses_an_option_like_host_without_running_ssh() {
    let mut s = server();
    s.host = "-oProxyCommand=id".into();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(test_connection(&s, Some(Zeroizing::new("pw".into())), None));
    assert_eq!(result.status, ConnectionStatus::Offline);
    assert_eq!(
        result.error_message.as_deref(),
        Some("Host cannot start with '-' or contain spaces or control characters")
    );
}

// -------- host keys --------

/// The question ssh asks about a host key it has no record of (OpenSSH 9.6).
const HOST_KEY_QUESTION: &str = "The authenticity of host 'example.com (203.0.113.7)' can't be \
    established.\nED25519 key fingerprint is SHA256:jq1oZMhXkPYmtQ5uuWcWN8rAyW0mLAU8Yp0J0y6t9cE.\n\
    This key is not known by any other names.\n\
    Are you sure you want to continue connecting (yes/no/[fingerprint])? ";
const HOST_KEY_FINGERPRINT: &str = "SHA256:jq1oZMhXkPYmtQ5uuWcWN8rAyW0mLAU8Yp0J0y6t9cE";

#[test]
fn test_args_do_not_turn_host_key_checking_off() {
    let s = server();
    for args in [
        build_password_auth_cmd_args(&s, "echo ok"),
        build_key_auth_cmd_args(&s, "echo ok"),
    ] {
        assert!(
            !args.iter().any(|arg| arg.contains("StrictHostKeyChecking")),
            "{args:?}"
        );
        // In batch mode ssh refuses an unknown key without asking, and the key could never
        // be shown to the user.
        assert!(args.contains(&"BatchMode=no".to_string()), "{args:?}");
    }
}

/// Without a fingerprint the user confirmed, the helper refuses the key, hands ssh's question
/// back on standard error, and gives away no secret.
#[test]
fn askpass_refuses_an_unknown_host_key_and_reports_it() {
    let output = run_askpass(
        HOST_KEY_QUESTION,
        Some((AskpassSecretKind::Password, "s3cret-pass")),
        None,
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "no\n");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        parse_unknown_host_key(&stderr),
        Some(UnknownHostKey {
            key_type: "ED25519".into(),
            fingerprint: HOST_KEY_FINGERPRINT.into(),
        })
    );
    assert!(!stderr.contains("s3cret-pass"));
}

/// ssh compares a fingerprint given as the answer with the key the server offered, so this
/// accepts the confirmed key and no other.
#[test]
fn askpass_answers_the_host_key_question_with_the_confirmed_fingerprint() {
    let output = run_askpass(HOST_KEY_QUESTION, None, Some(HOST_KEY_FINGERPRINT));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{HOST_KEY_FINGERPRINT}\n")
    );
    assert_eq!(
        parse_unknown_host_key(&String::from_utf8_lossy(&output.stderr)),
        None
    );
}

/// When the fingerprint is not the key's, ssh asks again; repeating it would never end.
#[test]
fn askpass_gives_up_when_ssh_does_not_accept_the_fingerprint() {
    let output = run_askpass(
        "Please type 'yes', 'no' or the fingerprint: ",
        None,
        Some(HOST_KEY_FINGERPRINT),
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "no\n");
}

#[test]
fn unknown_host_key_is_read_only_from_the_reported_question() {
    // What a server prints, or ssh's own messages, is not taken for the question.
    assert_eq!(
        parse_unknown_host_key("ED25519 key fingerprint is SHA256:abc.\nPermission denied."),
        None
    );
    assert_eq!(parse_unknown_host_key(""), None);

    let stderr = format!(
        "{HOST_KEY_PROMPT_BEGIN}\n{HOST_KEY_QUESTION}\n{HOST_KEY_PROMPT_END}\n\
         Host key verification failed.\n"
    );
    assert_eq!(
        parse_unknown_host_key(&stderr).map(|key| key.fingerprint),
        Some(HOST_KEY_FINGERPRINT.to_string())
    );
}

#[test]
fn only_a_fingerprint_is_accepted_as_a_trusted_host_key() {
    assert!(is_host_key_fingerprint(HOST_KEY_FINGERPRINT));
    assert!(is_host_key_fingerprint(
        "MD5:16:27:ac:a5:76:28:2d:36:63:1b:56:4d:eb:df:a6:48"
    ));
    // `yes` would make ssh accept whatever key the server offers.
    for value in ["yes", "", "SHA256:", "SHA256:abc def", "SHA256:abc\nyes"] {
        assert!(!is_host_key_fingerprint(value), "{value:?}");
    }

    let s = server();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(run_remote_command(
        &s,
        Some(Zeroizing::new("pw".into())),
        Some("yes"),
        "echo ok",
        TEST_TIMEOUT,
    ));
    assert_eq!(
        result.unwrap_err(),
        SshRunError::Failed("Invalid host key fingerprint".into())
    );
}
