use super::*;
use crate::chat::ChatOptions;
use serde_json::json;

/// A streamer whose event source is never polled: the tests feed it
/// `delta.tool_calls` payloads directly.
fn test_streamer(options: &ChatOptions) -> OpenAIStreamer {
	let request = reqwest::Client::new().get("http://127.0.0.1:1");
	OpenAIStreamer::new(
		EventSourceStream::new(request),
		ModelIden::new(AdapterKind::OpenAI, "test-model"),
		ChatOptionsSet::default().with_chat_options(Some(options)),
	)
}

fn queued_tool_calls(streamer: &mut OpenAIStreamer) -> Vec<ToolCall> {
	streamer
		.pending_events
		.drain(..)
		.map(|event| match event {
			InterStreamEvent::ToolCallChunk(tool_call) => tool_call,
			other => panic!("expected a tool call chunk, got {other:?}"),
		})
		.collect()
}

#[test]
fn every_tool_call_of_one_message_is_emitted_and_captured() {
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let mut streamer = test_streamer(&options);

	streamer.queue_delta_tool_calls(&json!([
		{"index": 0, "id": "call_a", "type": "function", "function": {"name": "read_files", "arguments": "{\"path\":\"a.rs\"}"}},
		{"index": 1, "id": "call_b", "type": "function", "function": {"name": "grep", "arguments": "{\"query\":\"main\"}"}}
	]));

	let emitted = queued_tool_calls(&mut streamer);
	assert_eq!(emitted.len(), 2);
	assert_eq!(emitted[0].call_id, "call_a");
	assert_eq!(emitted[0].fn_name, "read_files");
	assert_eq!(emitted[1].call_id, "call_b");
	assert_eq!(emitted[1].fn_name, "grep");

	let captured = streamer.captured_data.tool_calls.expect("captured tool calls");
	assert_eq!(captured.len(), 2);
	assert_eq!(captured[1].call_id, "call_b");
	assert_eq!(captured[1].fn_arguments, Value::String("{\"query\":\"main\"}".to_string()));
}

#[test]
fn fragments_of_two_tool_calls_in_one_message_reach_both_calls() {
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let mut streamer = test_streamer(&options);

	streamer.queue_delta_tool_calls(&json!([
		{"index": 0, "id": "call_a", "function": {"name": "read_files", "arguments": "{\"path\":"}},
		{"index": 1, "id": "call_b", "function": {"name": "grep", "arguments": "{\"query\":"}}
	]));
	streamer.queue_delta_tool_calls(&json!([
		{"index": 0, "function": {"arguments": "\"a.rs\"}"}},
		{"index": 1, "function": {"arguments": "\"main\"}"}}
	]));

	let emitted = queued_tool_calls(&mut streamer);
	assert_eq!(emitted.len(), 4);
	assert_eq!(emitted[3].call_id, "call_b");
	assert_eq!(emitted[3].fn_arguments, Value::String("{\"query\":\"main\"}".to_string()));

	let captured = streamer.captured_data.tool_calls.expect("captured tool calls");
	assert_eq!(captured[0].fn_arguments, Value::String("{\"path\":\"a.rs\"}".to_string()));
	assert_eq!(captured[1].fn_arguments, Value::String("{\"query\":\"main\"}".to_string()));
}

#[test]
fn tool_call_keeps_its_id_when_later_fragments_repeat_only_the_name() {
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let mut streamer = test_streamer(&options);

	streamer.queue_delta_tool_calls(&json!([
		{"index": 0, "id": "call_abc", "function": {"name": "read_files", "arguments": "{\"path\":"}}
	]));
	streamer.queue_delta_tool_calls(&json!([
		{"index": 0, "function": {"name": "read_files", "arguments": "\"a.rs\"}"}}
	]));

	let emitted = queued_tool_calls(&mut streamer);
	assert_eq!(emitted.len(), 2);
	assert_eq!(emitted[0].call_id, "call_abc");
	assert_eq!(emitted[1].call_id, "call_abc");
	assert_eq!(emitted[1].fn_arguments, Value::String("{\"path\":\"a.rs\"}".to_string()));

	let captured = streamer.captured_data.tool_calls.expect("captured tool calls");
	assert_eq!(captured.len(), 1);
	assert_eq!(captured[0].call_id, "call_abc");
}

