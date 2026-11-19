//! SFTP 会话管理模块
//!
//! 封装 SSH2 连接建立、认证和 SFTP 子系统通道创建。
//! author: logic
//! date: 2026-05-31

use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use crate::error::SftpError;
use crate::known_hosts::{self, KnownHosts, Verdict};
use crate::sftp::Sftp;

/// 默认连接超时时间（10 秒）
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// 认证方式
#[derive(Debug, Clone)]
pub enum AuthMethod {
    Password { password: String },
    PublicKey { key_path: PathBuf, passphrase: Option<String> },
}

/// SFTP 会话，封装 ssh2 连接
pub struct SftpSession {
    session: Arc<ssh2::Session>,
    _tcp: TcpStream,
    /// 标记是否已显式断开连接，防止 Drop 双重 disconnect
    disconnected: Arc<AtomicBool>,
}

impl SftpSession {
    /// 通过指定参数建立 SSH 连接
    ///
    /// # 参数
    /// - `host`: 服务器地址
    /// - `port`: 服务器端口
    /// - `username`: 用户名
    /// - `auth`: 认证方式
    /// - `timeout`: 可选的超时时间，None 使用默认 10 秒
    pub fn connect(
        host: &str,
        port: u16,
        username: &str,
        auth: AuthMethod,
        timeout: Option<Duration>,
    ) -> Result<Self, SftpError> {
        let effective_timeout = timeout.unwrap_or(DEFAULT_TIMEOUT);
        let addr = format!("{host}:{port}");

        // 通过 ToSocketAddrs 进行 DNS 解析，支持主机名和 IP 地址
        let socket_addr = addr.to_socket_addrs()
            .map_err(|e| SftpError::ConnectionFailed(format!("Address resolution failed: {e}")))?
            .next()
            .ok_or_else(|| {
                SftpError::ConnectionFailed(format!("DNS resolution returned no results: {addr}"))
            })?;

        // 使用带超时的 TCP 连接
        let tcp = TcpStream::connect_timeout(&socket_addr, effective_timeout)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    SftpError::Timeout
                } else {
                    SftpError::ConnectionFailed(format!("Failed to connect to {addr}: {e}"))
                }
            })?;

        let mut session = ssh2::Session::new().map_err(|e| {
            SftpError::ConnectionFailed(format!("Failed to create SSH session: {e}"))
        })?;

        let tcp_for_session = tcp.try_clone()
            .map_err(|e| SftpError::ConnectionFailed(format!("Failed to clone TCP stream: {e}")))?;
        session.set_tcp_stream(tcp_for_session);

        // 设置 SSH 会话超时（毫秒），影响 handshake 和后续所有阻塞操作
        session.set_timeout(effective_timeout.as_millis() as u32);

        // 和 ssh 的 StrictHostKeyChecking=accept-new 一样校验 host key:先读
        // known_hosts,让服务器优先给出已记录类型的 key,握手后再比对。
        let host_entry = known_hosts::host_entry_name(host, port);
        let user_known_hosts = known_hosts::user_known_hosts_path();
        let known_host_files: Vec<&Path> = user_known_hosts
            .as_deref()
            .into_iter()
            .chain([Path::new(known_hosts::GLOBAL_KNOWN_HOSTS_PATH)])
            .collect();
        let known = KnownHosts::load(&known_host_files).map_err(|e| {
            SftpError::HostKeyRejected(format!("Failed to read known_hosts: {e}"))
        })?;
        // 设置失败(libssh2 一个都不认)时保留默认顺序,不影响连接。
        let _ = session.method_pref(
            ssh2::MethodType::HostKey,
            &known_hosts::host_key_method_prefs(&known.key_types_for(&host_entry)),
        );

        session.handshake()
            .map_err(|e| {
                if is_timeout_error(&e) {
                    SftpError::Timeout
                } else {
                    SftpError::ConnectionFailed(format!("SSH handshake failed: {e}"))
                }
            })?;

        verify_host_key(&session, &known, &host_entry, user_known_hosts.as_deref())?;

        match &auth {
            AuthMethod::Password { password } => {
                session.userauth_password(username, password)
                    .map_err(|e| {
                        if is_timeout_error(&e) {
                            SftpError::Timeout
                        } else {
                            SftpError::AuthFailed(format!("Password authentication failed: {e}"))
                        }
                    })?;
            }
            AuthMethod::PublicKey { key_path, passphrase } => {
                let pass = passphrase.as_deref();
                session.userauth_pubkey_file(username, None, key_path, pass)
                    .map_err(|e| {
                        if is_timeout_error(&e) {
                            SftpError::Timeout
                        } else {
                            SftpError::AuthFailed(format!("Key authentication failed: {e}"))
                        }
                    })?;
            }
        }

        if !session.authenticated() {
            return Err(SftpError::AuthFailed("Authentication was not accepted".into()));
        }

        // 设置操作超时（30秒），避免网络异常时操作无限阻塞
        session.set_timeout(30_000);

        Ok(Self {
            session: Arc::new(session),
            _tcp: tcp,
            disconnected: Arc::new(AtomicBool::new(false)),
        })
    }

    /// 获取 SFTP 通道
    pub fn sftp(&self) -> Result<Sftp, SftpError> {
        let sftp = self.session.sftp()?;
        Ok(Sftp::new(sftp))
    }

    /// 断开连接
    pub fn disconnect(&self) -> Result<(), SftpError> {
        if self.disconnected.swap(true, Ordering::SeqCst) {
            // 已经断开过，跳过
            return Ok(());
        }
        self.session.disconnect(None, "bye", None)?;
        Ok(())
    }

    /// 检查连接是否存活
    pub fn is_authenticated(&self) -> bool {
        self.session.authenticated()
    }
}

