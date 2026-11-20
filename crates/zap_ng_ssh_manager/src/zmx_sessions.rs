// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

//! 列出服务器上的 [zmx](https://github.com/neurosnap/zmx) session,并取每个
//! session 输出的末尾若干行作预览。
//!
//! zmx 没有可供远端查询的协议,能问的只有服务器上的 `zmx list` 和
//! `zmx history`。这里用和连接测试相同的认证方式派生一次性的 ssh 子进程
//! 去跑它们,服务器上除 zmx 本身外不需要安装任何东西。

use std::time::Duration;

use uuid::Uuid;
use zeroize::Zeroizing;

use crate::ssh_command::{SshRunError, UnknownHostKey, run_remote_command};
use crate::types::SshServerInfo;

/// 整次查询的上限:一次 ssh 握手 + `zmx list` + 每个 session 一次 `zmx history`。
const LIST_TIMEOUT: Duration = Duration::from_secs(20);

/// 每个 session 从服务器取回的行数。比 `PREVIEW_LINES` 多,因为屏幕下半
/// 经常是空行,裁掉之后还要剩得下内容。
const PREVIEW_FETCH_LINES: usize = 60;

/// 预览里保留的行数。
pub const PREVIEW_LINES: usize = 24;

/// 预览里每行保留的字符数,超出的截掉(session 的终端可能比预览区宽得多)。
const PREVIEW_LINE_CHARS: usize = 200;

/// 服务器上的一个 zmx session。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZmxSession {
    pub name: String,
    /// 已 attach 的客户端数。
    pub clients: Option<u32>,
    /// session 当前的工作目录。
    pub cwd: Option<String>,
    /// 创建 session 时交给 `zmx attach` 的命令;直接开 shell 的 session 没有。
    pub command: Option<String>,
    /// session 的 PTY 上现在处于前台的进程名;旧版本的 zmx 不报这一项。
    pub foreground: Option<String>,
    /// session 输出的末尾若干行纯文本,取不到时为 `None`。
    pub preview: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZmxSessionsError {
    /// 服务器上找不到 zmx。
    NotInstalled,
    /// ssh has no record of the server's host key: show it to the user and list again with its
    /// fingerprint once they confirm it.
    UnknownHostKey(UnknownHostKey),
    /// ssh 失败或输出无法解析;内容是可以直接显示的原因。
    Failed(String),
}

