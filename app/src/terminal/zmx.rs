// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

//! Support for sessions run under [zmx](https://github.com/neurosnap/zmx).
//!
//! `zmx attach` looks like any other long-running command, so nothing in the block says what is
//! running inside the session. zmx reports that with a user variable each time a client attaches.
//! Before zmx did, a program could only be found in the command that attached to the session.

use std::borrow::Cow;

/// The user variable that zmx sets to the session's foreground process on attach.
pub const FOREGROUND_USER_VAR: &str = "zmx_foreground";

/// The user variable that zmx sets to the session's name when a client attaches or is switched
/// to it.
pub const SESSION_USER_VAR: &str = "zmx_session";

/// Wraps `script` into a command that any login shell hands to `sh` unchanged, and that finds a
/// zmx installed where the PATH of a non-interactive command doesn't reach. `script` must not
/// contain single quotes; what follows it becomes its positional parameters.
fn sh_command(script: &str, args: &[&str]) -> String {
    let mut command = format!(
        "sh -lc 'PATH=\"$PATH:$HOME/.local/bin:/usr/local/bin:/opt/homebrew/bin:\
         /home/linuxbrew/.linuxbrew/bin\"; {script}' sh"
    );
    for arg in args {
        command.push(' ');
        command.push_str(&shell_escape::unix::escape(Cow::Borrowed(*arg)));
    }
    command
}

/// The command that prints the names of the host's zmx sessions, one a line.
pub fn list_sessions_command() -> String {
    sh_command("ZMX_SESSION_PREFIX= zmx list --short 2>/dev/null", &[])
}

/// The names `list_sessions_command` printed.
pub fn parse_session_names(output: &str) -> Vec<String> {
    output
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The command that moves the client attached to `current` over to `target`. Inside a session
/// (`ZMX_SESSION` set) `zmx attach` doesn't attach: it asks that session's client to switch, which
/// is why this can run beside the terminal instead of in it. Both names are full session names,
/// as zmx reports them, so the prefix zmx would add is cleared.
pub fn switch_session_command(current: &str, target: &str) -> String {
    sh_command(
        "ZMX_SESSION=\"$1\" ZMX_SESSION_PREFIX= zmx attach \"$2\" >/dev/null 2>&1",
        &[current, target],
    )
}

/// Returns the program given to `zmx attach <session> [program...]`, if any.
pub fn attach_command(command: &str) -> Option<&str> {
    let (executable, rest) = next_word(command)?;
    if executable.rsplit('/').next() != Some("zmx") {
        return None;
    }
    let (subcommand, mut rest) = next_word(rest)?;
    if !matches!(subcommand, "a" | "attach") {
        return None;
    }
    loop {
        let (word, after_word) = next_word(rest)?;
        rest = after_word;
        if !word.starts_with('-') {
            break;
        }
    }
    let program = rest.trim();
    (!program.is_empty()).then_some(program)
}

fn next_word(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start();
    if text.is_empty() {
        return None;
    }
    Some(text.split_once(char::is_whitespace).unwrap_or((text, "")))
}

#[cfg(test)]
#[path = "zmx_tests.rs"]
mod tests;
