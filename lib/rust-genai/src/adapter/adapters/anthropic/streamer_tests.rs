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

/// Serves `body` once as an event stream on a local port and returns a streamer reading it.
fn streamer_of_response(options: &ChatOptions, body: &'static str) -> AnthropicStreamer {
	use std::io::{Read, Write};

	let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind a local port");
	let address = listener.local_addr().expect("local address");
	std::thread::spawn(move || {
		let (mut socket, _) = listener.accept().expect("accept the request");
		// The request has no body: it ends with the empty line after its headers.
		let mut request = Vec::new();
		let mut buffer = [0; 1024];
		while !request.windows(4).any(|window| window == b"\r\n\r\n") {
			match socket.read(&mut buffer) {
				Ok(0) | Err(_) => break,
				Ok(read) => request.extend_from_slice(&buffer[..read]),
			}
		}
		let response = format!(
			"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
			body.len()
		);
		// The test may have stopped reading by now.
		let _ = socket.write_all(response.as_bytes());
	});

	let client = reqwest::Client::builder().no_proxy().build().expect("http client");
	AnthropicStreamer::new(
		EventSourceStream::new(client.get(format!("http://{address}/"))),
		ModelIden::new(AdapterKind::Anthropic, "test-model"),
		ChatOptionsSet::default().with_chat_options(Some(options)),
	)
}

#[test]
fn frame_without_an_event_name_takes_its_type_from_the_data() {
	assert_eq!(frame_type("message", r#"{"type":"content_block_delta","index":0}"#), "content_block_delta");
	// A named frame is dispatched by its name, whatever its data says.
	assert_eq!(frame_type("ping", r#"{"type":"message_stop"}"#), "ping");
	// Data without a type, or that is not JSON, leaves the frame unknown.
	assert_eq!(frame_type("message", r#"{"index":0}"#), "message");
	assert_eq!(frame_type("message", "[DONE]"), "message");
}

#[test]
fn error_message_becomes_an_error_with_the_error_object_as_body() {
	let options = ChatOptions::default();
	let streamer = test_streamer(&options);

	let error = streamer
		.stream_error(r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#)
		.expect("the error message parses");

	match error {
		Error::ChatResponse { body, .. } => {
			assert_eq!(body["type"], "overloaded_error");
			assert_eq!(body["message"], "Overloaded");
		}
		other => panic!("unexpected error variant: {other:?}"),
	}
}

#[test]
fn tool_call_with_unparseable_arguments_keeps_them_as_a_string() {
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let mut streamer = test_streamer(&options);

	streamer
		.on_content_block_start(
			r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"read_files","input":{}}}"#,
		)
		.expect("start of the tool_use block");
	streamer
		.on_content_block_delta(
			r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"a.r"}}"#,
		)
		.expect("argument delta");
	streamer.on_content_block_stop();

	let captured = streamer.captured_data.tool_calls.expect("captured tool calls");
	assert_eq!(captured.len(), 1);
	assert_eq!(captured[0].call_id, "toolu_1");
	assert_eq!(captured[0].fn_arguments, Value::String(r#"{"path":"a.r"#.to_string()));
}

#[test]
fn tool_call_arguments_are_parsed_and_empty_ones_become_an_object() {
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let mut streamer = test_streamer(&options);

	streamer
		.on_content_block_start(
			r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"read_files","input":{}}}"#,
		)
		.expect("start of the first tool_use block");
	streamer
		.on_content_block_delta(
			r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"a.rs\"}"}}"#,
		)
		.expect("argument delta");
	streamer.on_content_block_stop();
	streamer
		.on_content_block_start(
			r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_2","name":"list_tabs","input":{}}}"#,
		)
		.expect("start of the second tool_use block");
	streamer.on_content_block_stop();

	let captured = streamer.captured_data.tool_calls.expect("captured tool calls");
	assert_eq!(captured[0].fn_arguments, serde_json::json!({"path": "a.rs"}));
	assert_eq!(captured[1].fn_arguments, serde_json::json!({}));
}

#[tokio::test]
async fn stream_of_data_only_frames_is_read_like_a_named_one() {
	use futures::StreamExt;

	// What a proxy that drops the `event:` lines forwards.
	let body = concat!(
		"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":7,\"output_tokens\":1}}}\n\n",
		"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
		"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n",
		"data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
		"data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"grep\",\"input\":{}}}\n\n",
		"data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"query\\\":\"}}\n\n",
		"data: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
		"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"max_tokens\"},\"usage\":{\"output_tokens\":9}}\n\n",
		"data: {\"type\":\"message_stop\"}\n\n",
	);
	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);
	let mut streamer = streamer_of_response(&options, body);

	let mut text = String::new();
	let mut end = None;
	while let Some(event) = streamer.next().await {
		match event.expect("the cut-off tool call must not fail the stream") {
			InterStreamEvent::Chunk(chunk) => text.push_str(&chunk),
			InterStreamEvent::End(stream_end) => end = Some(stream_end),
			InterStreamEvent::Start
			| InterStreamEvent::ReasoningChunk(_)
			| InterStreamEvent::ThoughtSignatureChunk(_)
			| InterStreamEvent::ToolCallChunk(_) => {}
		}
	}

	assert_eq!(text, "hello");
	let end = end.expect("the stream ends with message_stop");
	assert_eq!(end.captured_text_content.as_deref(), Some("hello"));
	assert_eq!(end.captured_usage.expect("captured usage").total_tokens, Some(16));
	assert!(matches!(end.captured_stop_reason, Some(StopReason::MaxTokens(_))));
	let tool_calls = end.captured_tool_calls.expect("captured tool calls");
	assert_eq!(tool_calls[0].fn_name, "grep");
	assert_eq!(tool_calls[0].fn_arguments, Value::String(r#"{"query":"#.to_string()));
}

#[tokio::test]
async fn error_event_ends_the_stream_with_an_error() {
	use futures::StreamExt;

	let body = concat!(
		"event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":7}}}\n\n",
		"event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n",
	);
	let options = ChatOptions::default();
	let mut streamer = streamer_of_response(&options, body);

	let mut error = None;
	while let Some(event) = streamer.next().await {
		if let Err(stream_error) = event {
			error = Some(stream_error);
		}
	}

	match error.expect("the error event must reach the caller") {
		Error::ChatResponse { body, .. } => assert_eq!(body["type"], "overloaded_error"),
		other => panic!("unexpected error variant: {other:?}"),
	}
}
