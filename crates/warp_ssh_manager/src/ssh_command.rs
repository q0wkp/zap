//! 把 `SshServerInfo` 拼成 `ssh ...` 命令,并派生测试连接的子进程。
//!
//! 写入 PTY 时调 `build_ssh_command_line`,会用 shell-escape 引用每个 arg,
//! 防止用户名 / host / key_path 里的空格或单引号破坏命令行。
//!
//! ## How secrets reach ssh
//!
//! OpenSSH never reads a password or a key's passphrase from standard input: it asks on the
//! terminal, or, without one, runs the program named by `SSH_ASKPASS` and reads the answer
//! from that program's output. The ssh processes spawned here are therefore given a small
//! askpass helper (`askpass.sh`, written to a private directory for the lifetime of the
//! process) and `SSH_ASKPASS_REQUIRE=force`, and the secret travels in an environment variable
//! of that one ssh process, which only the same user can read. It is never part of an argument
//! list (visible to every user through `ps` and `/proc/<pid>/cmdline`) nor written to a file.

use crate::types::{AuthType, ConnectionStatus, SshServerInfo};
use std::borrow::Cow;
use std::io::Write as _;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use thiserror::Error;
use zeroize::Zeroizing;

/// Why a server's host, user name or port can't be handed to `ssh`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum SshEndpointError {
    #[error("Host cannot be empty")]
    EmptyHost,
    #[error("Host cannot start with '-' or contain spaces or control characters")]
    InvalidHost,
    #[error("User cannot start with '-' or contain spaces or control characters")]
    InvalidUsername,
    #[error("Port must be a number between 1 and 65535")]
    InvalidPort,
}

/// Checks that `ssh` will read the server's destination as a destination.
///
/// `build_ssh_args` passes `user@host` (or the bare host) as one argument, so a value that
/// starts with `-` would be parsed as an option instead (`-oProxyCommand=...` runs a local
/// command). Whitespace and control characters never belong in a host or user name either.
/// The values may come from another device through sync or from `~/.ssh/config`, so they are
/// checked again wherever a command is built from them, not only when the user types them.
pub fn validate_ssh_endpoint(
    host: &str,
    username: &str,
    port: u16,
) -> Result<(), SshEndpointError> {
    let is_unsafe = |value: &str| {
        value.starts_with('-') || value.chars().any(|c| c.is_whitespace() || c.is_control())
    };
    if host.is_empty() {
        return Err(SshEndpointError::EmptyHost);
    }
    if is_unsafe(host) {
        return Err(SshEndpointError::InvalidHost);
    }
    if is_unsafe(username) {
        return Err(SshEndpointError::InvalidUsername);
    }
    if port == 0 {
        return Err(SshEndpointError::InvalidPort);
    }
    Ok(())
}

pub fn build_ssh_args(server: &SshServerInfo) -> Vec<String> {
    let mut args: Vec<String> = vec!["ssh".into()];
    if server.port != 22 {
        args.push("-p".into());
        args.push(server.port.to_string());
    }
    if matches!(server.auth_type, AuthType::Key | AuthType::OneKey) {
        if let Some(path) = server.key_path.as_deref() {
            if !path.is_empty() {
                args.push("-i".into());
                args.push(path.to_string());
            }
        }
    }
    let target = if server.username.is_empty() {
        server.host.clone()
    } else {
        format!("{}@{}", server.username, server.host)
    };
    args.push(target);
    args
}

