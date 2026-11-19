/// resolve_test_password 单元测试
/// author: logic
/// date: 2026/06/01
use super::*;
use pathfinder_geometry::vector::vec2f;
use std::collections::HashMap;
use std::sync::Mutex;
use warp_core::ui::appearance::Appearance;
use warpui::platform::WindowStyle;
use warpui::{App, WindowId, WindowInvalidation};

use crate::settings_view::keybindings::KeybindingChangedNotifier;
use crate::test_util::settings::initialize_settings_for_tests;
use crate::view_components::dropdown::DropdownAction;

/// 进程内 mock,绕开 OS keychain。支持错误注入,模拟 NoBackend / Keyring 错。
struct MockSecretStore {
    inner: Mutex<HashMap<String, String>>,
    get_err: Mutex<Option<SshSecretStoreError>>,
}

impl MockSecretStore {
    fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            get_err: Mutex::new(None),
        }
    }

    fn with_secret(node: &str, kind: SecretKind, value: &str) -> Self {
        let s = Self::new();
        s.set(node, kind, value).unwrap();
        s
    }

    fn inject_get_error(&self, err: SshSecretStoreError) {
        *self.get_err.lock().unwrap() = Some(err);
    }
}

fn account_key(node_id: &str, kind: SecretKind) -> String {
    let suffix = match kind {
        SecretKind::Password => "password",
        SecretKind::Passphrase => "passphrase",
        SecretKind::RootPassword => "root_password",
        SecretKind::OneKeyPassword => "onekey_password",
    };
    format!("{node_id}:{suffix}")
}

impl SshSecretStore for MockSecretStore {
    fn set(
        &self,
        node_id: &str,
        kind: SecretKind,
        secret: &str,
    ) -> Result<(), SshSecretStoreError> {
        self.inner
            .lock()
            .unwrap()
            .insert(account_key(node_id, kind), secret.to_string());
        Ok(())
    }

    fn get(
        &self,
        node_id: &str,
        kind: SecretKind,
    ) -> Result<Option<Zeroizing<String>>, SshSecretStoreError> {
        if let Some(err) = self.get_err.lock().unwrap().take() {
            return Err(err);
        }
        Ok(self
            .inner
            .lock()
            .unwrap()
            .get(&account_key(node_id, kind))
            .cloned()
            .map(Zeroizing::new))
    }

    fn delete(&self, _node_id: &str, _kind: SecretKind) -> Result<(), SshSecretStoreError> {
        unimplemented!()
    }
}

#[test]
fn auth_toggle_includes_onekey_option() {
    crate::i18n::init(Some("en"));

    let options = auth_toggle_options();
    assert_eq!(
        options,
        [AuthType::Password, AuthType::Key, AuthType::OneKey]
    );
    assert_eq!(auth_toggle_label(AuthType::OneKey), "OneKey");
    assert_eq!(
        auth_toggle_action(AuthType::OneKey),
        SshServerAction::SetAuthOneKey
    );
}

#[test]
fn onekey_auth_only_renders_credential_field_in_server_form() {
    assert_eq!(
        auth_specific_fields(AuthType::OneKey),
        vec![AuthSpecificField::OneKeyCredential]
    );
}

#[test]
fn empty_editor_empty_store_returns_none() {
    let store = MockSecretStore::new();
    assert!(resolve_test_password(Some("n1"), SecretKind::Password, "", &store).is_none());
}

#[test]
fn empty_editor_stored_returns_secret() {
    let store = MockSecretStore::with_secret("n1", SecretKind::Password, "from-keychain");
    let pw = resolve_test_password(Some("n1"), SecretKind::Password, "", &store).unwrap();
    assert_eq!(&*pw, "from-keychain");
}

#[test]
fn filled_editor_ignores_keychain() {
    // keychain 存了旧密码,form 敲了新密码 → 必须用 form 的新密码,
    // 否则用户改 host 后测试会被旧密码污染。
    let store = MockSecretStore::with_secret("n1", SecretKind::Password, "old-pw");
    let pw = resolve_test_password(Some("n1"), SecretKind::Password, "new-pw", &store).unwrap();
    assert_eq!(&*pw, "new-pw");
}

#[test]
fn empty_editor_no_backend_returns_none() {
    let store = MockSecretStore::new();
    store.inject_get_error(SshSecretStoreError::NoBackend);
    assert!(resolve_test_password(Some("n1"), SecretKind::Password, "", &store).is_none());
}

#[test]
fn empty_editor_keyring_error_returns_none() {
    let store = MockSecretStore::new();
    store.inject_get_error(SshSecretStoreError::Keyring("locked".into()));
    assert!(resolve_test_password(Some("n1"), SecretKind::Password, "", &store).is_none());
}

