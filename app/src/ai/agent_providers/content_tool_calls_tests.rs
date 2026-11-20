// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: rewrote the tests so a tool call written as text runs only when the whole reply is that one call.

use serde_json::json;

use super::extract_tool_calls_from_assistant_text;

fn offered(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn shell_tools() -> Vec<String> {
    offered(&["run_shell_command"])
}

#[test]
fn bare_call_object_is_extracted() {
    let text = r#"{"name":"run_shell_command","arguments":{"command":"echo hello"}}"#;
    let calls = extract_tool_calls_from_assistant_text(text, &shell_tools());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].fn_name, "run_shell_command");
    assert_eq!(calls[0].fn_arguments, json!({"command": "echo hello"}));
    assert!(!calls[0].call_id.is_empty());
}

#[test]
fn fenced_multiline_call_is_extracted() {
    let text = "```json\n{\n  \"name\": \"run_shell_command\",\n  \"arguments\": {\n    \"command\": \"echo hello\"\n  }\n}\n```\n";
    let calls = extract_tool_calls_from_assistant_text(text, &shell_tools());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].fn_arguments["command"], "echo hello");

    let unlabelled =
        "```\n{\"name\":\"run_shell_command\",\"arguments\":{\"command\":\"ls\"}}\n```";
    let calls = extract_tool_calls_from_assistant_text(unlabelled, &shell_tools());
    assert_eq!(calls.len(), 1);
}

#[test]
fn parameters_and_input_hold_the_arguments_too() {
    let text =
        r#"{"type":"function","name":"run_shell_command","parameters":{"command":"echo hello"}}"#;
    let calls = extract_tool_calls_from_assistant_text(text, &shell_tools());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].fn_arguments["command"], "echo hello");

    let text =
        r#"{"type":"tool_use","id":"toolu_1","name":"run_shell_command","input":{"command":"ls"}}"#;
    let calls = extract_tool_calls_from_assistant_text(text, &shell_tools());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].fn_arguments["command"], "ls");
}

#[test]
fn arguments_as_a_json_string_are_parsed() {
    let text = r#"{"name":"run_shell_command","arguments":"{\"command\":\"echo hello\"}"}"#;
    let calls = extract_tool_calls_from_assistant_text(text, &shell_tools());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].fn_arguments, json!({"command": "echo hello"}));
}

#[test]
fn openai_tool_calls_wrapper_yields_every_call_in_order() {
    let text = r#"{"tool_calls":[
        {"id":"call_1","type":"function","function":{"name":"webfetch","arguments":"{\"url\":\"https://example.com\"}"}},
        {"type":"function","function":{"name":"run_shell_command","arguments":{"command":"echo hello"}}}
    ]}"#;
    let calls =
        extract_tool_calls_from_assistant_text(text, &offered(&["run_shell_command", "webfetch"]));
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].fn_name, "webfetch");
    assert_eq!(calls[0].fn_arguments["url"], "https://example.com");
    assert_eq!(calls[1].fn_name, "run_shell_command");
    assert_ne!(calls[0].call_id, calls[1].call_id);
}

#[test]
fn several_objects_and_arrays_yield_every_call() {
    let text = r#"{"name":"webfetch","parameters":{"url":"https://example.com"}}
{"name":"run_shell_command","parameters":{"command":"echo hello"}}"#;
    let tools = offered(&["run_shell_command", "webfetch"]);
    let calls = extract_tool_calls_from_assistant_text(text, &tools);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].fn_name, "webfetch");
    assert_eq!(calls[1].fn_name, "run_shell_command");

    let text = r#"[{"name":"webfetch","arguments":{"url":"https://example.com"}},
        {"name":"run_shell_command","arguments":{"command":"ls"}}]"#;
    let calls = extract_tool_calls_from_assistant_text(text, &tools);
    assert_eq!(calls.len(), 2);
}

#[test]
fn tool_call_tags_are_extracted() {
    let text = "<tool_call>\n{\"name\": \"run_shell_command\", \"arguments\": {\"command\": \"ls\"}}\n</tool_call>\n<tool_call>\n{\"name\": \"run_shell_command\", \"arguments\": {\"command\": \"pwd\"}}\n</tool_call>";
    let calls = extract_tool_calls_from_assistant_text(text, &shell_tools());
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].fn_arguments["command"], "ls");
    assert_eq!(calls[1].fn_arguments["command"], "pwd");
}

#[test]
fn leading_think_block_is_skipped_and_never_searched() {
    let text = "<think>I could run {\"name\":\"run_shell_command\",\"arguments\":{\"command\":\"rm -rf /\"}}</think>\n{\"name\":\"run_shell_command\",\"arguments\":{\"command\":\"ls\"}}";
    let calls = extract_tool_calls_from_assistant_text(text, &shell_tools());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].fn_arguments["command"], "ls");

    let only_reasoning =
        "<think>{\"name\":\"run_shell_command\",\"arguments\":{\"command\":\"ls\"}}</think>";
    assert!(extract_tool_calls_from_assistant_text(only_reasoning, &shell_tools()).is_empty());

    let unterminated = "<think>{\"name\":\"run_shell_command\",\"arguments\":{\"command\":\"ls\"}}";
    assert!(extract_tool_calls_from_assistant_text(unterminated, &shell_tools()).is_empty());
}

