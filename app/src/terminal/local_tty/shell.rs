use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
use warp_core::channel::{Channel, ChannelState};
use warp_util::path::warp_shell_path;

use crate::{
    terminal::{
        available_shells::AvailableShell,
        bootstrap::init_shell_script_for_shell,
        local_tty::docker_sandbox::DockerSandboxShellStarter,
        shell::{ShellName, ShellType},
        ShellLaunchData,
    },
    util::path::resolve_executable,
};

pub const ZSH_SHELL_PATH: &str = "/bin/zsh";
pub const BASH_SHELL_PATH: &str = "/bin/bash";
pub const FISH_SHELL_PATH: &str = "/bin/fish";

/// Returns an iterator of additional PATH entries to append to the shell's PATH.
/// * On macOS, this includes `$APP_PATH/Contents/Resources/bin`, in which we put a wrapper around the Zap CLI.
/// * On all other platforms, this is empty.
pub fn extra_path_entries() -> impl Iterator<Item = PathBuf> {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
            use itertools::Either;

            if let Some(resources_path) = warp_core::paths::bundled_resources_dir() {
                let bin_path = resources_path.join("bin");
                Either::Left(std::iter::once(bin_path))
            } else {
                Either::Right(std::iter::empty())
            }
        } else {
            std::iter::empty()
        }
    }
}

/// Returns `true` if the given `path_or_command` is a valid, executable command or path to a
/// executable binary for one of Zap's supported shell types (bash, fish, zsh).
pub fn is_valid_path_or_command_for_supported_shell(path_or_command: &str) -> bool {
    supported_shell_path_and_type(path_or_command).is_some()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellStarter {
    /// Bootstrap the shell directly.
    Direct(DirectShellStarter),
    /// Bootstrap a shell running inside a Docker sandbox via `sbx run`.
    /// The final `sbx` args are computed at PTY spawn time so we can include
    /// the resolved workspace path, read-only init-script mount, and base
    /// Docker image (`--template <base_image>`).
    DockerSandbox(DockerSandboxShellStarter),
}

impl ShellStarter {
    /// Constructs a `ShellStarter` represent the shell binary (and corresponding arguments) to be
    /// used to spawn a shell process for a new top-level Zap session.
    ///
    /// Returns an enum indicating the source from which the shell was determined. If the fallback
    /// default shell is used, also includes the requested but unsupported shell information.
    pub fn init(preferred_shell: AvailableShell) -> Option<ShellStarterSource> {
        if let Some(launch_data) = preferred_shell.get_valid_shell_path_and_type() {
            match launch_data {
                ShellLaunchData::Executable {
                    executable_path,
                    shell_type,
                } => {
                    return Some(ShellStarterSource::Override(ShellStarter::Direct(
                        DirectShellStarter {
                            args: arguments_for_session_spawning_command(
                                executable_path.to_string_lossy().as_ref(),
                                shell_type,
                            ),
                            shell_path: executable_path,
                            shell_type,
                        },
                    )));
                }
                ShellLaunchData::DockerSandbox {
                    sbx_path,
                    base_image,
                } => {
                    // The sandbox runs `sbx` on the host; the actual shell
                    // lives inside the container (conventionally bash). We
                    // still thread a `DirectShellStarter` with `shell_type =
                    // Bash` through so existing code that asks for the
                    // "shell type" of the session gets a sensible answer.
                    return Some(ShellStarterSource::Override(ShellStarter::DockerSandbox(
                        DockerSandboxShellStarter::new(
                            DirectShellStarter {
                                args: Vec::new(),
                                shell_path: sbx_path,
                                shell_type: ShellType::Bash,
                            },
                            base_image,
                        ),
                    )));
                }
            }
        }

        if let Some(warp_shell_env_var) = warp_shell_path() {
            let (warp_shell_path, shell_type) = supported_shell_path_and_type(&warp_shell_env_var)
                .unwrap_or_else(|| {
                    panic!("Cannot spawn shell; $WARP_SHELL_PATH is invalid: {warp_shell_env_var}")
                });
            return Some(ShellStarterSource::Environment(DirectShellStarter {
                args: arguments_for_session_spawning_command(
                    warp_shell_path.as_path().to_string_lossy().as_ref(),
                    shell_type,
                ),
                shell_path: warp_shell_path,
                shell_type,
            }));
        }

        Self::compute_fallback_shell()
    }

    fn compute_fallback_shell() -> Option<ShellStarterSource> {
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                let pw_shell_path = nix::unistd::User::from_uid(nix::unistd::getuid())
                    .expect("should not fail to read user information")
                    .expect("current user should exist")
                    .shell
                    .display()
                    .to_string();
                if let Some((resolved_pw_shell_path, shell_type)) =
                    supported_shell_path_and_type(&pw_shell_path)
                {
                    return Some(ShellStarterSource::UserDefault(DirectShellStarter {
                        args: arguments_for_session_spawning_command(
                            resolved_pw_shell_path.as_path().to_string_lossy().as_ref(),
                            shell_type,
                        ),
                        shell_path: resolved_pw_shell_path,
                        shell_type,
                    }));
                }
                let unsupported_shell = Some(pw_shell_path);

                let (resolved_default_shell_path, shell_type) = if let Some(shell_path_and_type) =
                    supported_shell_path_and_type(ZSH_SHELL_PATH)
                {
                    shell_path_and_type
                } else if let Some(shell_path_and_type) = supported_shell_path_and_type(BASH_SHELL_PATH) {
                    shell_path_and_type
                } else if let Some(shell_path_and_type) = supported_shell_path_and_type(FISH_SHELL_PATH) {
                    shell_path_and_type
                } else {
                    log::warn!("Did not find valid binaries when attempting to load fallback shell (not bash, fish, or zsh).");
                    return None;
                };

                Some(ShellStarterSource::Fallback {
                    unsupported_shell,
                    starter: DirectShellStarter {
                        args: arguments_for_session_spawning_command(
                            resolved_default_shell_path.as_path().to_string_lossy().as_ref(),
                            shell_type,
                        ),
                        shell_path: resolved_default_shell_path,
                        shell_type,
                    },
                })
            }
        }
    }

    pub fn shell_type(&self) -> ShellType {
        match self {
            ShellStarter::Direct(starter) => starter.shell_type(),
            ShellStarter::DockerSandbox(starter) => starter.shell_type(),
        }
    }

    pub fn is_docker_sandbox(&self) -> bool {
        matches!(self, ShellStarter::DockerSandbox(_))
    }

    fn display_name(&self) -> &str {
        match self {
            Self::Direct(starter) => starter.display_name(),
            Self::DockerSandbox(starter) => starter.display_name(),
        }
    }
}

