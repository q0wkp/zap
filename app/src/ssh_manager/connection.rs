//! Connecting a terminal to a server saved in the SSH manager: typing its `ssh` command line
//! into the terminal's local shell, and answering the password prompts that follow with the
//! credentials saved in the keychain.

use warp_ssh_manager::{
    KeychainSecretStore, SecretKind, SshRepository, SshSecretStore, SshServerInfo,
};
use warpui::{SingletonEntity, View, ViewContext, ViewHandle};

use crate::settings::{AISettings, DefaultSessionMode};
use crate::terminal::TerminalView;

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
/// `ssh` command line, and starts the injectors that type the saved password, passphrase and
/// root password at their prompts. The injectors are owned by `ctx`'s view. The server's startup
/// command (and its zmx session) runs once the remote shell is ready if `run_startup_command`.
pub fn connect_terminal<O: View>(
    terminal_view: &ViewHandle<TerminalView>,
    node_id: String,
    server: SshServerInfo,
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
                log::warn!("ssh auth resolution failed (will continue without injection): {e}");
                let fallback_kind = match server.auth_type {
                    warp_ssh_manager::AuthType::Password => SecretKind::Password,
                    warp_ssh_manager::AuthType::Key => SecretKind::Passphrase,
                    warp_ssh_manager::AuthType::OneKey => SecretKind::OneKeyPassword,
                };
                (server.clone(), node_id.clone(), fallback_kind)
            }
        };
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
    let secret = match KeychainSecretStore.get(&secret_lookup_id, secret_kind) {
        Ok(opt) => opt.unwrap_or_else(|| zeroize::Zeroizing::new(String::new())),
        Err(e) => {
            log::warn!("ssh keychain read failed (will continue without injection): {e}");
            zeroize::Zeroizing::new(String::new())
        }
    };

    // 2. 注入器 spawn 必须在 execute_command 之前启动 — 否则 password prompt
    //    在 spawn 之前已经飞过 broadcast,injector 拿不到。
    let pty_reads_rx = terminal_view.read(ctx, |v, c| v.inactive_pty_reads_rx(c));
    crate::ssh_manager::secret_injector::spawn_password_injector(
        pty_reads_rx,
        terminal_view.downgrade(),
        secret,
        ctx,
    );

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
