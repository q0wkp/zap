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
