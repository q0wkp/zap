//! SSH 管理器数据层 — 持久化的服务器 / 文件夹树 + OS keychain 凭据存储 +
//! 命令拼装。UI 与 PTY 注入逻辑放在 `app/src/ssh_manager/` 与 `secret_injector`
//! 模块,这里保持纯 Rust、无 zap_ng_ui 依赖、可单独 `cargo test` 跑。

pub mod db;
pub mod node_secrets;
pub mod repository;
pub mod secrets;
pub mod ssh_command;
pub mod ssh_config_parser;
pub mod sync_provider;
pub mod types;
pub mod zmx_sessions;

pub use db::{set_database_path, with_conn};
pub use node_secrets::{NodeSecretsError, clone_server_with_secrets, delete_node_with_secrets};
pub use repository::{SshRepository, SshRepositoryError, SyncMetaRepository};
pub use secrets::{KeychainSecretStore, SecretKind, SshSecretStore, SshSecretStoreError};
pub use ssh_command::{
    ConnectionTestResult, SshEndpointError, UnknownHostKey, build_ssh_args, build_ssh_command_line,
    build_startup_command_line, test_connection, validate_ssh_endpoint,
};
pub use ssh_config_parser::{
    LoadOutcome, LoadResult, SshConfigCandidate, default_ssh_config_path, load_candidates,
    load_candidates_from, parse_ssh_config,
};
pub use sync_provider::{
    DbVersionStore, SshSyncData, SshSyncProvider, SyncNode, SyncOneKeyCredential, SyncServer,
};
pub use types::ConnectionStatus;
pub use types::{
    AuthType, NodeKind, OneKeyCredentialKind, ResolvedSshAuth, SshNode, SshOneKeyCredential,
    SshServerInfo,
};
pub use zmx_sessions::{ZmxSession, ZmxSessionsError, list_zmx_sessions};