#[test]
fn repeated_identical_call_is_returned_once() {
    let text = r#"{"name":"run_shell_command","arguments":{"command":"ls"}}
{"name":"run_shell_command","arguments":{"command":"ls"}}"#;
    let calls = extract_tool_calls_from_assistant_text(text, &shell_tools());
    assert_eq!(calls.len(), 1);
}

#[test]
fn prose_is_not_a_call() {
    let text = "I ran echo hello and the output was hello.";
    assert!(extract_tool_calls_from_assistant_text(text, &shell_tools()).is_empty());
    assert!(extract_tool_calls_from_assistant_text("", &shell_tools()).is_empty());
}

#[test]
fn call_inside_prose_is_not_executed() {
    let before = r#"To list the files I would call {"name":"run_shell_command","arguments":{"command":"ls"}}"#;
    assert!(extract_tool_calls_from_assistant_text(before, &shell_tools()).is_empty());

    let after = r#"{"name":"run_shell_command","arguments":{"command":"ls"}} is what such a call looks like."#;
    assert!(extract_tool_calls_from_assistant_text(after, &shell_tools()).is_empty());

    let fenced = "Here is an example:\n```json\n{\"name\":\"run_shell_command\",\"arguments\":{\"command\":\"ls\"}}\n```";
    assert!(extract_tool_calls_from_assistant_text(fenced, &shell_tools()).is_empty());

    let two_fences = "```json\n{\"name\":\"run_shell_command\",\"arguments\":{\"command\":\"ls\"}}\n```\n```json\n{\"name\":\"run_shell_command\",\"arguments\":{\"command\":\"pwd\"}}\n```";
    assert!(extract_tool_calls_from_assistant_text(two_fences, &shell_tools()).is_empty());

    let tagged = "<tool_call>{\"name\":\"run_shell_command\",\"arguments\":{\"command\":\"ls\"}}</tool_call> and then I will report.";
    assert!(extract_tool_calls_from_assistant_text(tagged, &shell_tools()).is_empty());
}

#[test]
fn fence_of_another_language_is_not_a_call() {
    let text = "```bash\n{\"name\":\"run_shell_command\",\"arguments\":{\"command\":\"ls\"}}\n```";
    assert!(extract_tool_calls_from_assistant_text(text, &shell_tools()).is_empty());
}

#[test]
fn tool_that_was_not_offered_is_not_called() {
    // A registered tool, but not offered in this request.
    let text = r#"{"name":"websearch","arguments":{"query":"test"}}"#;
    assert!(extract_tool_calls_from_assistant_text(text, &shell_tools()).is_empty());
}

#[test]
fn invented_tool_name_is_never_mapped_to_an_offered_tool() {
    for text in [
        r#"{"name":"totally_made_up_tool","parameters":{"command":"echo hi"}}"#,
        r#"{"name":"bash","arguments":{"command":"echo hi"}}"#,
        r#"{"name":"echo","arguments":{"message":"hello"}}"#,
        r#"{"name":"Run_Shell_Command","arguments":{"command":"echo hi"}}"#,
    ] {
        assert!(
            extract_tool_calls_from_assistant_text(text, &shell_tools()).is_empty(),
            "{text}"
        );
    }
}

#[test]
fn one_invalid_call_rejects_the_whole_response() {
    let text = r#"{"name":"run_shell_command","arguments":{"command":"ls"}}
{"name":"websearch","arguments":{"query":"x"}}"#;
    assert!(extract_tool_calls_from_assistant_text(text, &shell_tools()).is_empty());

    let wrapper =
        r#"{"tool_calls":[{"name":"run_shell_command","arguments":{"command":"ls"}},"ls"]}"#;
    assert!(extract_tool_calls_from_assistant_text(wrapper, &shell_tools()).is_empty());
}

#[test]
fn object_with_other_keys_is_not_a_call() {
    for text in [
        // Data that happens to have a `name`: a package manifest, a config entry.
        r#"{"name":"run_shell_command","version":"1.0.0","arguments":{"command":"ls"}}"#,
        r#"{"tool_calls":[{"name":"run_shell_command","arguments":{"command":"ls"}}],"note":"example"}"#,
        r#"{"type":"object","name":"run_shell_command","arguments":{"command":"ls"}}"#,
        r#"{"function":{"name":"run_shell_command","arguments":{"command":"ls"},"description":"Runs a command"}}"#,
    ] {
        assert!(
            extract_tool_calls_from_assistant_text(text, &shell_tools()).is_empty(),
            "{text}"
        );
    }
}

#[test]
fn arguments_must_be_one_json_object() {
    for text in [
        r#"{"name":"run_shell_command"}"#,
        r#"{"name":"run_shell_command","arguments":"ls"}"#,
        r#"{"name":"run_shell_command","arguments":["ls"]}"#,
        r#"{"name":"run_shell_command","arguments":null}"#,
        r#"{"name":"run_shell_command","arguments":{"command":"ls"},"parameters":{"command":"pwd"}}"#,
    ] {
        assert!(
            extract_tool_calls_from_assistant_text(text, &shell_tools()).is_empty(),
            "{text}"
        );
    }
}
