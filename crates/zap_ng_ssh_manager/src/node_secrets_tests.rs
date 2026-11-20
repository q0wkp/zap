use diesel::sqlite::SqliteConnection;
use zeroize::Zeroizing;

use super::*;
use crate::repository::setup_in_memory;
use crate::secrets::test_support::InMemorySecretStore;

/// An in-memory secret store that can be told to fail.
#[derive(Default)]
struct TestStore {
    inner: InMemorySecretStore,
    /// Deleting a secret of this kind fails.
    failing_delete: Option<SecretKind>,
    /// Storing a secret of this kind fails.
    failing_set: Option<SecretKind>,
    /// Every call fails as on a system without a keychain.
    no_backend: bool,
}

impl TestStore {
    fn failure() -> SshSecretStoreError {
        SshSecretStoreError::Keyring("keychain is locked".to_string())
    }

    fn secret(&self, node_id: &str, kind: SecretKind) -> Option<String> {
        self.inner
            .get(node_id, kind)
            .unwrap()
            .map(|secret| secret.to_string())
    }
}

impl SshSecretStore for TestStore {
    fn set(
        &self,
        node_id: &str,
        kind: SecretKind,
        secret: &str,
    ) -> Result<(), SshSecretStoreError> {
        if self.no_backend {
            return Err(SshSecretStoreError::NoBackend);
        }
        if self.failing_set == Some(kind) {
            return Err(Self::failure());
        }
        self.inner.set(node_id, kind, secret)
    }

    fn get(
        &self,
        node_id: &str,
        kind: SecretKind,
    ) -> Result<Option<Zeroizing<String>>, SshSecretStoreError> {
        if self.no_backend {
            return Err(SshSecretStoreError::NoBackend);
        }
        self.inner.get(node_id, kind)
    }

    fn delete(&self, node_id: &str, kind: SecretKind) -> Result<(), SshSecretStoreError> {
        if self.no_backend {
            return Err(SshSecretStoreError::NoBackend);
        }
        if self.failing_delete == Some(kind) {
            return Err(Self::failure());
        }
        self.inner.delete(node_id, kind)
    }
}

fn create_server(conn: &mut SqliteConnection, parent_id: Option<&str>, name: &str) -> SshNode {
    let mut info = SshServerInfo::new_default(String::new());
    info.host = format!("{name}.example.com");
    info.username = "root".to_string();
    SshRepository::create_server(conn, parent_id, name, &info).unwrap()
}

fn node_ids(conn: &mut SqliteConnection) -> Vec<String> {
    let mut ids: Vec<String> = SshRepository::list_nodes(conn)
        .unwrap()
        .into_iter()
        .map(|node| node.id)
        .collect();
    ids.sort();
    ids
}

#[test]
fn delete_removes_a_server_and_its_secrets() {
    let mut conn = setup_in_memory();
    let store = TestStore::default();
    let server = create_server(&mut conn, None, "a");
    for kind in NODE_SECRET_KINDS {
        store.inner.set(&server.id, kind, "secret").unwrap();
    }

    delete_node_with_secrets(&mut conn, &store, &server.id).unwrap();

    assert!(node_ids(&mut conn).is_empty());
    for kind in NODE_SECRET_KINDS {
        assert_eq!(store.secret(&server.id, kind), None, "{kind:?}");
    }
}

/// The database deletes the servers inside a folder with it; their secrets must go too.
#[test]
fn delete_of_a_folder_removes_the_secrets_of_every_server_inside() {
    let mut conn = setup_in_memory();
    let store = TestStore::default();
    let folder = SshRepository::create_folder(&mut conn, None, "F").unwrap();
    let inner_folder = SshRepository::create_folder(&mut conn, Some(&folder.id), "G").unwrap();
    let direct = create_server(&mut conn, Some(&folder.id), "direct");
    let nested = create_server(&mut conn, Some(&inner_folder.id), "nested");
    let outside = create_server(&mut conn, None, "outside");
    for server in [&direct, &nested, &outside] {
        store
            .inner
            .set(&server.id, SecretKind::Password, "pw")
            .unwrap();
    }

    delete_node_with_secrets(&mut conn, &store, &folder.id).unwrap();

    assert_eq!(node_ids(&mut conn), vec![outside.id.clone()]);
    assert_eq!(store.secret(&direct.id, SecretKind::Password), None);
    assert_eq!(store.secret(&nested.id, SecretKind::Password), None);
    assert_eq!(
        store.secret(&outside.id, SecretKind::Password).as_deref(),
        Some("pw")
    );
}

