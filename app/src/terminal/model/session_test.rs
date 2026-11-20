// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: added a test that a local SSH command follows the session tree; renamed the Warp and Zap names to ZapNG.

use std::collections::HashMap;

use zap_ng_ui::{
    elements::Empty, platform::WindowStyle, App, AppContext, Element, Entity, ModelHandle,
    TypedActionView, View, ViewContext,
};

use super::{BootstrapSessionType, SessionId, SessionInfo, Sessions, SessionsEvent};
use crate::terminal::model::terminal_model::SubshellInitializationInfo;
use crate::terminal::ssh::util::parse_interactive_ssh_command;

struct TestView {
    events: Vec<SessionsEvent>,
}

impl Entity for TestView {
    type Event = usize;
}

impl View for TestView {
    fn render<'a>(&self, _: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }

    fn ui_name() -> &'static str {
        "TestView"
    }
}

impl TypedActionView for TestView {
    type Action = ();
}

impl TestView {
    fn new(model: ModelHandle<Sessions>, ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(&model, |me, _, event, _| {
            me.events.push(event.to_owned());
        });
        Self { events: Vec::new() }
    }
}

#[test]
fn test_set_env_var_emits_event() {
    App::test((), |mut app| async move {
        let model_handle = app.add_model(|_| Sessions::new_for_test());
        let session_id: SessionId = 0.into();
        let (_, view_handle) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            TestView::new(model_handle.clone(), ctx)
        });
        view_handle.read(&app, |view, _ctx| {
            assert!(view.events.is_empty());
        });
        model_handle.update(&mut app, |sessions, ctx| {
            let new_vars = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
            sessions.set_env_vars_for_session(session_id, new_vars, ctx)
        });

        view_handle.read(&app, |view, _ctx| {
            assert_eq!(view.events.len(), 1);
            let expected_session_id = session_id;
            let event = view.events.first().expect("checked length already");
            if let SessionsEvent::EnvironmentVariablesUpdated { session_id } = event {
                assert_eq!(*session_id, expected_session_id);
            } else {
                assert!(matches!(
                    event,
                    SessionsEvent::EnvironmentVariablesUpdated { .. }
                ));
            }
        });
    });
}

#[test]
fn test_set_env_var_emits_no_event_when_no_change() {
    App::test((), |mut app| async move {
        let model_handle = app.add_model(|_| Sessions::new_for_test());
        let session_id: SessionId = 0.into();
        let (_, view_handle) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            TestView::new(model_handle.clone(), ctx)
        });
        view_handle.read(&app, |view, _ctx| {
            assert!(view.events.is_empty());
        });
        model_handle.update(&mut app, |sessions, ctx| {
            let new_vars = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
            sessions.set_env_vars_for_session(session_id, new_vars, ctx)
        });

        view_handle.read(&app, |view, _ctx| {
            assert_eq!(view.events.len(), 1);
        });

        model_handle.update(&mut app, |sessions, ctx| {
            let new_vars = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
            sessions.set_env_vars_for_session(session_id, new_vars, ctx)
        });

        view_handle.read(&app, |view, _ctx| {
            assert_eq!(view.events.len(), 1);
        });
    });
}

#[test]
fn local_ssh_command_follows_the_session_tree() {
    let subshell_info = |command: &str| SubshellInitializationInfo {
        spawning_command: command.to_owned(),
        was_triggered_by_rc_file_snippet: false,
        env_var_collection_name: None,
        ssh_connection_info: parse_interactive_ssh_command(command),
    };
    let remote = |id: u64, spawning_session_id: u64| SessionInfo {
        session_type: BootstrapSessionType::ZapNGifiedRemote,
        spawning_session_id: Some(SessionId::from(spawning_session_id)),
        ..SessionInfo::new_for_test().with_id(id)
    };

    let mut sessions = Sessions::new_for_test();
    sessions.register_session_for_test(SessionInfo::new_for_test().with_id(1));
    // `ssh` run through the SSH wrapper in the local session.
    sessions.register_session_for_test(SessionInfo {
        ssh_wrapper_command: Some("ssh -p 2222 me@a".to_owned()),
        ..remote(2, 1)
    });
    // A subshell inside it, e.g. `sudo -s`.
    sessions.register_session_for_test(SessionInfo {
        subshell_info: Some(subshell_info("sudo -s")),
        ..remote(3, 2)
    });
    // A nested `ssh` from the remote host.
    sessions.register_session_for_test(SessionInfo {
        subshell_info: Some(subshell_info("ssh b")),
        ..remote(4, 3)
    });
    // `ssh` warpified as a subshell from the local session.
    sessions.register_session_for_test(SessionInfo {
        subshell_info: Some(subshell_info("ssh c")),
        ..remote(5, 1)
    });
    // A local subshell.
    sessions.register_session_for_test(SessionInfo {
        subshell_info: Some(subshell_info("bash")),
        spawning_session_id: Some(SessionId::from(1)),
        ..SessionInfo::new_for_test().with_id(6)
    });

    let command = |id: u64| sessions.local_ssh_command(SessionId::from(id));
    assert_eq!(command(1), None);
    assert_eq!(command(2).as_deref(), Some("ssh -p 2222 me@a"));
    assert_eq!(command(3).as_deref(), Some("ssh -p 2222 me@a"));
    assert_eq!(command(4), None);
    assert_eq!(command(5).as_deref(), Some("ssh c"));
    assert_eq!(command(6), None);
    assert_eq!(command(7), None);
}