/// 连接成功后写入 PTY 的命令:先跑启动命令,再 attach 到 zmx session。
pub fn build_startup_command_line(server: &SshServerInfo) -> Option<String> {
    let non_empty = |value: &Option<String>| {
        value
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let attach = non_empty(&server.zmx_session).map(|name| {
        format!(
            "zmx attach {}",
            shell_escape::unix::escape(Cow::Owned(name))
        )
    });
    match (non_empty(&server.startup_command), attach) {
        (Some(startup), Some(attach)) => Some(format!("{startup}; {attach}")),
        (Some(command), None) | (None, Some(command)) => Some(command),
        (None, None) => None,
    }
}

pub fn build_ssh_command_line(server: &SshServerInfo) -> String {
    let args = build_ssh_args(server);
    args.iter()
        .map(|a| shell_escape::unix::escape(Cow::Borrowed(a.as_str())).to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

const TEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct ConnectionTestResult {
    pub status: ConnectionStatus,
    pub latency_ms: Option<u64>,
    pub error_message: Option<String>,
}

pub async fn test_connection(
    server: &SshServerInfo,
    password: Option<Zeroizing<String>>,
) -> ConnectionTestResult {
    let start = instant::Instant::now();

    if let Err(e) = validate_ssh_endpoint(&server.host, &server.username, server.port) {
        return ConnectionTestResult {
            status: ConnectionStatus::Offline,
            latency_ms: None,
            error_message: Some(e.to_string()),
        };
    }

    let result = match run_remote_command(server, password, "echo ok", TEST_TIMEOUT).await {
        Ok(output) => finalize_test_result(&output, server.auth_type),
        Err(e) => Err(e),
    };

    let latency = start.elapsed().as_millis() as u64;

    match result {
        Ok(()) => ConnectionTestResult {
            status: ConnectionStatus::Online,
            latency_ms: Some(latency),
            error_message: None,
        },
        Err(e) => ConnectionTestResult {
            status: ConnectionStatus::Offline,
            latency_ms: Some(latency),
            error_message: Some(e),
        },
    }
}

/// 解析 ssh 子进程的输出,统一成功/失败判定逻辑。
fn finalize_test_result(output: &std::process::Output, auth_type: AuthType) -> Result<(), String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr_trimmed = String::from_utf8_lossy(&output.stderr).trim().to_string();

    // 始终把 ssh 真实 stderr 落日志,即便成功也留痕,便于事后排查
    // "为什么 server 接受了 password 但 UI 报成功"的差异。
    if !stderr_trimmed.is_empty() {
        log::warn!("ssh test stderr: {stderr_trimmed}");
    }

    // 成功判定:严格匹配 `echo ok` 的输出。原先 `ends_with("ok")` 的兜底
    // 会让 banner / motd 末尾碰巧以 "ok" 结尾时误判为成功,这里去掉。
    if output.status.success() && stdout.trim() == "ok" {
        Ok(())
    } else if stderr_trimmed.contains("Permission denied")
        || stderr_trimmed.contains("Authentication failed")
    {
        // 错误信息带上精简 stderr(<= 200 字符),便于用户判断 server 端
        // 是没启 password、还是配置了 kbd-only AuthenticationMethods 等。
        let detail = if stderr_trimmed.is_empty() {
            String::new()
        } else {
            let snippet: String = stderr_trimmed.chars().take(200).collect();
            if stderr_trimmed.chars().count() > 200 {
                format!(" ({snippet}...)")
            } else {
                format!(" ({snippet})")
            }
        };
        match auth_type {
            AuthType::Key => Err(format!(
                "Authentication failed: the key or its passphrase was not accepted{detail}"
            )),
            AuthType::Password | AuthType::OneKey => {
                Err(format!("Authentication failed: wrong password{detail}"))
            }
        }
    } else {
        Err(format!(
            "Unexpected output: stdout={} stderr={}",
            stdout.trim(),
            stderr_trimmed
        ))
    }
}

/// What the secret handed to ssh is for, which decides the prompt it answers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AskpassSecretKind {
    Password,
    Passphrase,
}

impl AskpassSecretKind {
    /// The value of `ASKPASS_SECRET_KIND_ENV` the helper compares against.
    fn as_env_value(self) -> &'static str {
        match self {
            AskpassSecretKind::Password => "password",
            AskpassSecretKind::Passphrase => "passphrase",
        }
    }
}

/// The environment variables the askpass helper reads.
const ASKPASS_SECRET_ENV: &str = "ZAP_SSH_ASKPASS_SECRET";
const ASKPASS_SECRET_KIND_ENV: &str = "ZAP_SSH_ASKPASS_SECRET_KIND";

