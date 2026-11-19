use super::*;

/// Feeds `chunks` to the event stream splitter as the network would deliver them and
/// returns the events it finds, followed by what is left over at the end of the stream.
fn events_of_chunks(chunks: &[&str]) -> (Vec<String>, Option<String>) {
	let mut partial_message = None;
	let mut events = Vec::new();

	for chunk in chunks {
		let BuffResponse {
			first_message,
			next_messages,
			candidate_message,
		} = process_buff_string_event_stream(chunk.to_string(), &mut partial_message).expect("the chunk is split");
		events.extend(first_message);
		events.extend(next_messages.into_iter().flatten());
		partial_message = candidate_message;
	}

	(events, partial_message.filter(|partial| !partial.is_empty()))
}

#[test]
fn events_separated_by_line_feeds_are_split() {
	let (events, rest) = events_of_chunks(&["event: a\ndata: 1\n\ndata: 2\n\ndata: 3"]);

	assert_eq!(events, ["event: a\ndata: 1", "data: 2"]);
	assert_eq!(rest.as_deref(), Some("data: 3"));
}

#[test]
fn events_separated_by_crlf_are_split_as_they_arrive() {
	let (events, rest) = events_of_chunks(&["event: a\r\ndata: 1\r\n\r\ndata: 2\r\n\r\n"]);

	assert_eq!(events, ["event: a\ndata: 1", "data: 2"]);
	assert_eq!(rest, None);
}

#[test]
fn crlf_separator_split_across_chunks_is_found() {
	// Every possible cut through `\r\n\r\n`.
	for cut in 1..4 {
		let (head, tail) = "\r\n\r\n".split_at(cut);
		let first = format!("data: 1{head}");
		let second = format!("{tail}data: 2\r\n\r\n");

		let (events, rest) = events_of_chunks(&[&first, &second]);

		assert_eq!(events, ["data: 1", "data: 2"], "cut at {cut}");
		assert_eq!(rest, None, "cut at {cut}");
	}
}

#[test]
fn events_separated_by_carriage_returns_are_split() {
	let (events, rest) = events_of_chunks(&["data: 1\r\rdata: 2\r", "\rdata: 3"]);

	assert_eq!(events, ["data: 1", "data: 2"]);
	assert_eq!(rest.as_deref(), Some("data: 3"));
}

#[test]
fn event_split_inside_a_line_is_put_together() {
	let (events, rest) = events_of_chunks(&["data: {\"text\":\"hel", "lo\"}\r\n", "\r\n"]);

	assert_eq!(events, ["data: {\"text\":\"hello\"}"]);
	assert_eq!(rest, None);
}

#[tokio::test]
async fn crlf_event_stream_is_delivered_event_by_event() {
	use crate::webc::{Event, EventSourceStream};
	use futures::StreamExt;
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
		let body = "event: first\r\ndata: 1\r\n\r\n: keep-alive\r\n\r\ndata: 2\r\n\r\n";
		let response = format!(
			"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
			body.len()
		);
		socket.write_all(response.as_bytes()).expect("write the response");
	});

	let client = reqwest::Client::builder().no_proxy().build().expect("http client");
	let mut stream = EventSourceStream::new(client.get(format!("http://{address}/")));

	let mut messages = Vec::new();
	while let Some(event) = stream.next().await {
		match event.expect("the stream is read") {
			Event::Open => {}
			Event::Message(message) => messages.push((message.event, message.data)),
		}
	}

	assert_eq!(
		messages,
		[("first".to_string(), "1".to_string()), ("message".to_string(), "2".to_string())]
	);
}
