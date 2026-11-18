//! Connecting a terminal to a server saved in the SSH manager: typing its `ssh` command line
//! into the terminal's local shell, and answering the password and passphrase prompts that
//! follow with the credentials saved in the keychain, through ssh's askpass (see
//! [`warp_ssh_manager::askpass`]).

use std::collections::HashMap;
use std::ffi::OsString;

use warp_ssh_manager::{
    KeychainSecretStore, SecretKind, SshRepository, SshSecretStore, SshServerInfo,
};
use warpui::{SingletonEntity, View, ViewContext, ViewHandle};

use crate::settings::{AISettings, DefaultSessionMode};
use crate::terminal::TerminalView;

/// The secret channel of a connection the SSH manager is about to make, which its terminal's
/// shell must have been started with the environment of.
#[cfg(unix)]
pub type ConnectionAskpass = warp_ssh_manager::askpass::AskpassChannel;
#[cfg(not(unix))]
pub enum ConnectionAskpass {}

/// Prepares the askpass channel for a connection, before its terminal is created, and returns the
/// environment that terminal's shell must start with. Without it (`None` and no variables) ssh
/// asks for the password on the terminal.
pub fn prepare_askpass() -> (Option<ConnectionAskpass>, HashMap<OsString, OsString>) {
    #[cfg(unix)]
    match warp_ssh_manager::askpass::AskpassChannel::prepare() {
        Ok((channel, env)) => return (Some(channel), env.into_iter().collect()),
        Err(e) => log::warn!("ssh askpass unavailable, ssh will ask on the terminal: {e}"),
    }
    (None, HashMap::new())
}

/// A connection the SSH manager made in a terminal.
#[derive(Clone, Debug)]
pub struct SshManagerConnection {
    /// The id of the server's node in the SSH manager's tree.
    pub node_id: String,
    pub server: SshServerInfo,
    /// The `ssh` command line typed into the terminal.
    pub command: String,
}

/// Connects `terminal_view`, a terminal whose local shell is starting, to `server`: queues its
/// `ssh` command line, hands the saved password or passphrase to `askpass` (whose environment the
/// terminal's shell was started with), and starts the injector that offers the root password at
/// `su`. The injector is owned by `ctx`'s view. The server's startup command (and its zmx
/// session) runs once the remote shell is ready if `run_startup_command`.
pub fn connect_terminal<O: View>(
    terminal_view: &ViewHandle<TerminalView>,
    node_id: String,
    server: SshServerInfo,
    askpass: Option<ConnectionAskpass>,
    run_startup_command: bool,
    ctx: &mut ViewContext<O>,
) {
    let (server_for_connection, secret_lookup_id, secret_kind) =
        match warp_ssh_manager::with_conn(|conn| {
            let resolved_auth = SshRepository::resolve_server_auth(conn, &server)?;
            let mut server_for_connection = server.clone();
            server_for_connection.username = resolved_auth.username;
            server_for_connection.auth_type = resolved_auth.auth_type;
            server_for_connection.key_path = resolved_auth.key_path;
            Ok((
                server_for_connection,
                resolved_auth.secret_lookup_id,
                resolved_auth.secret_kind,
            ))
        }) {
            Ok(resolved) => resolved,
            Err(e) => {
                log::warn!("ssh auth resolution failed (ssh will ask on the terminal): {e}");
                let fallback_kind = match server.auth_type {
                    warp_ssh_manager::AuthType::Password => SecretKind::Password,
                    warp_ssh_manager::AuthType::Key => SecretKind::Passphrase,
                    warp_ssh_manager::AuthType::OneKey => SecretKind::OneKeyPassword,
                };
                (server.clone(), node_id.clone(), fallback_kind)
            }
        };
    // The host and user name may come from another device through sync, so they are checked here
    // too, where the command line is built: a value starting with `-` would be read by `ssh` as
    // an option.
    if let Err(e) = warp_ssh_manager::validate_ssh_endpoint(
        &server_for_connection.host,
        &server_for_connection.username,
        server_for_connection.port,
    ) {
        log::warn!("ssh manager: not connecting to a server with an invalid endpoint: {e}");
        return;
    }
    let cmd = warp_ssh_manager::build_ssh_command_line(&server_for_connection);

    // A default CLI agent isn't launched on the remote host, nor replaced by agent view there.
    let ai_settings = AISettings::as_ref(ctx);
    if ai_settings.default_session_mode(ctx) == DefaultSessionMode::Agent
        && ai_settings.default_cli_agent(ctx).is_none()
    {
        terminal_view.update(ctx, |view, _| {
            view.set_enter_agent_view_after_ssh_bootstrap();
        });
    }

    // 1. 同步读 keychain(主线程 OK)。OneKey server 会使用共享凭据 id。
    // 2. 交给 askpass:ssh 自己要密码 / passphrase 时由 askpass 助手取,不再看终端输出。
    match (askpass, KeychainSecretStore.get(&secret_lookup_id, secret_kind)) {
        #[cfg(unix)]
        (Some(askpass), Ok(Some(secret))) if !secret.is_empty() => {
            askpass.serve(secret, warp_ssh_manager::askpass::SERVE_TIMEOUT);
        }
        (_, Err(e)) => log::warn!("ssh keychain read failed (ssh will ask on the terminal): {e}"),
        _ => {}
    }

    // 启动命令注入器 — 等待 shell ready 后自动执行 startup_command
    if let Some(startup_cmd) =
        warp_ssh_manager::build_startup_command_line(&server).filter(|_| run_startup_command)
    {
        crate::ssh_manager::startup_command_injector::spawn_startup_command_injector(
            terminal_view.read(ctx, |v, c| v.inactive_pty_reads_rx(c)),
            terminal_view.downgrade(),
            startup_cmd,
            ctx,
        );
    }

    // su 密码注入器 — 监听 su 密码提示,自动输入 root 密码
    let root_secret = match KeychainSecretStore.get(&node_id, SecretKind::RootPassword) {
        Ok(opt) => opt,
        Err(e) => {
            log::debug!("ssh root password keychain read failed: {e}");
            None
        }
    };
    if crate::ssh_manager::su_password_injector::should_spawn_su_password_injector(
        root_secret.as_ref(),
    ) {
        crate::ssh_manager::su_password_injector::spawn_su_password_injector(
            terminal_view.read(ctx, |v, c| v.inactive_pty_reads_rx(c)),
            terminal_view.downgrade(),
            root_secret,
            ctx,
        );
    }

    // 3. 排队 ssh 命令,等 bootstrap 完成自动 flush。
    terminal_view.update(ctx, |view, ctx| {
        view.execute_command_or_set_pending(&cmd, ctx);
        view.set_ssh_manager_connection(SshManagerConnection {
            node_id,
            server,
            command: cmd,
        });
    });
}
