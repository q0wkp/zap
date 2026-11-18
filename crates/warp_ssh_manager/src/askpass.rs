//! Answers ssh's password and passphrase prompts with the credentials saved in the SSH manager,
//! through ssh's own `SSH_ASKPASS` mechanism rather than by watching the terminal.
//!
//! A terminal the SSH manager opens gets `SSH_ASKPASS` (the helper script below),
//! `SSH_ASKPASS_REQUIRE=force` and `ZAP_ASKPASS_FIFO` in its shell's environment, so the `ssh` it
//! runs there asks the helper instead of the terminal. The helper answers a password or
//! passphrase prompt from a FIFO that only this user can open, which Zap writes the secret into
//! once and then removes; every other question (a new host key, a one-time code, a retry after a
//! wrong password) it asks on the terminal, as ssh would have. Nothing the remote host prints can
//! make Zap type the secret, which an injector watching the terminal's output could not promise.
//!
//! `SSH_ASKPASS_REQUIRE` needs OpenSSH 8.4 or later; an older ssh ignores it and asks on the
//! terminal as before.

use std::ffi::{CString, OsString};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use zeroize::Zeroizing;

/// The helper ssh runs for each prompt. `ZAP_ASKPASS_TTY` only exists for tests.
pub const HELPER_SCRIPT: &str = r#"#!/bin/sh
# SSH_ASKPASS helper of Zap's SSH manager. ssh runs it, instead of asking on the terminal, for
# every question it has (SSH_ASKPASS_REQUIRE=force). A password or passphrase prompt is answered
# from the one-shot FIFO named by ZAP_ASKPASS_FIFO, which Zap writes once and then removes; every
# other question, and a password prompt once the FIFO is gone, is asked on the terminal.
prompt=$1
tty=${ZAP_ASKPASS_TTY:-/dev/tty}

ask() {
    printf '%s' "$prompt" >> "$tty"
    if [ "$1" = hidden ]; then
        stty -echo < "$tty" 2> /dev/null
        IFS= read -r answer < "$tty"
        stty echo < "$tty" 2> /dev/null
        printf '\n' >> "$tty"
    else
        IFS= read -r answer < "$tty"
    fi
}

case ${SSH_ASKPASS_PROMPT:-} in
    confirm)
        ask visible
        case $answer in
            [yY] | [yY][eE][sS]) exit 0 ;;
        esac
        exit 1
        ;;
    none)
        printf '%s\n' "$prompt" >> "$tty"
        exit 0
        ;;
esac

case $prompt in
    *"(yes/no"*)
        ask visible
        printf '%s\n' "$answer"
        exit 0
        ;;
    *[Pp]assword* | *[Pp]assphrase*)
        fifo=${ZAP_ASKPASS_FIFO:-}
        if [ -n "$fifo" ] && [ -p "$fifo" ]; then
            exec cat "$fifo"
        fi
        ;;
esac
ask hidden
printf '%s\n' "$answer"
"#;

/// How long a saved secret waits for ssh to ask for it before its FIFO is removed.
pub const SERVE_TIMEOUT: Duration = Duration::from_secs(300);

const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// A one-shot FIFO prepared for one connection, before its terminal is created.
#[derive(Debug)]
pub struct AskpassChannel {
    fifo: PathBuf,
}

impl AskpassChannel {
    /// Prepares the helper and a fresh FIFO in Zap's private askpass directory, and returns the
    /// environment the connection's shell must start with.
    pub fn prepare() -> io::Result<(Self, Vec<(OsString, OsString)>)> {
        Self::prepare_in(&askpass_dir()?)
    }

    pub fn prepare_in(dir: &Path) -> io::Result<(Self, Vec<(OsString, OsString)>)> {
        ensure_private_dir(dir)?;
        let helper = dir.join("askpass.sh");
        write_helper(&helper)?;
        let fifo = dir.join(format!("{}.fifo", uuid::Uuid::new_v4()));
        make_fifo(&fifo)?;
        let env = vec![
            (OsString::from("SSH_ASKPASS"), helper.into_os_string()),
            (OsString::from("SSH_ASKPASS_REQUIRE"), OsString::from("force")),
            (OsString::from("ZAP_ASKPASS_FIFO"), fifo.clone().into_os_string()),
        ];
        Ok((AskpassChannel { fifo }, env))
    }

    /// Writes `secret` once to the first reader of the FIFO, on a thread of its own, then removes
    /// the FIFO; after `timeout` without a reader it just removes it.
    pub fn serve(self, secret: Zeroizing<String>, timeout: Duration) {
        let spawned = std::thread::Builder::new()
            .name("ssh-askpass".to_owned())
            .spawn(move || {
                if let Err(e) = self.write_once(&secret, timeout) {
                    log::debug!("ssh askpass: secret not handed over: {e}");
                }
            });
        if let Err(e) = spawned {
            log::warn!("ssh askpass: failed to start the serving thread: {e}");
        }
    }

    fn write_once(&self, secret: &str, timeout: Duration) -> io::Result<()> {
        let deadline = Instant::now() + timeout;
        // Opening a FIFO for writing without a reader fails with ENXIO when non-blocking, so
        // poll until ssh's helper opens it or the time is up.
        let mut file = loop {
            match OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&self.fifo)
            {
                Ok(file) => break file,
                Err(e) if e.raw_os_error() == Some(libc::ENXIO) && Instant::now() < deadline => {
                    std::thread::sleep(POLL_INTERVAL);
                }
                Err(e) => return Err(e),
            }
        };
        let mut line = Zeroizing::new(Vec::with_capacity(secret.len() + 1));
        line.extend_from_slice(secret.as_bytes());
        line.push(b'\n');
        file.write_all(&line)
    }
}

impl Drop for AskpassChannel {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.fifo);
    }
}

/// `$XDG_RUNTIME_DIR/zap-askpass` where there is one (Linux), else a per-user directory in the
/// temporary directory (on macOS already private to the user).
fn askpass_dir() -> io::Result<PathBuf> {
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) {
        if runtime.is_dir() {
            return Ok(runtime.join("zap-askpass"));
        }
    }
    // SAFETY: getuid has no preconditions and cannot fail.
    let uid = unsafe { libc::getuid() };
    Ok(std::env::temp_dir().join(format!("zap-askpass-{uid}")))
}

/// Creates `dir` readable only by this user, or checks that an existing one is.
fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    match fs::create_dir(dir) {
        Ok(()) => fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    let meta = fs::symlink_metadata(dir)?;
    // SAFETY: getuid has no preconditions and cannot fail.
    let uid = unsafe { libc::getuid() };
    if !meta.is_dir() || meta.uid() != uid || meta.mode() & 0o077 != 0 {
        return Err(io::Error::other(format!(
            "{} is not a directory private to this user",
            dir.display()
        )));
    }
    Ok(())
}

/// Writes the helper through a temporary file and a rename, so a running ssh never reads a
/// half-written script.
fn write_helper(path: &Path) -> io::Result<()> {
    if fs::read(path).is_ok_and(|current| current == HELPER_SCRIPT.as_bytes()) {
        return Ok(());
    }
    let tmp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o700)
        .open(&tmp)?;
    file.write_all(HELPER_SCRIPT.as_bytes())?;
    drop(file);
    fs::rename(&tmp, path)
}

fn make_fifo(path: &Path) -> io::Result<()> {
    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::other("FIFO path contains a NUL byte"))?;
    // SAFETY: c_path is a valid NUL-terminated string that outlives the call.
    if unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
#[path = "askpass_tests.rs"]
mod tests;