#[test]
fn onekey_lookup_uses_shared_credential_id_and_kind() {
    let store = MockSecretStore::with_secret("cred-1", SecretKind::OneKeyPassword, "shared-pw");
    let pw = resolve_test_password(Some("cred-1"), SecretKind::OneKeyPassword, "", &store).unwrap();
    assert_eq!(&*pw, "shared-pw");
}

fn credential(
    id: &str,
    username: &str,
    kind: OneKeyCredentialKind,
    key_path: Option<&str>,
) -> SshOneKeyCredential {
    let now = chrono::Utc::now().naive_utc();
    SshOneKeyCredential {
        id: id.to_string(),
        label: "shared".to_string(),
        username: username.to_string(),
        kind,
        key_path: key_path.map(ToString::to_string),
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn onekey_test_connection_uses_shared_password_credential() {
    let store = MockSecretStore::with_secret("cred-1", SecretKind::OneKeyPassword, "shared-pw");
    let credentials = vec![credential(
        "cred-1",
        "shared-user",
        OneKeyCredentialKind::Password,
        None,
    )];
    let server = SshServerInfo {
        node_id: "server-1".to_string(),
        host: "example.com".to_string(),
        port: 22,
        username: "draft-user".to_string(),
        auth_type: AuthType::OneKey,
        key_path: None,
        credential_id: Some("cred-1".to_string()),
        startup_command: None,
        zmx_session: None,
        notes: None,
        last_connected_at: None,
    };

    let (server, pw) = resolve_test_server_and_password(server, &credentials, "", &store).unwrap();

    assert_eq!(server.username, "shared-user");
    assert_eq!(server.auth_type, AuthType::Password);
    assert_eq!(server.key_path, None);
    assert_eq!(&*pw.unwrap(), "shared-pw");
}

#[test]
fn onekey_test_connection_prefers_editor_password() {
    let store = MockSecretStore::with_secret("cred-1", SecretKind::OneKeyPassword, "old-pw");
    let credentials = vec![credential(
        "cred-1",
        "shared-user",
        OneKeyCredentialKind::Password,
        None,
    )];
    let server = SshServerInfo {
        node_id: "server-1".to_string(),
        host: "example.com".to_string(),
        port: 22,
        username: "draft-user".to_string(),
        auth_type: AuthType::OneKey,
        key_path: None,
        credential_id: Some("cred-1".to_string()),
        startup_command: None,
        zmx_session: None,
        notes: None,
        last_connected_at: None,
    };

    let (_, pw) =
        resolve_test_server_and_password(server, &credentials, "typed-pw", &store).unwrap();

    assert_eq!(&*pw.unwrap(), "typed-pw");
}

#[test]
fn onekey_key_credential_resolves_test_connection_to_key_auth() {
    let store = MockSecretStore::with_secret("cred-1", SecretKind::Passphrase, "key-passphrase");
    let credentials = vec![credential(
        "cred-1",
        "key-user",
        OneKeyCredentialKind::Key,
        Some("/home/me/.ssh/id_ed25519"),
    )];
    let server = SshServerInfo {
        node_id: "server-1".to_string(),
        host: "example.com".to_string(),
        port: 22,
        username: "draft-user".to_string(),
        auth_type: AuthType::OneKey,
        key_path: None,
        credential_id: Some("cred-1".to_string()),
        startup_command: None,
        zmx_session: None,
        notes: None,
        last_connected_at: None,
    };

    let (server, pw) = resolve_test_server_and_password(server, &credentials, "", &store).unwrap();

    assert_eq!(server.username, "key-user");
    assert_eq!(server.auth_type, AuthType::Key);
    assert_eq!(server.key_path.as_deref(), Some("/home/me/.ssh/id_ed25519"));
    assert_eq!(&*pw.unwrap(), "key-passphrase");
}

#[test]
fn missing_lookup_id_returns_none_when_editor_empty() {
    let store = MockSecretStore::new();
    assert!(resolve_test_password(None, SecretKind::OneKeyPassword, "", &store).is_none());
}

#[test]
fn selecting_onekey_dropdown_item_does_not_rebuild_dropdown_while_it_is_borrowed() {
    App::test((), |mut app| async move {
        crate::i18n::init(Some("en"));
        initialize_settings_for_tests(&mut app);
        app.add_singleton_model(|_| Appearance::mock());
        app.add_singleton_model(|_| SshTreeChangedNotifier::new());

        let (window_id, view) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut view = SshServerView::new("server-1".to_string(), ctx);
            view.node = Some(SshNode {
                id: "server-1".to_string(),
                parent_id: None,
                kind: NodeKind::Server,
                name: "server".to_string(),
                sort_order: 0,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
                is_collapsed: false,
            });
            view.auth_type = AuthType::OneKey;
            view.onekey_credentials = vec![credential(
                "cred-1",
                "shared-user",
                OneKeyCredentialKind::Password,
                None,
            )];
            view.rebuild_onekey_credential_dropdown(ctx);
            view
        });
        let presenter = app.presenter(window_id).unwrap();
        let mut updated = std::collections::HashSet::new();
        updated.insert(app.root_view_id(window_id).unwrap());
        app.update(|ctx| {
            let mut presenter = presenter.borrow_mut();
            presenter.invalidate(
                WindowInvalidation {
                    updated,
                    ..Default::default()
                },
                ctx,
            );
            presenter.build_scene(vec2f(640., 480.), 1., None, ctx);
        });

        let dropdown = view.read(&app, |view, _| view.onekey_credential_dropdown.clone());
        dropdown.update(&mut app, |dropdown, ctx| {
            dropdown.handle_action(
                &DropdownAction::SelectActionAndClose(SshServerAction::SelectOneKeyCredential(
                    Some(0),
                )),
                ctx,
            );
        });

        view.read(&app, |view, _| {
            assert_eq!(
                view.selected_onekey_credential_id.as_deref(),
                Some("cred-1")
            );
        });
    });
}

