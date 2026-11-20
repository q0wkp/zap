// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

//! Operations that change a node in the database together with its secrets in the keychain.
//!
//! The two stores can't share a transaction, so each operation here does the database part
//! inside a transaction that is only committed once the keychain part has succeeded, and puts
//! the keychain back as it was when either part fails. A node is then never left without its
//! secrets, nor secrets without their node.

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;
use thiserror::Error;
use zeroize::Zeroizing;

use crate::repository::{SshRepository, SshRepositoryError};
use crate::secrets::{SecretKind, SshSecretStore, SshSecretStoreError};
use crate::types::{NodeKind, SshNode, SshServerInfo};

/// The secrets stored under a server node's own id. A OneKey credential's secret is stored
/// under the credential's id and outlives the servers that use it.
const NODE_SECRET_KINDS: [SecretKind; 3] = [
    SecretKind::Password,
    SecretKind::Passphrase,
    SecretKind::RootPassword,
];

/// The secrets a copy of a server takes over from the original.
const CLONED_SECRET_KINDS: [SecretKind; 2] = [SecretKind::Password, SecretKind::Passphrase];

#[derive(Debug, Error)]
pub enum NodeSecretsError {
    #[error(transparent)]
    Repository(#[from] SshRepositoryError),
    #[error(transparent)]
    Secrets(#[from] SshSecretStoreError),
}

impl From<DieselError> for NodeSecretsError {
    fn from(error: DieselError) -> Self {
        Self::Repository(error.into())
    }
}

/// Deletes a node and everything below it, with the keychain secrets of every server among
/// them. If a secret can't be deleted, or the database refuses, nothing is deleted: the
/// database is rolled back and the secrets already removed are stored again.
pub fn delete_node_with_secrets(
    conn: &mut SqliteConnection,
    store: &dyn SshSecretStore,
    node_id: &str,
) -> Result<(), NodeSecretsError> {
    let mut removed: Vec<(String, SecretKind, Zeroizing<String>)> = Vec::new();
    let result = conn.transaction::<_, NodeSecretsError, _>(|conn| {
        let nodes = SshRepository::list_nodes(conn)?;
        let server_ids = server_ids_in_subtree(&nodes, node_id);
        SshRepository::delete_node(conn, node_id)?;
        for server_id in &server_ids {
            for kind in NODE_SECRET_KINDS {
                let Some(secret) = read_secret(store, server_id, kind)? else {
                    continue;
                };
                store.delete(server_id, kind)?;
                removed.push((server_id.clone(), kind, secret));
            }
        }
        Ok(())
    });
    if result.is_err() {
        for (server_id, kind, secret) in &removed {
            if let Err(e) = store.set(server_id, *kind, secret) {
                log::error!("ssh manager: could not restore a secret of node {server_id}: {e}");
            }
        }
    }
    result
}

/// Creates a copy of the server `source_id` named `name` under `parent_id`, with the
/// original's password and passphrase. If a secret can't be copied, no copy is made: the new
/// node is rolled back and the secrets already written for it are deleted.
pub fn clone_server_with_secrets(
    conn: &mut SqliteConnection,
    store: &dyn SshSecretStore,
    source_id: &str,
    parent_id: Option<&str>,
    name: &str,
) -> Result<SshNode, NodeSecretsError> {
    let mut written: Vec<(String, SecretKind)> = Vec::new();
    let result = conn.transaction::<_, NodeSecretsError, _>(|conn| {
        let source = SshRepository::get_server(conn, source_id)?
            .ok_or_else(|| SshRepositoryError::NotFound(source_id.to_string()))?;
        let info = SshServerInfo::clone_from_template(&source, String::new());
        let node = SshRepository::create_server(conn, parent_id, name, &info)?;
        for kind in CLONED_SECRET_KINDS {
            let Some(secret) = read_secret(store, source_id, kind)? else {
                continue;
            };
            // Recorded before writing: a write that fails may still have stored something.
            written.push((node.id.clone(), kind));
            store.set(&node.id, kind, &secret)?;
        }
        Ok(node)
    });
    if result.is_err() {
        for (node_id, kind) in &written {
            if let Err(e) = store.delete(node_id, *kind) {
                log::error!("ssh manager: could not remove a secret of node {node_id}: {e}");
            }
        }
    }
    result
}

/// Reads a secret, treating a system without a keychain as one that holds none: there is
/// nothing to copy or delete, and the node itself must stay manageable.
fn read_secret(
    store: &dyn SshSecretStore,
    node_id: &str,
    kind: SecretKind,
) -> Result<Option<Zeroizing<String>>, SshSecretStoreError> {
    match store.get(node_id, kind) {
        Ok(secret) => Ok(secret),
        Err(SshSecretStoreError::NoBackend) => Ok(None),
        Err(e @ SshSecretStoreError::Keyring(_)) => Err(e),
    }
}

/// The ids of the servers at or below `root_id`.
fn server_ids_in_subtree(nodes: &[SshNode], root_id: &str) -> Vec<String> {
    let mut server_ids = Vec::new();
    let mut pending = vec![root_id];
    while let Some(id) = pending.pop() {
        for node in nodes {
            if node.id == id && node.kind == NodeKind::Server {
                server_ids.push(node.id.clone());
            }
            if node.parent_id.as_deref() == Some(id) {
                pending.push(node.id.as_str());
            }
        }
    }
    server_ids
}

#[cfg(test)]
#[path = "node_secrets_tests.rs"]
mod tests;