/// Wraps up a shell type and the command to start it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectShellStarter {
    shell_type: ShellType,
    shell_path: PathBuf,

    /// Arguments to be passed to the shell binary at [`shell_path`] when spawning a new Zap
    /// session.
    args: Vec<OsString>,
}

#[derive(Debug)]
pub enum ShellStarterSource {
    /// The user chose the path by setting a custom shell path in settings.
    Override(ShellStarter),
    /// The user chose the path to the shell by setting the `WARP_SHELL_PATH` environment variable.
    Environment(DirectShellStarter),
    /// The default shell for the user (as indicated by the user's passwd entry on UNIX).
    UserDefault(DirectShellStarter),
    /// We weren't able to find a shell that could be bootstrapped for the user.
    Fallback {
        unsupported_shell: Option<String>,
        starter: DirectShellStarter,
    },
}

impl ShellStarterSource {
    #[cfg(test)]
    pub fn shell_type(&self) -> ShellType {
        match self {
            Self::Override(starter) => starter.shell_type(),
            Self::Environment(starter) => starter.shell_type(),
            Self::UserDefault(starter) => starter.shell_type(),
            Self::Fallback { starter, .. } => starter.shell_type(),
        }
    }

    fn display_name(&self) -> &str {
        match self {
            Self::Override(starter) => starter.display_name(),
            Self::Environment(starter) => starter.display_name(),
            Self::UserDefault(starter) => starter.display_name(),
            Self::Fallback { starter, .. } => starter.display_name(),
        }
    }

    pub fn name(&self) -> ShellName {
        ShellName::MoreDescriptive(self.display_name().to_owned())
    }
}

impl From<ShellStarterSource> for ShellStarter {
    fn from(value: ShellStarterSource) -> Self {
        match value {
            ShellStarterSource::Override(starter) => starter,
            ShellStarterSource::Environment(starter) => Self::Direct(starter),
            ShellStarterSource::UserDefault(starter) => Self::Direct(starter),
            ShellStarterSource::Fallback { starter, .. } => Self::Direct(starter),
        }
    }
}

impl DirectShellStarter {
    pub fn shell_path(&self) -> &Path {
        &self.shell_path
    }

    /// Returns the logical path to the shell binary referred to by this `ShellStarter's`
    /// `ShellPath`.
    pub fn logical_shell_path(&self) -> &Path {
        self.shell_path.as_ref()
    }

    pub fn shell_type(&self) -> ShellType {
        self.shell_type
    }

    pub fn args(&self) -> &Vec<OsString> {
        &self.args
    }

    pub(super) fn display_name(&self) -> &str {
        if self
            .shell_path
            .file_stem()
            .is_some_and(|stem| stem.eq_ignore_ascii_case("powershell"))
        {
            "Windows PowerShell"
        } else {
            self.shell_type.name()
        }
    }
}

/// If the given `path_or_command` resolves to a supported shell binary, returns a tuple
/// containing the resolved path to the binary and the corresponding `ShellType`. Else, returns
/// None.
pub fn supported_shell_path_and_type(path_or_command: &str) -> Option<(PathBuf, ShellType)> {
    resolve_executable(path_or_command)
        .and_then(|resolved_path| parse_shell_type_from_path(resolved_path.as_ref()))
}

/// If the given `path` is a supported shell binary, returns a tuple containing
/// the path and the corresponding `ShellType`. This function does not validate
/// that the path exists or is executable.
fn parse_shell_type_from_path(path: &Path) -> Option<(PathBuf, ShellType)> {
    path.file_name()
        .and_then(|file_name| file_name.to_str().and_then(ShellType::from_name))
        .map(|shell_type| (path.to_path_buf(), shell_type))
}

