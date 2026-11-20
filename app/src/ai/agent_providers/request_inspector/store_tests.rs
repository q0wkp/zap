// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;

use chrono::Local;

use super::*;
use crate::ai::agent_providers::request_inspector::{CallKind, CallOutcome, CaptureEvent};

fn request(id: u64, kind: CallKind, conversation: Option<&str>, body: &str) -> CaptureEvent {
    CaptureEvent::Started(Arc::new(CapturedRequest {
        id,
        started_at: Local::now(),
        kind,
        conversation: conversation.map(str::to_owned),
        model_id: "model".to_owned(),
        url: Some("https://example.test/v1/chat/completions".to_owned()),
        body: body.to_owned(),
    }))
}

fn text(call: u64, text: &str) -> CaptureEvent {
    CaptureEvent::Text {
        call,
        text: text.to_owned(),
    }
}

#[test]
fn a_streamed_response_is_assembled_in_place() {
    let mut store = CallStore::default();
    store.apply(request(1, CallKind::AgentTurn, Some("root"), "{}"));
    store.apply(CaptureEvent::Reasoning {
        call: 1,
        text: "thinking".to_owned(),
    });
    store.apply(text(1, "Hel"));
    store.apply(text(1, "lo"));
    for arguments in ["{\"comm", "{\"command\": \"ls\"}"] {
        store.apply(CaptureEvent::ToolCall {
            call: 1,
            call_id: "call-1".to_owned(),
            name: "run_shell_command".to_owned(),
            arguments: arguments.to_owned(),
        });
    }
    store.apply(CaptureEvent::Usage {
        call: 1,
        usage: "{\"prompt_tokens\": 10}".to_owned(),
    });

    let record = store.get(1).unwrap();
    assert_eq!(record.status, CallStatus::InProgress);
    assert_eq!(record.reasoning, "thinking");
    assert_eq!(record.text, "Hello");
    assert_eq!(
        record.tool_calls.len(),
        1,
        "a later chunk replaces the call"
    );
    assert_eq!(record.tool_calls[0].arguments, "{\"command\": \"ls\"}");

    store.apply(CaptureEvent::Finished {
        call: 1,
        at: Local::now(),
        outcome: CallOutcome::Failed("HTTP 500".to_owned()),
    });
    let record = store.get(1).unwrap();
    let response = response_text(record);
    for part in [
        "thinking",
        "Hello",
        "run_shell_command",
        "prompt_tokens",
        "HTTP 500",
    ] {
        assert!(response.contains(part), "{part} missing from {response}");
    }
    assert!(record_text(record).contains("POST https://example.test/v1/chat/completions\n{}"));
}

#[test]
fn events_of_calls_not_held_are_ignored() {
    let mut store = CallStore::default();
    assert!(!store.apply(text(7, "orphan")));
    assert!(store.is_empty());
}

#[test]
fn the_oldest_calls_go_first_when_over_the_count() {
    let mut store = CallStore::with_limits(3, usize::MAX);
    for id in 1..=5 {
        store.apply(request(id, CallKind::Title, None, "{}"));
    }
    let ids: Vec<u64> = store.calls().map(|call| call.request.id).collect();
    assert_eq!(ids, vec![3, 4, 5]);
}

#[test]
fn the_oldest_calls_go_first_when_over_the_bytes() {
    let mut store = CallStore::with_limits(usize::MAX, 100);
    store.apply(request(1, CallKind::AgentTurn, None, &"a".repeat(60)));
    store.apply(request(2, CallKind::AgentTurn, None, &"b".repeat(30)));
    assert_eq!(store.len(), 2);
    // The response streaming into call 2 pushes the total over the limit.
    store.apply(text(2, &"c".repeat(20)));
    let ids: Vec<u64> = store.calls().map(|call| call.request.id).collect();
    assert_eq!(ids, vec![2]);
    // The newest call stays even when it alone is over the limit.
    store.apply(text(2, &"d".repeat(200)));
    assert_eq!(store.len(), 1);
}

#[test]
fn a_runaway_response_is_cut_off() {
    let mut store = CallStore::default();
    store.apply(request(1, CallKind::AgentTurn, None, "{}"));
    let chunk = "x".repeat(MAX_RESPONSE_BYTES / 2);
    for _ in 0..3 {
        store.apply(text(1, &chunk));
    }
    let record = store.get(1).unwrap();
    assert_eq!(record.text.len(), 2 * chunk.len());
    assert!(record.response_truncated);
}

#[test]
fn the_filter_selects_by_group_and_conversation() {
    let mut store = CallStore::default();
    store.apply(request(1, CallKind::AgentTurn, Some("conv-a"), "{}"));
    store.apply(request(2, CallKind::Title, Some("conv-a"), "{}"));
    store.apply(request(3, CallKind::AgentTurn, Some("conv-b"), "{}"));
    store.apply(request(4, CallKind::NextCommand, None, "{}"));
    assert_eq!(store.conversations(), vec!["conv-a", "conv-b"]);

    let shown = |filter: &CallFilter| -> Vec<u64> {
        store
            .calls()
            .filter(|call| filter.matches(&call.request))
            .map(|call| call.request.id)
            .collect()
    };

    let mut filter = CallFilter::default();
    assert_eq!(shown(&filter), vec![1, 2, 3, 4]);

    filter.toggle_group(CallGroup::ActiveAi);
    assert_eq!(shown(&filter), vec![1, 2, 3]);

    filter.conversation = Some("conv-a".to_owned());
    assert_eq!(shown(&filter), vec![1, 2]);

    filter.toggle_group(CallGroup::Titles);
    filter.toggle_group(CallGroup::ActiveAi);
    assert_eq!(shown(&filter), vec![1]);
}

#[test]
fn short_ids_keep_eight_characters() {
    assert_eq!(short_id("0123456789abcdef"), "01234567");
    assert_eq!(short_id("abc"), "abc");
}