#[test]
fn delete_keeps_the_node_and_its_secrets_when_a_secret_cannot_be_deleted() {
    let mut conn = setup_in_memory();
    let store = TestStore {
        failing_delete: Some(SecretKind::Passphrase),
        ..Default::default()
    };
    let server = create_server(&mut conn, None, "a");
    store
        .inner
        .set(&server.id, SecretKind::Password, "pw")
        .unwrap();
    store
        .inner
        .set(&server.id, SecretKind::Passphrase, "pp")
        .unwrap();

    let error = delete_node_with_secrets(&mut conn, &store, &server.id).unwrap_err();

    assert!(matches!(error, NodeSecretsError::Secrets(_)), "{error:?}");
    assert_eq!(node_ids(&mut conn), vec![server.id.clone()]);
    assert!(
        SshRepository::get_server(&mut conn, &server.id)
            .unwrap()
            .is_some()
    );
    // The password was deleted before the passphrase failed, and is back.
    assert_eq!(
        store.secret(&server.id, SecretKind::Password).as_deref(),
        Some("pw")
    );
    assert_eq!(
        store.secret(&server.id, SecretKind::Passphrase).as_deref(),
        Some("pp")
    );
}

#[test]
fn delete_of_a_missing_node_fails_without_touching_secrets() {
    let mut conn = setup_in_memory();
    let store = TestStore::default();
    let server = create_server(&mut conn, None, "a");
    store
        .inner
        .set(&server.id, SecretKind::Password, "pw")
        .unwrap();

    let error = delete_node_with_secrets(&mut conn, &store, "nope").unwrap_err();

    assert!(
        matches!(
            error,
            NodeSecretsError::Repository(SshRepositoryError::NotFound(_))
        ),
        "{error:?}"
    );
    assert_eq!(
        store.secret(&server.id, SecretKind::Password).as_deref(),
        Some("pw")
    );
}

/// Without a keychain there are no secrets, and a node can still be deleted.
#[test]
fn delete_works_on_a_system_without_a_keychain() {
    let mut conn = setup_in_memory();
    let store = TestStore {
        no_backend: true,
        ..Default::default()
    };
    let server = create_server(&mut conn, None, "a");

    delete_node_with_secrets(&mut conn, &store, &server.id).unwrap();

    assert!(node_ids(&mut conn).is_empty());
}

#[test]
fn clone_copies_the_server_with_its_password_and_passphrase() {
    let mut conn = setup_in_memory();
    let store = TestStore::default();
    let folder = SshRepository::create_folder(&mut conn, None, "F").unwrap();
    let source = create_server(&mut conn, Some(&folder.id), "a");
    store
        .inner
        .set(&source.id, SecretKind::Password, "pw")
        .unwrap();
    store
        .inner
        .set(&source.id, SecretKind::Passphrase, "pp")
        .unwrap();

    let copy =
        clone_server_with_secrets(&mut conn, &store, &source.id, Some(&folder.id), "a (copy)")
            .unwrap();

    assert_ne!(copy.id, source.id);
    assert_eq!(copy.name, "a (copy)");
    assert_eq!(copy.parent_id.as_deref(), Some(folder.id.as_str()));
    let info = SshRepository::get_server(&mut conn, &copy.id)
        .unwrap()
        .unwrap();
    assert_eq!(info.host, "a.example.com");
    assert_eq!(
        store.secret(&copy.id, SecretKind::Password).as_deref(),
        Some("pw")
    );
    assert_eq!(
        store.secret(&copy.id, SecretKind::Passphrase).as_deref(),
        Some("pp")
    );
    // The original keeps its own.
    assert_eq!(
        store.secret(&source.id, SecretKind::Password).as_deref(),
        Some("pw")
    );
}

#[test]
fn clone_leaves_nothing_behind_when_a_secret_cannot_be_copied() {
    let mut conn = setup_in_memory();
    let store = TestStore {
        failing_set: Some(SecretKind::Passphrase),
        ..Default::default()
    };
    let source = create_server(&mut conn, None, "a");
    store
        .inner
        .set(&source.id, SecretKind::Password, "pw")
        .unwrap();
    store
        .inner
        .set(&source.id, SecretKind::Passphrase, "pp")
        .unwrap();
    let before = store.inner.count();

    let error =
        clone_server_with_secrets(&mut conn, &store, &source.id, None, "a (copy)").unwrap_err();

    assert!(matches!(error, NodeSecretsError::Secrets(_)), "{error:?}");
    // No copy in the database, and the password already written for it is gone again.
    assert_eq!(node_ids(&mut conn), vec![source.id.clone()]);
    assert_eq!(store.inner.count(), before);
}

#[test]
fn clone_of_a_missing_server_creates_nothing() {
    let mut conn = setup_in_memory();
    let store = TestStore::default();

    let error = clone_server_with_secrets(&mut conn, &store, "nope", None, "copy").unwrap_err();

    assert!(
        matches!(
            error,
            NodeSecretsError::Repository(SshRepositoryError::NotFound(_))
        ),
        "{error:?}"
    );
    assert!(node_ids(&mut conn).is_empty());
}

/// Without a keychain there is nothing to copy, and a server can still be cloned.
#[test]
fn clone_works_on_a_system_without_a_keychain() {
    let mut conn = setup_in_memory();
    let store = TestStore {
        no_backend: true,
        ..Default::default()
    };
    let source = create_server(&mut conn, None, "a");

    let copy = clone_server_with_secrets(&mut conn, &store, &source.id, None, "a (copy)").unwrap();

    assert_eq!(node_ids(&mut conn).len(), 2);
    assert_eq!(copy.name, "a (copy)");
}