fn zmx_session(name: &str, preview: Option<&str>) -> ZmxSession {
    ZmxSession {
        name: name.to_string(),
        clients: Some(1),
        cwd: Some("/srv/app".to_string()),
        command: None,
        foreground: None,
        preview: preview.map(str::to_string),
    }
}

/// 把窗口完整地布局并绘制一遍,渲染代码里的 panic 会在这里暴露。
fn build_scene(app: &mut App, window_id: WindowId) {
    let presenter = app.presenter(window_id).unwrap();
    let mut updated = std::collections::HashSet::new();
    updated.insert(app.root_view_id(window_id).unwrap());
    app.update(|ctx| {
        let mut presenter = presenter.borrow_mut();
        presenter.invalidate(
            WindowInvalidation {
                updated,
                ..Default::default()
            },
            ctx,
        );
        presenter.build_scene(vec2f(640., 480.), 1., None, ctx);
    });
}

/// 开一个窗口,里面是一个 server 节点的编辑表单。
fn open_server_view(app: &mut App) -> (WindowId, ViewHandle<SshServerView>) {
    crate::i18n::init(Some("en"));
    initialize_settings_for_tests(app);
    app.add_singleton_model(|_| Appearance::mock());
    app.add_singleton_model(|_| SshTreeChangedNotifier::new());
    // 表单里的 editor 构造时要用它。
    app.add_singleton_model(|_| KeybindingChangedNotifier::mock());
    // 表单一构造就读 SSH 管理器的库:没有路径会 panic,给一个临时文件即可,
    // 里面没有表,读不到节点只记一条日志。进程内只有第一次设置生效。
    warp_ssh_manager::set_database_path(
        std::env::temp_dir().join("warp_ssh_server_view_test.sqlite"),
    );

    app.add_window(WindowStyle::NotStealFocus, |ctx| {
        let mut view = SshServerView::new("server-1".to_string(), ctx);
        view.node = Some(SshNode {
            id: "server-1".to_string(),
            parent_id: None,
            kind: NodeKind::Server,
            name: "server".to_string(),
            sort_order: 0,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            is_collapsed: false,
        });
        view
    })
}

#[test]
fn selecting_a_listed_zmx_session_fills_the_session_field() {
    App::test((), |mut app| async move {
        let (window_id, view) = open_server_view(&mut app);
        view.update(&mut app, |view, ctx| {
            view.zmx_session_row_states =
                vec![MouseStateHandle::default(), MouseStateHandle::default()];
            view.zmx_session_list = ZmxSessionList::Loaded(vec![
                zmx_session("dev", None),
                zmx_session("build", Some("$ make\n\nok")),
            ]);
            ctx.notify();
        });
        // 还没选中任何 session:只画列表。
        build_scene(&mut app, window_id);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SshServerAction::SelectZmxSession(1), ctx);
        });
        // 选中之后多画一块预览。
        build_scene(&mut app, window_id);

        view.read(&app, |view, app| {
            assert_eq!(view.current_text(&view.zmx_session_editor, app), "build");
        });
    });
}

