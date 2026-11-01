//! Support for sessions run under [zmx](https://github.com/neurosnap/zmx).
//!
//! `zmx attach` looks like any other long-running command, so nothing in the block says what is
//! running inside the session. zmx reports that with a user variable each time a client attaches.
//! Before zmx did, a program could only be found in the command that attached to the session.

/// The user variable that zmx sets to the session's foreground process on attach.
pub const FOREGROUND_USER_VAR: &str = "zmx_foreground";

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