/// 列出 `server` 上的 zmx session。
///
/// `trusted_fingerprint` is the fingerprint of a host key the user has just confirmed, from an
/// earlier [`ZmxSessionsError::UnknownHostKey`].
pub async fn list_zmx_sessions(
    server: &SshServerInfo,
    password: Option<Zeroizing<String>>,
    trusted_fingerprint: Option<&str>,
) -> Result<Vec<ZmxSession>, ZmxSessionsError> {
    let marker = format!("ZMX{}", Uuid::new_v4().simple());
    let command = build_list_command(&marker);
    let output = run_remote_command(
        server,
        password,
        trusted_fingerprint,
        &command,
        LIST_TIMEOUT,
    )
    .await
    .map_err(|e| match e {
        SshRunError::UnknownHostKey(key) => ZmxSessionsError::UnknownHostKey(key),
        SshRunError::Failed(message) => ZmxSessionsError::Failed(message),
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.contains(&marker) {
        // 脚本没跑起来:认证失败、连不上,或登录 shell 拒绝执行。
        return Err(ZmxSessionsError::Failed(ssh_failure_message(
            &String::from_utf8_lossy(&output.stderr),
        )));
    }
    parse_list_output(&stdout, &marker)
}

/// 拼出在服务器上执行的命令。输出的各段以 `marker` 开头的行分隔:
///
/// ```text
/// <marker>:begin
/// <marker>:session
/// <`zmx list` 的一行>
/// <该 session 输出的末尾若干行>
/// <marker>:session
/// ...
/// <marker>:end
/// ```
///
/// 找不到 zmx 时只输出 `<marker>:begin` 和 `<marker>:missing`。
///
/// 写法上的约束:
/// - 整条命令由用户的登录 shell 解析(可能是 fish、csh),所以只交给它一个
///   单引号字符串,真正的脚本由 `sh` 执行;脚本里因此不能出现单引号。
/// - 非交互的 ssh 不读 shell 的 rc 文件,PATH 往往只有系统目录,所以用
///   `sh -l` 并补上 zmx 常见的安装位置。补在后面,不盖过用户自己的 PATH。
/// - `zmx list` 逐个探测 session 的 socket,只跑一次;session 名从它的输出里
///   取,不再跑第二次 `zmx list --short`。
/// - `zmx history` 的输出不一定以换行结尾,后面补一个 `echo`,否则下一个
///   标记会接在最后一行的行尾。
/// - 某个 session 的 daemon 卡住时 `zmx history` 不返回,有 `timeout` 就用它
///   限时,免得一个 session 拖掉整次查询。
fn build_list_command(marker: &str) -> String {
    let script = format!(
        concat!(
            "PATH=\"$PATH:$HOME/.local/bin:/usr/local/bin:/opt/homebrew/bin:",
            "/home/linuxbrew/.linuxbrew/bin\"; ",
            "echo {marker}:begin; ",
            "command -v zmx >/dev/null 2>&1 || {{ echo {marker}:missing; exit 0; }}; ",
            "t=; command -v timeout >/dev/null 2>&1 && t=\"timeout 3\"; ",
            "zmx list 2>/dev/null | while IFS= read -r l; do ",
            "echo {marker}:session; ",
            "printf \"%s\\n\" \"$l\"; ",
            "case \"$l\" in *\"\tpid=\"*) ",
            "n=${{l#*name=}}; n=${{n%%\"\t\"*}}; ",
            "$t zmx history \"$n\" 2>/dev/null </dev/null | tail -n {lines}; echo;; ",
            "esac; ",
            "done; ",
            "echo {marker}:end",
        ),
        marker = marker,
        lines = PREVIEW_FETCH_LINES,
    );
    format!("sh -lc '{script}'")
}

/// 解析 `build_list_command` 的输出。`<marker>:begin` 之前的内容(登录 shell
/// 的 rc 文件打印的东西)被丢掉。
fn parse_list_output(output: &str, marker: &str) -> Result<Vec<ZmxSession>, ZmxSessionsError> {
    let begin = format!("{marker}:begin");
    let missing = format!("{marker}:missing");
    let session = format!("{marker}:session");
    let end = format!("{marker}:end");

    let mut lines = output.lines().map(|line| line.trim_end_matches('\r'));
    if !lines.any(|line| line == begin) {
        return Err(ZmxSessionsError::Failed("Unexpected output".into()));
    }

    let mut sessions: Vec<ZmxSession> = Vec::new();
    // 当前这一段:`zmx list` 的那一行解析出的 session(解析不出时为 None)
    // 和它后面的预览行。
    let mut current: Option<(Option<ZmxSession>, Vec<&str>)> = None;
    for line in lines {
        if line == missing {
            return Err(ZmxSessionsError::NotInstalled);
        }
        if line == session || line == end {
            push_section(&mut sessions, current.take());
            if line == end {
                break;
            }
            current = Some((None, Vec::new()));
            continue;
        }
        match current.as_mut() {
            // 标记后的第一行是 `zmx list` 的输出。
            Some((parsed, preview)) if parsed.is_none() && preview.is_empty() => {
                match parse_session_line(line) {
                    Some(session) => *parsed = Some(session),
                    // 占住位置,后面的行才不会被再当成 `zmx list` 的输出。
                    None => preview.push(""),
                }
            }
            Some((_, preview)) => preview.push(line),
            None => {}
        }
    }
    // 输出被截断(没有 end 标记)时,已经读到的最后一段也算数。
    push_section(&mut sessions, current.take());
    Ok(sessions)
}

fn push_section(sessions: &mut Vec<ZmxSession>, section: Option<(Option<ZmxSession>, Vec<&str>)>) {
    let Some((Some(mut session), preview)) = section else {
        return;
    };
    session.preview = trim_preview(&preview);
    // zmx 的 session 名不会重复;输出异常时保留后出现的那个。
    sessions.retain(|existing| existing.name != session.name);
    sessions.push(session);
}

/// 解析 `zmx list` 的一行:以 tab 分隔的 `key=value`,当前 session 前有 `→ `。
/// 探测失败的 session(`name=..<tab>err=..`)和无关的输出返回 `None`。
fn parse_session_line(line: &str) -> Option<ZmxSession> {
    let line = line.trim_start().trim_start_matches('→').trim();
    let mut fields = line.split('\t').map(|field| {
        // 命令里可以有 `=`,只在第一个 `=` 处切开。
        field.split_once('=').unwrap_or((field, ""))
    });

    let (key, name) = fields.next()?;
    if key != "name" || name.is_empty() {
        return None;
    }
    // 活着的 session 第二个字段一定是 pid。
    let (key, _) = fields.next()?;
    if key != "pid" {
        return None;
    }

    let mut session = ZmxSession {
        name: name.to_string(),
        clients: None,
        cwd: None,
        command: None,
        foreground: None,
        preview: None,
    };
    for (key, value) in fields {
        match key {
            "clients" => session.clients = value.parse().ok(),
            // zmx 0.7.0 把这个字段叫 start_dir,值是不带 scheme 的路径。
            "cwd" | "start_dir" => session.cwd = decode_cwd(value),
            "cmd" => session.command = (!value.is_empty()).then(|| value.to_string()),
            // zmx 自己的字段排在 label 前面,所以只认第一个 fg,后面同名的是 label。
            "fg" if session.foreground.is_none() => {
                session.foreground = (!value.is_empty()).then(|| value.to_string())
            }
            // 其余是创建时间、任务状态(ended / exit_code)和用户自己加的 label,这里用不上。
            _ => {}
        }
    }
    Some(session)
}

/// `cwd` 是 OSC 7 的 URI(`file://<host><path>`,path 经过百分号编码),
/// 旧版本是裸路径。
fn decode_cwd(value: &str) -> Option<String> {
    let path = match value.strip_prefix("file://") {
        Some(rest) => percent_decode(&rest[rest.find('/')?..]),
        None => value.to_string(),
    };
    (!path.is_empty()).then_some(path)
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = (bytes[i] == b'%')
            .then(|| text.get(i + 1..i + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                i += 3;
            }
            None => {
                decoded.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// 去掉首尾的空行和行尾空白,只留末尾 `PREVIEW_LINES` 行。全是空行时返回 `None`。
fn trim_preview(lines: &[&str]) -> Option<String> {
    let end = lines.iter().rposition(|line| !line.trim().is_empty())? + 1;
    let start = end.saturating_sub(PREVIEW_LINES);
    let kept = &lines[start..end];
    let first = kept.iter().position(|line| !line.trim().is_empty())?;
    Some(
        kept[first..]
            .iter()
            .map(|line| {
                line.trim_end()
                    .chars()
                    .take(PREVIEW_LINE_CHARS)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// 从 ssh 的 stderr 里取一段给用户看的原因。
fn ssh_failure_message(stderr: &str) -> String {
    let stderr = stderr.trim();
    if stderr.is_empty() {
        return "Unexpected output".into();
    }
    let snippet: String = stderr.chars().take(200).collect();
    if stderr.chars().count() > 200 {
        format!("{snippet}...")
    } else {
        snippet
    }
}

#[cfg(test)]
#[path = "zmx_sessions_tests.rs"]
mod tests;
