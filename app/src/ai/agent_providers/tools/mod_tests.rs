use serde_json::json;

use super::*;

#[test]
fn parses_the_invalid_arguments_payload() {
    let payload = json!({
        "error": "invalid_arguments",
        "detail": "missing field `operations`",
        "tool": "apply_file_diffs",
        "received_args": "{}",
        "hint": "Re-emit the tool call.",
    })
    .to_string();
    assert_eq!(
        parse_invalid_tool_call(&payload),
        Some(InvalidToolCall {
            tool_name: Some("apply_file_diffs".to_owned()),
            detail: Some("missing field `operations`".to_owned()),
        })
    );
}

#[test]
fn parses_the_todowrite_payload() {
    let payload = todowrite::invalid_arguments_result_to_json("bad todos".to_owned(), "{}");
    assert_eq!(
        parse_invalid_tool_call(&payload.to_string()),
        Some(InvalidToolCall {
            tool_name: Some(todowrite::TOOL_NAME.to_owned()),
            detail: Some("bad todos".to_owned()),
        })
    );
}

#[test]
fn missing_or_empty_fields_are_none() {
    assert_eq!(
        parse_invalid_tool_call(r#"{"error":"invalid_arguments","tool":" ","detail":""}"#),
        Some(InvalidToolCall {
            tool_name: None,
            detail: None,
        })
    );
}

#[test]
fn other_payloads_are_not_invalid_tool_calls() {
    for payload in [
        "",
        "not json",
        r#"{"status":"cancelled"}"#,
        r#"{"_byop_intercepted":true,"status":"ok","message":"no todos"}"#,
        r#"{"error":"something_else","tool":"grep"}"#,
    ] {
        assert_eq!(parse_invalid_tool_call(payload), None, "{payload}");
    }
}

fn file_edits_result(notes: Vec<String>) -> AIAgentActionResult {
    use crate::ai::agent::task::TaskId;
    use crate::ai::agent::{AIAgentActionResultType, RequestFileEditsResult};

    AIAgentActionResult {
        id: "tool_call".to_string().into(),
        task_id: TaskId::new("task".to_string()),
        result: AIAgentActionResultType::RequestFileEdits(RequestFileEditsResult::Success {
            diff: String::new(),
            updated_files: Vec::new(),
            deleted_files: vec!["gone.rs".to_string()],
            lines_added: 0,
            lines_removed: 0,
            notes,
        }),
    }
}

#[test]
fn edit_result_carries_match_notes() {
    let note = "a.rs: search block 1 matched line 2 only when leading whitespace is ignored";
    let json: serde_json::Value = serde_json::from_str(
        &serialize_action_result(&file_edits_result(vec![note.to_string()]))
            .expect("the edit result is serialized"),
    )
    .unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["notes"], json!([note]));
}

#[test]
fn edit_result_without_notes_has_no_notes_field() {
    let json: serde_json::Value = serde_json::from_str(
        &serialize_action_result(&file_edits_result(Vec::new()))
            .expect("the edit result is serialized"),
    )
    .unwrap();
    assert_eq!(json["status"], "ok");
    assert!(json.get("notes").is_none());
}