impl Drop for SftpSession {
    fn drop(&mut self) {
        if !self.disconnected.swap(true, Ordering::SeqCst) {
            let _ = self.session.disconnect(None, "bye", None);
        }
    }
}

/// Checks the key the server presented against known_hosts before anything secret is sent: a
/// changed or revoked key is refused, a host seen for the first time is recorded.
fn verify_host_key(
    session: &ssh2::Session,
    known: &KnownHosts,
    host_entry: &str,
    user_known_hosts: Option<&Path>,
) -> Result<(), SftpError> {
    let (key, key_type) = session
        .host_key()
        .ok_or_else(|| SftpError::HostKeyRejected("The server sent no host key".into()))?;
    let key_type = match key_type {
        ssh2::HostKeyType::Rsa => "ssh-rsa",
        ssh2::HostKeyType::Dss => "ssh-dss",
        ssh2::HostKeyType::Ecdsa256 => "ecdsa-sha2-nistp256",
        ssh2::HostKeyType::Ecdsa384 => "ecdsa-sha2-nistp384",
        ssh2::HostKeyType::Ecdsa521 => "ecdsa-sha2-nistp521",
        ssh2::HostKeyType::Ed25519 => "ssh-ed25519",
        ssh2::HostKeyType::Unknown => {
            return Err(SftpError::HostKeyRejected(format!(
                "{host_entry} sent a host key of an unknown type"
            )));
        }
    };
    let key = STANDARD.encode(key);
    match known.verify(host_entry, key_type, &key) {
        Verdict::Known => Ok(()),
        Verdict::Changed => Err(SftpError::HostKeyRejected(format!(
            "the {key_type} host key of {host_entry} differs from the one in known_hosts; \
             if the change is expected, remove the old entry (ssh-keygen -R '{host_entry}')"
        ))),
        Verdict::Revoked => Err(SftpError::HostKeyRejected(format!(
            "the {key_type} host key of {host_entry} is marked @revoked in known_hosts"
        ))),
        Verdict::Unknown => {
            let path = user_known_hosts.ok_or_else(|| {
                SftpError::HostKeyRejected(format!(
                    "{host_entry} is not in known_hosts, and there is no home directory to record it in"
                ))
            })?;
            known_hosts::append_entry(path, host_entry, key_type, &key).map_err(|e| {
                SftpError::HostKeyRejected(format!(
                    "Failed to add {host_entry} to {}: {e}",
                    path.display()
                ))
            })
        }
    }
}

/// 判断 ssh2 错误是否为超时错误
fn is_timeout_error(error: &ssh2::Error) -> bool {
    // ssh2 错误码 Session(-37) 对应 LIBSSH2_ERROR_SOCKET_TIMEOUT
    error.code() == ssh2::ErrorCode::Session(-37)
}
