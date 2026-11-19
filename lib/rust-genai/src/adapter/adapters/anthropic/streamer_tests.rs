use super::*;
use crate::adapter::AdapterKind;
use crate::chat::ChatOptions;

/// A streamer whose event source is never polled: the tests feed it SSE
/// message payloads directly.
fn test_streamer(options: &ChatOptions) -> AnthropicStreamer {
	let request = reqwest::Client::new().get("http://127.0.0.1:1");
	AnthropicStreamer::new(
		EventSourceStream::new(request),
		ModelIden::new(AdapterKind::Anthropic, "test-model"),
		ChatOptionsSet::default().with_chat_options(Some(options)),
	)
}

#[test]
fn deltas_of_an_unhandled_block_are_skipped() {
	let options = ChatOptions::default().with_capture_content(true);
	let mut streamer = test_streamer(&options);

	let start = streamer
		.on_content_block_start(
			r#"{"type":"content_block_start","index":0,"content_block":{"type":"server_tool_use","id":"srvtoolu_1","name":"web_search","input":{}}}"#,
		)
		.expect("an unhandled block type must not fail the stream");
	assert!(start.is_none());

	let delta = streamer
		.on_content_block_delta(
			r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"query\":\"rust\"}"}}"#,
		)
		.expect("a delta of an unhandled block must not fail the stream");
	assert!(delta.is_none());
	assert!(streamer.captured_data.content.is_none());
	assert!(streamer.captured_data.tool_calls.is_none());
}

#[test]
fn text_after_an_unhandled_block_is_still_streamed() {
	let options = ChatOptions::default().with_capture_content(true);
	let mut streamer = test_streamer(&options);

	streamer
		.on_content_block_start(
			r#"{"type":"content_block_start","index":0,"content_block":{"type":"redacted_thinking","data":"abc"}}"#,
		)
		.expect("start of the unhandled block");
	streamer
		.on_content_block_start(r#"{"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#)
		.expect("start of the text block");
	let delta = streamer
		.on_content_block_delta(
			r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"hello"}}"#,
		)
		.expect("text delta");

	assert!(matches!(delta, Some(InterStreamEvent::Chunk(text)) if text == "hello"));
	assert_eq!(streamer.captured_data.content.as_deref(), Some("hello"));
}

#[test]
fn citations_delta_in_a_text_block_is_skipped() {
	let options = ChatOptions::default().with_capture_content(true);
	let mut streamer = test_streamer(&options);

	streamer
		.on_content_block_start(r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#)
		.expect("start of the text block");
	let delta = streamer
		.on_content_block_delta(
			r#"{"type":"content_block_delta","index":0,"delta":{"type":"citations_delta","citation":{"type":"char_location","cited_text":"x"}}}"#,
		)
		.expect("a delta without text must not fail the stream");

	assert!(delta.is_none());
	assert!(streamer.captured_data.content.is_none());
}

#[test]
fn tool_use_deltas_accumulate_the_arguments() {
	let options = ChatOptions::default();
	let mut streamer = test_streamer(&options);

	let start = streamer
		.on_content_block_start(
			r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"read_files","input":{}}}"#,
		)
		.expect("start of the tool_use block");
	assert!(matches!(start, Some(InterStreamEvent::ToolCallChunk(call)) if call.call_id == "toolu_1"));

	streamer
		.on_content_block_delta(
			r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":"}}"#,
		)
		.expect("first argument delta");
	let delta = streamer
		.on_content_block_delta(
			r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"a.rs\"}"}}"#,
		)
		.expect("second argument delta");

	let Some(InterStreamEvent::ToolCallChunk(call)) = delta else {
		panic!("expected a tool call chunk");
	};
	assert_eq!(call.fn_name, "read_files");
	assert_eq!(call.fn_arguments, Value::String(r#"{"path":"a.rs"}"#.to_string()));
}

#[test]
fn message_delta_usage_replaces_the_counts_of_message_start() {
	let options = ChatOptions::default().with_capture_usage(true);
	let mut streamer = test_streamer(&options);

	streamer
		.capture_usage(
			"message_start",
			r#"{"type":"message_start","message":{"usage":{"input_tokens":25,"cache_creation_input_tokens":100,"cache_read_input_tokens":2000,"output_tokens":3}}}"#,
		)
		.expect("usage of message_start");
	streamer
		.capture_usage(
			"message_delta",
			r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"input_tokens":25,"cache_creation_input_tokens":100,"cache_read_input_tokens":2000,"output_tokens":40}}"#,
		)
		.expect("usage of message_delta");

	let usage = streamer.captured_data.usage.expect("captured usage");
	assert_eq!(usage.prompt_tokens, Some(2125));
	assert_eq!(usage.completion_tokens, Some(40));
	let details = usage.prompt_tokens_details.expect("cache details");
	assert_eq!(details.cache_creation_tokens, Some(100));
	assert_eq!(details.cached_tokens, Some(2000));
}

#[test]
fn message_delta_with_output_tokens_only_keeps_the_input_tokens() {
	let options = ChatOptions::default().with_capture_usage(true);
	let mut streamer = test_streamer(&options);

	streamer
		.capture_usage(
			"message_start",
			r#"{"type":"message_start","message":{"usage":{"input_tokens":25,"output_tokens":1}}}"#,
		)
		.expect("usage of message_start");
	streamer
		.capture_usage("message_delta", r#"{"type":"message_delta","delta":{},"usage":{"output_tokens":15}}"#)
		.expect("usage of message_delta");

	let usage = streamer.captured_data.usage.expect("captured usage");
	assert_eq!(usage.prompt_tokens, Some(25));
	assert_eq!(usage.completion_tokens, Some(15));
	assert!(usage.prompt_tokens_details.is_none());
}

#[test]
fn input_tokens_reported_only_in_message_delta_are_captured() {
	let options = ChatOptions::default().with_capture_usage(true);
	let mut streamer = test_streamer(&options);

	streamer
		.capture_usage(
			"message_start",
			r#"{"type":"message_start","message":{"usage":{"input_tokens":0,"output_tokens":0}}}"#,
		)
		.expect("usage of message_start");
	streamer
		.capture_usage(
			"message_delta",
			r#"{"type":"message_delta","delta":{},"usage":{"input_tokens":310,"output_tokens":12}}"#,
		)
		.expect("usage of message_delta");

	let usage = streamer.captured_data.usage.expect("captured usage");
	assert_eq!(usage.prompt_tokens, Some(310));
	assert_eq!(usage.completion_tokens, Some(12));
}
