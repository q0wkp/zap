//! 把 `SshServerInfo` 拼成 `ssh ...` 命令,并派生测试连接的子进程。
//!
//! 写入 PTY 时调 `build_ssh_command_line`,会用 shell-escape 引用每个 arg,
//! 防止用户名 / host / key_path 里的空格或单引号破坏命令行。
//!
//! ## 密码认证安全
//!
//! `ssh` 在 pipe stdin 模式下能正常从 stdin 读密码,采用一次性
//! stdin 注入(`build_password_auth_stdin`)。密码全程只在内存中以
//! `Zeroizing<String>` 形式持有,不进 argv,不会出现在 `/proc/<pid>/cmdline`、
//! `ps` 等同机可读的进程信息里(对 sshpass `-p` 模式的修复)。

use crate::types::{AuthType, ConnectionStatus, SshServerInfo};
use futures_lite::io::AsyncWriteExt as _;
use std::borrow::Cow;
use std::process::Stdio;
use std::time::Duration;
use zeroize::Zeroizing;

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

    let result = match server.auth_type {
        AuthType::Key => test_key_auth(server).await,
        AuthType::Password | AuthType::OneKey => test_password_auth(server, password).await,
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

async fn test_key_auth(server: &SshServerInfo) -> Result<(), String> {
    let mut args = build_ssh_args(server);
    // build_ssh_args 末尾是 destination (user@host),-o 选项必须插在
    // destination 之前,否则 SSH 把 -o 当作远程命令的一部分而非自身选项。
    let target = args.pop().unwrap();
    args.extend([
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "ConnectTimeout=5".into(),
        "-o".into(),
        "StrictHostKeyChecking=no".into(),
        "-o".into(),
        "LogLevel=ERROR".into(),
    ]);
    args.push(target);
    args.push("echo ok".into());
    let cmd_args = args;

    match tokio::time::timeout(TEST_TIMEOUT, run_ssh_test(&cmd_args)).await {
        Ok(Ok(output)) => {
            // 严格匹配 `echo ok`,不放过 banner/motd 末尾恰好是 "ok" 的误判。
            if output.trim() == "ok" {
                Ok(())
            } else {
                Err(format!("Unexpected output: {}", output.trim()))
            }
        }
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("Connection timeout".into()),
    }
}

async fn test_password_auth(
    server: &SshServerInfo,
    password: Option<Zeroizing<String>>,
) -> Result<(), String> {
    let password = password.ok_or("Password not provided")?;

    // 构造 ssh 命令参数(注意 -o 选项必须插在 destination 之前,见该函数注释)
    let cmd_args = build_password_auth_cmd_args(server);

    test_password_auth_unix(cmd_args, &password).await
}

/// `ssh` 能从 pipe stdin 正常读密码。
async fn test_password_auth_unix(
    cmd_args: Vec<String>,
    password: &Zeroizing<String>,
) -> Result<(), String> {
    let stdin_bytes = build_password_auth_stdin(password);

    let mut child = command::r#async::Command::new("ssh")
        .args(&cmd_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start ssh: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(&stdin_bytes)
            .await
            .map_err(|e| format!("Failed to write password: {e}"))?;
    }

    let output = match tokio::time::timeout(TEST_TIMEOUT, child.output()).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => return Err(format!("Failed to read ssh output: {e}")),
        Err(_) => return Err("Connection timeout".into()),
    };

    finalize_password_test_result(&output)
}

/// 解析 ssh 子进程的输出,统一成功/失败判定逻辑。
fn finalize_password_test_result(output: &std::process::Output) -> Result<(), String> {
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
        Err(format!("Authentication failed: wrong password{detail}"))
    } else {
        Err(format!(
            "Unexpected output: stdout={} stderr={}",
            stdout.trim(),
            stderr_trimmed
        ))
    }
}

/// 把密码编码成要写入 ssh stdin 的字节流:密码 UTF-8 + 换行。
/// 独立成纯函数,便于单测断言"stdin 包含密码字面量 + 换行"。
fn build_password_auth_stdin(password: &Zeroizing<String>) -> Zeroizing<Vec<u8>> {
    let mut v = Zeroizing::new(Vec::with_capacity(password.len() + 1));
    v.extend_from_slice(password.as_bytes());
    v.push(b'\n');
    v
}

/// 拼出 password 认证测试时给 ssh 子进程的完整 argv。
///
/// 与 `build_ssh_args` 不同:这里跳过首项 `"ssh"`(我们用
/// `Command::new("ssh")` 显式派生),追加测试用 `-o` 选项和 `echo ok` 远端命令。
///
/// 关键选项含义:
/// - `BatchMode=no`:允许 ssh 从 stdin 读密码
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
/// `echo ok` 作为远端命令,严格匹配 stdout 判定成功(避免 banner / motd
/// 末尾恰好含 "ok" 的误判)。
///
/// author: logic
/// date: 2026-06-01
fn build_password_auth_cmd_args(server: &SshServerInfo) -> Vec<String> {
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
    args.push("echo ok".into());
    args
}

async fn run_ssh_test(args: &[String]) -> Result<String, std::io::Error> {
    // 统一走 command::r#async 派生子进程(见 .clippy.toml 对 tokio::process::Command 的禁用)。
    let output = command::r#async::Command::new(&args[0])
        .args(&args[1..])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // 成功判定:进程退出码为 0,或远端 `echo ok` 的输出已回传(部分 sshpass
    // 警告会让退出码非零,但 stdout 里仍含 "ok")。
    if output.status.success() || stdout.contains("ok") {
        Ok(stdout)
    } else {
        Err(std::io::Error::other(stderr))
    }
}

#[cfg(test)]
#[path = "ssh_command_tests.rs"]
mod tests;