#[test]
fn tool_call_keeps_the_id_it_was_first_emitted_with() {
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let mut streamer = test_streamer(&options);

	streamer.queue_delta_tool_calls(&json!([
		{"index": 0, "function": {"name": "read_files", "arguments": ""}}
	]));
	streamer.queue_delta_tool_calls(&json!([
		{"index": 0, "id": "call_abc", "function": {"arguments": "{}"}}
	]));

	let emitted = queued_tool_calls(&mut streamer);
	assert_eq!(emitted[0].call_id, "call_0");
	assert_eq!(emitted[1].call_id, "call_0");
	assert_eq!(emitted[1].fn_name, "read_files");
	assert_eq!(emitted[1].fn_arguments, Value::String("{}".to_string()));
}

#[test]
fn tool_call_without_any_id_is_identified_by_its_index() {
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let mut streamer = test_streamer(&options);

	streamer.queue_delta_tool_calls(&json!([
		{"index": 0, "function": {"name": "read_files", "arguments": "{}"}},
		{"index": 1, "id": "", "function": {"name": "grep", "arguments": "{}"}}
	]));

	let emitted = queued_tool_calls(&mut streamer);
	assert_eq!(emitted[0].call_id, "call_0");
	assert_eq!(emitted[1].call_id, "call_1");
}

#[test]
fn tool_call_indexes_that_skip_ahead_capture_no_filler_calls() {
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let mut streamer = test_streamer(&options);

	streamer.queue_delta_tool_calls(&json!([
		{"index": 1, "id": "call_a", "function": {"name": "read_files", "arguments": "{\"path\":"}}
	]));
	streamer.queue_delta_tool_calls(&json!([
		{"index": 3, "id": "call_b", "function": {"name": "grep", "arguments": "{}"}}
	]));
	streamer.queue_delta_tool_calls(&json!([
		{"index": 1, "function": {"arguments": "\"a.rs\"}"}}
	]));

	let captured = streamer.captured_data.tool_calls.take().expect("captured tool calls");
	assert_eq!(captured.len(), 2);
	assert_eq!(captured[0].call_id, "call_a");
	assert_eq!(captured[0].fn_arguments, Value::String("{\"path\":\"a.rs\"}".to_string()));
	assert_eq!(captured[1].call_id, "call_b");
}

#[test]
fn huge_tool_call_index_does_not_size_the_captured_calls() {
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let mut streamer = test_streamer(&options);

	streamer.queue_delta_tool_calls(&json!([
		{"index": u32::MAX, "id": "call_a", "function": {"name": "read_files", "arguments": "{}"}}
	]));

	let captured = streamer.captured_data.tool_calls.take().expect("captured tool calls");
	assert_eq!(captured.len(), 1);
	assert_eq!(captured[0].call_id, "call_a");
}

#[test]
fn captured_arguments_of_a_call_without_any_become_an_empty_object() {
	assert_eq!(parse_captured_arguments(Value::String(String::new())), json!({}));
	assert_eq!(parse_captured_arguments(Value::String(" \n".to_string())), json!({}));
}

#[test]
fn captured_arguments_are_parsed_or_kept_as_the_raw_string() {
	assert_eq!(
		parse_captured_arguments(Value::String("{\"path\":\"a.rs\"}".to_string())),
		json!({"path": "a.rs"})
	);
	assert_eq!(
		parse_captured_arguments(Value::String("{\"path\":\"a.r".to_string())),
		Value::String("{\"path\":\"a.r".to_string())
	);
	assert_eq!(parse_captured_arguments(json!({"path": "a.rs"})), json!({"path": "a.rs"}));
}