const ASKPASS_SCRIPT: &str = include_str!("askpass.sh");

/// The askpass helper on disk, in a directory of its own that only the user can enter. The
/// script holds no secret. Dropping this removes it.
struct AskpassHelper {
    path: PathBuf,
    _dir: tempfile::TempDir,
}

impl AskpassHelper {
    fn create() -> std::io::Result<Self> {
        // The per-user runtime directory where there is one (Linux), else the temporary
        // directory, which on macOS is per-user as well.
        let base = dirs::runtime_dir()
            .filter(|dir| dir.is_dir())
            .unwrap_or_else(std::env::temp_dir);
        let dir = tempfile::Builder::new()
            .prefix("zap-ssh-askpass-")
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir_in(base)?;
        let path = dir.path().join("askpass");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o700)
            .open(&path)?;
        file.write_all(ASKPASS_SCRIPT.as_bytes())?;
        Ok(Self { path, _dir: dir })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

/// Runs `ssh` with `cmd_args` and returns its output. `secret` is what the askpass helper
/// answers the matching prompt with; without one, a prompt gets no answer and ssh gives up.
async fn run_ssh(
    cmd_args: &[String],
    secret: Option<(AskpassSecretKind, &Zeroizing<String>)>,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let askpass = AskpassHelper::create()
        .map_err(|e| format!("Failed to prepare the askpass helper: {e}"))?;

    // A session of its own leaves ssh without a controlling terminal, so it can never prompt
    // on the terminal Zap was started from.
    let mut command = command::r#async::Command::new_with_session("ssh");
    command
        .args(cmd_args)
        .env("SSH_ASKPASS", askpass.path())
        .env("SSH_ASKPASS_REQUIRE", "force")
        .env_remove(ASKPASS_SECRET_ENV)
        .env_remove(ASKPASS_SECRET_KIND_ENV)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some((kind, secret)) = secret {
        command
            .env(ASKPASS_SECRET_ENV, secret.as_str())
            .env(ASKPASS_SECRET_KIND_ENV, kind.as_env_value());
    }
    let child = command
        .spawn()
        .map_err(|e| format!("Failed to start ssh: {e}"))?;

    let result = match tokio::time::timeout(timeout, child.output()).await {
        Ok(Ok(out)) => Ok(out),
        Ok(Err(e)) => Err(format!("Failed to read ssh output: {e}")),
        Err(_) => Err("Connection timeout".into()),
    };
    // ssh has exited (or was killed with the timed-out future), so the helper can go.
    drop(askpass);
    result
}

/// 拼出 password 认证测试时给 ssh 子进程的完整 argv。
///
/// 与 `build_ssh_args` 不同:这里跳过首项 `"ssh"`(我们用
/// `Command::new("ssh")` 显式派生),追加测试用 `-o` 选项和远端命令。
///
/// 关键选项含义:
/// - `BatchMode=no`: lets ssh ask for the password, which the askpass helper answers
/// - `PreferredAuthentications=password`:**只**声明想试 password,不带
///   `keyboard-interactive`。否则 server 端 PAM 在 password 之后会触发
///   kbd-interactive fallback,kbd-int 子 prompt 拿不到响应,会逐项重试
///   并触发 `pam_faildelay`(~2s/次),累计 ~8-10s 顶满 `TEST_TIMEOUT`。
/// - `KbdInteractiveAuthentication=no`:客户端能力开关,直接禁掉整个 kbd-int
///   协议。光靠 `PreferredAuthentications` 不够——它只约束 password 子方法的
///   prompt 次数,kbd-int 仍可走;两个开关都设才是 defense in depth。
/// - `NumberOfPasswordPrompts=1`:password 子方法只允许 1 次重试。
/// - `ConnectTimeout=5`:单次 TCP 连接超时。
/// - `StrictHostKeyChecking=no`:不拦 known_hosts(测试场景下避免 host key
///   变化导致误报,真实终端连接走别的路径)。
/// - `LogLevel=ERROR`:抑制 host key 提示 / banner 等噪音。
///
/// 连接测试以 `echo ok` 作为远端命令,严格匹配 stdout 判定成功(避免 banner /
/// motd 末尾恰好含 "ok" 的误判)。
///
/// author: logic
/// date: 2026-06-01
fn build_password_auth_cmd_args(server: &SshServerInfo, remote_command: &str) -> Vec<String> {
    // skip(1) 去掉 "ssh" 本身(Command::new 已指定),剩下
    // ["-p","2222","user@host"]。-o 选项必须插在 destination 之前,
    // 否则 SSH 把 -o 当作远程命令的一部分而非自身选项。
    let mut args: Vec<String> = build_ssh_args(server).into_iter().skip(1).collect();
    let target = args.pop().unwrap();
    args.extend([
        "-o".into(),
        "BatchMode=no".into(),
        "-o".into(),
        "PreferredAuthentications=password".into(),
        "-o".into(),
        "KbdInteractiveAuthentication=no".into(),
        "-o".into(),
        "NumberOfPasswordPrompts=1".into(),
        "-o".into(),
        "ConnectTimeout=5".into(),
        "-o".into(),
        "StrictHostKeyChecking=no".into(),
        "-o".into(),
        "LogLevel=ERROR".into(),
    ]);
    args.push(target);
    args.push(remote_command.into());
    args
}

/// 拼出密钥认证时给 ssh 子进程的 argv(不含首项 `"ssh"`),末尾是远端命令。
fn build_key_auth_cmd_args(server: &SshServerInfo, remote_command: &str) -> Vec<String> {
    // build_ssh_args 末尾是 destination (user@host),-o 选项必须插在
    // destination 之前,否则 SSH 把 -o 当作远程命令的一部分而非自身选项。
    let mut args: Vec<String> = build_ssh_args(server).into_iter().skip(1).collect();
    let target = args.pop().unwrap();
    // `BatchMode=yes` would stop ssh asking for the key's passphrase, so the two ways of
    // signing in that need a password typed are turned off one by one instead. Without a
    // passphrase to give, the askpass helper leaves the prompt unanswered and ssh moves on.
    args.extend([
        "-o".into(),
        "BatchMode=no".into(),
        "-o".into(),
        "PasswordAuthentication=no".into(),
        "-o".into(),
        "KbdInteractiveAuthentication=no".into(),
        "-o".into(),
        "NumberOfPasswordPrompts=1".into(),
        "-o".into(),
        "ConnectTimeout=5".into(),
        "-o".into(),
        "StrictHostKeyChecking=no".into(),
        "-o".into(),
        "LogLevel=ERROR".into(),
    ]);
    args.push(target);
    args.push(remote_command.into());
    args
}

/// Runs a remote command on the server and returns the output of the ssh process. With key
/// authentication `password` is the key's passphrase, if it has one; otherwise it is the
/// account's password.
pub(crate) async fn run_remote_command(
    server: &SshServerInfo,
    password: Option<Zeroizing<String>>,
    remote_command: &str,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    validate_ssh_endpoint(&server.host, &server.username, server.port)
        .map_err(|e| e.to_string())?;
    match server.auth_type {
        AuthType::Key => {
            let cmd_args = build_key_auth_cmd_args(server, remote_command);
            // A key without a passphrase, or one the agent already holds, needs no secret.
            let passphrase = password
                .as_ref()
                .filter(|passphrase| !passphrase.is_empty())
                .map(|passphrase| (AskpassSecretKind::Passphrase, passphrase));
            run_ssh(&cmd_args, passphrase, timeout).await
        }
        AuthType::Password | AuthType::OneKey => {
            let password = password.ok_or("Password not provided")?;
            let cmd_args = build_password_auth_cmd_args(server, remote_command);
            run_ssh(
                &cmd_args,
                Some((AskpassSecretKind::Password, &password)),
                timeout,
            )
            .await
        }
    }
}

#[cfg(test)]
#[path = "ssh_command_tests.rs"]
mod tests;