fn arguments_for_session_spawning_command(
    resolved_shell_path: &str,
    shell_type: ShellType,
) -> Vec<OsString> {
    // Note we typically go through bash so that we can launch the user's shell
    // with a leading '-', making it a login shell.
    match shell_type {
        ShellType::Zsh => {
            // The --no-rcs option executes the minimal level of startup files so we can
            // take over. The one exception: "Commands are first read from /etc/zshenv; this cannot be overridden."
            // The -g option sets the HIST_IGNORE_SPACE option, which ignores a command from history if it
            // begins with a space. We use this to hide Zap bootstrap commands from the history.
            vec![
                "-c".to_owned().into(),
                format!("exec -a -zsh '{resolved_shell_path}' -g --no-rcs").into(),
            ]
        }
        ShellType::Bash => {
            /*
             * There are many layers of bash happening here
             * 1. We pass the command we want to be running to bash -c to ensure the shell
             * is interpreting the arguments, rather than passing literal strings
             * 2. Make a call to exec when we launch the subshell. From FreeBSD porter's
             * handbook:  "The exec statement replaces the shell process with the
             * specified program. If exec is omitted, the shell process remains
             * in memory while the program is executing, and needlessly consumes system resources."
             * 3. The rcfile option reads the startup script from a file
             * 4. Process substitution i.e. <() send the output of a process via
             * /dev/fd/<n> (or temp files if this is unavailable) to another process
             * 5. Send an InitShell message to Zap through escape sequences.
             * The warp_send_message function is inlined here.
             * 6. We disable PS2 and the line editor to work around a gnarly bug involving
             * garbage being inserted in every line. We further disable PS1 and echo'ing
             * in order to show nothing to the user when we input characters. We later
             * restore the echo'ing in the bootstrap script.
             *
             * TODO(zheng) Add error handling
             */
            vec![
                "-c".to_owned().into(),
                // Keep this command up-to-date with the one in the bootstrap script
                // Notice the first level of escaping is the double-brackets in the macro string {{}}
                format!(
                    r#"exec -a bash '{}' --rcfile <(echo '{}')"#,
                    resolved_shell_path,
                    init_shell_script_for_shell(ShellType::Bash, &crate::ASSETS)
                )
                .into(),
            ]
        }
        ShellType::Fish => {
            // For now, we are going to plug the init cmd into single quotes.
            // Note it contains single quotes and there is no way to escape a single quote,
            // so instead we exit single quotes, emit an (escaped) quote, and re-enter them.
            //
            // TODO: we should eventually refactor this and build the init cmd up so that
            // we don't need to do complicated escaping.
            // We should also probably store the hex encoded json as a static string
            // rather than computing it at runtime for each shell we start.
            vec![
                // fish sources configuration files whenever it's invoked,
                // including in non-interactive mode (i.e. '-c'). This differs
                // from other shells like zsh, so we have to explicitly tell fish
                // to not source config files.
                //
                // There's an open GH issue against fish contesting this behaviour:
                // https://github.com/fish-shell/fish-shell/issues/5394.
                "--no-config".to_owned().into(),
                "-c".to_owned().into(),
                format!(
                    // We do _not_ specify `--no-config` here because
                    // we want fish to source config files for us (we don't
                    // manually do so in the bootstrap script like we do for zsh, for example).
                    // `-f no-mark-prompt` disables OSC 133 (the non-standard FinalTerm escape codes).
                    // Fish's implementation of this breaks Zap by emitting `OSC 133 A` but not
                    // `OSC 133 B` afterwards, which we have assumed. This is a temporary workaround.
                    // See this issue: https://github.com/zerx-lab/warp/issues/7588
                    r#"exec '{}' -f no-mark-prompt --login --init-command '{}'"#,
                    resolved_shell_path,
                    init_shell_script_for_shell(ShellType::Fish, &crate::ASSETS)
                )
                .into(),
            ]
        }
        ShellType::PowerShell => vec![
            // When PowerShell starts a session, it writes "PowerShell <version>" to the PTY. This
            // option suppresses that message.
            "-NoLogo".to_owned().into(),
            // Skip RC files. We load these manually later.
            "-NoProfile".to_owned().into(),
            // Normally, passing the "-Command" option causes the shell to exit after executing
            // those commands. Passing "-NoExit" suppresses that so PowerShell remains interactive
            // afterwards.
            "-NoExit".to_owned().into(),
            // This arg must be last, as everything positioned after the "-Command" flag is treated
            // as the value for this arg.
            "-Command".to_owned().into(),
            init_shell_script_for_shell(ShellType::PowerShell, &crate::ASSETS).into(),
        ],
    }
}

pub fn ssh_socket_dir() -> String {
    let mut socket_dir = if ChannelState::channel() == Channel::Integration {
        std::env::var("ORIGINAL_HOME").unwrap_or("~".into())
    } else {
        "~".into()
    };
    socket_dir.push_str("/.ssh");
    socket_dir
}

#[cfg(test)]
#[path = "shell_tests.rs"]
mod tests;