#[test]
fn zmx_session_states_other_than_a_list_render() {
    App::test((), |mut app| async move {
        let (window_id, view) = open_server_view(&mut app);

        for list in [
            ZmxSessionList::Loading,
            ZmxSessionList::Failed("Permission denied (publickey).".to_string()),
            ZmxSessionList::Loaded(Vec::new()),
        ] {
            view.update(&mut app, |view, ctx| {
                view.zmx_session_list = list;
                ctx.notify();
            });
            build_scene(&mut app, window_id);
        }
    });
}

#[test]
fn zmx_session_subtitle_skips_what_the_session_does_not_have() {
    // 客户端数那一项是翻译过的文案,这里只看拼接,所以不带它。
    let mut session = zmx_session("dev", None);
    session.clients = None;
    session.command = Some("vim".to_string());
    assert_eq!(zmx_session_subtitle(&session), "/srv/app · vim");

    session.foreground = Some("claude".to_string());
    assert_eq!(zmx_session_subtitle(&session), "claude · /srv/app · vim");
    session.foreground = None;

    session.cwd = None;
    assert_eq!(zmx_session_subtitle(&session), "vim");

    session.command = None;
    assert_eq!(zmx_session_subtitle(&session), "");
}

#[test]
fn form_endpoint_defaults_an_empty_port_to_22() {
    assert_eq!(
        checked_form_endpoint("example.com", "root", AuthType::Password, "  "),
        Ok(22)
    );
    assert_eq!(
        checked_form_endpoint("example.com", "root", AuthType::Password, " 2222 "),
        Ok(2222)
    );
}

#[test]
fn form_endpoint_rejects_ports_out_of_range() {
    for port in ["0", "65536", "-1", "ssh"] {
        assert_eq!(
            checked_form_endpoint("example.com", "root", AuthType::Password, port),
            Err(SshEndpointError::InvalidPort),
            "{port}"
        );
    }
}

#[test]
fn form_endpoint_rejects_a_host_or_user_that_reads_as_an_option() {
    assert_eq!(
        checked_form_endpoint("-oProxyCommand=id", "root", AuthType::Key, "22"),
        Err(SshEndpointError::InvalidHost)
    );
    assert_eq!(
        checked_form_endpoint("example.com", "-l root", AuthType::Key, "22"),
        Err(SshEndpointError::InvalidUsername)
    );
    assert_eq!(
        checked_form_endpoint("", "root", AuthType::Key, "22"),
        Err(SshEndpointError::EmptyHost)
    );
}

#[test]
fn form_endpoint_ignores_the_hidden_user_field_of_a_onekey_server() {
    assert_eq!(
        checked_form_endpoint("example.com", "-stale", AuthType::OneKey, "22"),
        Ok(22)
    );
}

fn pending_host_key(host: &str, port: u16) -> PendingHostKey {
    PendingHostKey {
        host: host.to_string(),
        port,
        key: UnknownHostKey {
            key_type: "ED25519".to_string(),
            fingerprint: "SHA256:jq1oZMhXkPYmtQ5uuWcWN8rAyW0mLAU8Yp0J0y6t9cE".to_string(),
        },
        retry: HostKeyRetry::TestConnection,
    }
}

#[test]
fn confirmed_host_key_is_trusted_for_the_server_it_was_offered_by() {
    let mut server = SshServerInfo::new_default("n".to_string());
    server.host = "example.com".to_string();
    let pending = pending_host_key("example.com", 22);
    assert_eq!(
        trusted_fingerprint_for(Some(&pending), &server),
        Some(pending.key.fingerprint.clone())
    );
    assert_eq!(trusted_fingerprint_for(None, &server), None);
}

/// The form may have been edited while the question was on screen: a key confirmed for one
/// server is never passed on as trusted for another.
#[test]
fn confirmed_host_key_is_not_trusted_for_another_server() {
    let mut server = SshServerInfo::new_default("n".to_string());
    server.host = "other.example.com".to_string();
    let pending = pending_host_key("example.com", 22);
    assert_eq!(trusted_fingerprint_for(Some(&pending), &server), None);

    server.host = "example.com".to_string();
    server.port = 2222;
    assert_eq!(trusted_fingerprint_for(Some(&pending), &server), None);
}

#[test]
fn unknown_host_key_question_renders_and_goes_away_when_rejected() {
    App::test((), |mut app| async move {
        let (window_id, view) = open_server_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.pending_host_key = Some(pending_host_key("example.com", 2222));
            ctx.notify();
        });
        build_scene(&mut app, window_id);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SshServerAction::RejectHostKey, ctx);
            assert_eq!(view.pending_host_key, None);
        });
        build_scene(&mut app, window_id);
    });
}
