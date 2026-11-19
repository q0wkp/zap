use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::anthropic::parse_cache_creation_details;
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{CacheCreationDetails, ChatOptionsSet, PromptTokensDetails, StopReason, ToolCall, Usage};
use crate::webc::{Event, EventSourceStream};
use crate::{Error, ModelIden, Result};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::pin::Pin;
use std::task::{Context, Poll};
use value_ext::JsonValueExt;

pub struct AnthropicStreamer {
	inner: EventSourceStream,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to prevent polling the EventSource after a MessageStop event
	done: bool,

	captured_data: StreamerCapturedData,
	in_progress_block: InProgressBlock,
	usage_counts: UsageCounts,
}

/// The usage counts as Anthropic reports them, each the highest value seen so far.
#[derive(Default)]
struct UsageCounts {
	input_tokens: Option<i32>,
	output_tokens: Option<i32>,
	cache_creation_tokens: Option<i32>,
	cache_read_tokens: Option<i32>,
	cache_creation_details: Option<CacheCreationDetails>,
}

impl UsageCounts {
	/// Same normalization as the non-streaming `AnthropicAdapter::into_usage()`.
	fn to_usage(&self) -> Usage {
		let cache_creation = self.cache_creation_tokens.unwrap_or(0);
		let cache_read = self.cache_read_tokens.unwrap_or(0);

		// NOTE: Anthropic's input_tokens does NOT include cached tokens, so we must add them.
		let prompt_tokens = if self.input_tokens.is_some() || cache_creation > 0 || cache_read > 0 {
			Some(self.input_tokens.unwrap_or(0) + cache_creation + cache_read)
		} else {
			None
		};

		let prompt_tokens_details = if cache_creation > 0 || cache_read > 0 || self.cache_creation_details.is_some() {
			Some(PromptTokensDetails {
				cache_creation_tokens: Some(cache_creation),
				cache_creation_details: self.cache_creation_details.clone(),
				cached_tokens: Some(cache_read),
				audio_tokens: None,
			})
		} else {
			None
		};

		Usage {
			prompt_tokens,
			prompt_tokens_details,
			completion_tokens: self.output_tokens,
			completion_tokens_details: None,
			// Computed at `message_stop`.
			total_tokens: None,
		}
	}
}

/// Raises `count` to the value of `key` in `usage_data`, when it is present.
fn merge_count(count: &mut Option<i32>, usage_data: &Value, key: &str) {
	if let Some(value) = usage_data.get(key).and_then(Value::as_i64) {
		let value = value as i32;
		*count = Some(count.map_or(value, |current| current.max(value)));
	}
}

/// The type of an SSE frame: its event name, or, when the frame has none (some proxies
/// strip the `event:` lines and forward only `data:`), the `type` field of its JSON data,
/// which always repeats the event name.
fn frame_type<'a>(event: &'a str, data: &str) -> Cow<'a, str> {
	// `message` is what the event source reports for a frame without an event name.
	if event != "message" {
		return Cow::Borrowed(event);
	}

	serde_json::from_str::<Value>(data)
		.ok()
		.and_then(|data| data.get("type").and_then(Value::as_str).map(str::to_owned))
		.map_or(Cow::Borrowed(event), Cow::Owned)
}

enum InProgressBlock {
	Text,
	ToolUse { id: String, name: String, input: String },
	Thinking,
	/// A block of a type this streamer does not surface (e.g. `server_tool_use`,
	/// `redacted_thinking`). Its deltas are skipped.
	Ignored,
}

impl AnthropicStreamer {
	pub fn new(inner: EventSourceStream, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
			in_progress_block: InProgressBlock::Text,
			usage_counts: Default::default(),
		}
	}
}

impl futures::Stream for AnthropicStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		while let Poll::Ready(event) = Pin::new(&mut self.inner).poll_next(cx) {
			// NOTE: At this point, we capture more events than needed for genai::StreamItem, but it serves as documentation.
			match event {
				Some(Ok(Event::Open)) => return Poll::Ready(Some(Ok(InterStreamEvent::Start))),
				Some(Ok(Event::Message(message))) => {
					let message_type = frame_type(&message.event, &message.data);
					let message_type = message_type.as_ref();

					match message_type {
						"message_start" => {
							self.capture_usage(message_type, &message.data)?;
							continue;
						}
						"message_delta" => {
							self.capture_usage(message_type, &message.data)?;
							// Capture stop_reason from delta (e.g., "end_turn", "max_tokens", "tool_use")
							if let Ok(data) = self.parse_message_data(&message.data)
								&& let Ok(reason) = data.x_get::<String>("/delta/stop_reason")
							{
								self.captured_data.stop_reason = Some(reason);
							}
							continue;
						}
						"content_block_start" => {
							if let Some(event) = self.on_content_block_start(&message.data)? {
								return Poll::Ready(Some(Ok(event)));
							}
							continue;
						}
						"content_block_delta" => {
							if let Some(event) = self.on_content_block_delta(&message.data)? {
								return Poll::Ready(Some(Ok(event)));
							}
							continue;
						}
						"content_block_stop" => {
							self.on_content_block_stop();
							continue;
						}
						// -- ERROR
						"error" => {
							// The server sends nothing after an error: do not poll it again.
							self.done = true;
							return Poll::Ready(Some(Err(self.stream_error(&message.data)?)));
						}
						// -- END MESSAGE
						"message_stop" => {
							// Ensure we do not poll the EventSource anymore on the next poll.
							// NOTE: This way, the last MessageStop event is still sent,
							//       but then, on the next poll, it will be stopped.
							self.done = true;

							// Capture the usage
							let captured_usage = if self.options.capture_usage {
								self.captured_data.usage.take().map(|mut usage| {
									// Compute the total if any of input/output are not null
									if usage.prompt_tokens.is_some() || usage.completion_tokens.is_some() {
										usage.total_tokens = Some(
											usage.prompt_tokens.unwrap_or(0) + usage.completion_tokens.unwrap_or(0),
										);
									}
									usage
								})
							} else {
								None
							};

							let inter_stream_end = InterStreamEnd {
								captured_usage,
								captured_stop_reason: self.captured_data.stop_reason.take().map(StopReason::from),
								captured_text_content: self.captured_data.content.take(),
								captured_reasoning_content: self.captured_data.reasoning_content.take(),
								captured_tool_calls: self.captured_data.tool_calls.take(),
								captured_thought_signatures: None,
								captured_response_id: None,
							};

							// TODO: Need to capture the data as needed
							return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
						}

						"ping" => continue, // Loop to the next event
						other => tracing::warn!("UNKNOWN MESSAGE TYPE: {other}"),
					}
				}
				Some(Err(err)) => {
					tracing::error!("Error: {}", err);
					return Poll::Ready(Some(Err(Error::WebStream {
						model_iden: self.options.model_iden.clone(),
						cause: err.to_string(),
						error: err,
					})));
				}
				None => return Poll::Ready(None),
			}
		}
		Poll::Pending
	}
}

// Support
impl AnthropicStreamer {
	/// Handles a `content_block_start` message, returning the event to emit, if any.
	fn on_content_block_start(&mut self, message_data: &str) -> Result<Option<InterStreamEvent>> {
		let mut data = self.parse_message_data(message_data)?;

		match data.x_get_str("/content_block/type") {
			Ok("text") => self.in_progress_block = InProgressBlock::Text,
			Ok("thinking") => self.in_progress_block = InProgressBlock::Thinking,
			Ok("tool_use") => {
				let id: String = data.x_take("/content_block/id")?;
				let name: String = data.x_take("/content_block/name")?;

				// Emit an initial ToolCallChunk with name and empty args,
				// matching OpenAI's incremental streaming behaviour.
				let tc = ToolCall {
					call_id: id.clone(),
					fn_name: name.clone(),
					fn_arguments: Value::String(String::new()),
					thought_signatures: None,
				};

				self.in_progress_block = InProgressBlock::ToolUse {
					id,
					name,
					input: String::new(),
				};

				return Ok(Some(InterStreamEvent::ToolCallChunk(tc)));
			}
			Ok(txt) => {
				tracing::warn!("unhandled content type: {txt}");
				// Without this, the deltas of this block would be read as deltas
				// of the previous block's type and fail the whole stream.
				self.in_progress_block = InProgressBlock::Ignored;
			}
			Err(e) => {
				tracing::error!("{e:?}");
				self.in_progress_block = InProgressBlock::Ignored;
			}
		}

		Ok(None)
	}

	/// Handles a `content_block_delta` message, returning the event to emit, if any.
	fn on_content_block_delta(&mut self, message_data: &str) -> Result<Option<InterStreamEvent>> {
		let mut data = self.parse_message_data(message_data)?;

		match &mut self.in_progress_block {
			InProgressBlock::Text => {
				// A text block also receives deltas that carry no text (`citations_delta`).
				let Ok(content) = data.x_take::<String>("/delta/text") else {
					return Ok(None);
				};

				// Add to the captured_content if chat options say so
				if self.options.capture_content {
					match self.captured_data.content {
						Some(ref mut c) => c.push_str(&content),
						None => self.captured_data.content = Some(content.clone()),
					}
				}

				Ok(Some(InterStreamEvent::Chunk(content)))
			}
			InProgressBlock::ToolUse { id, name, input } => {
				let partial = data.x_get_str("/delta/partial_json")?;
				input.push_str(partial);

				// Emit incremental ToolCallChunk with accumulated args
				// (as Value::String, same convention as OpenAI adapter).
				let tc = ToolCall {
					call_id: id.clone(),
					fn_name: name.clone(),
					fn_arguments: Value::String(input.clone()),
					thought_signatures: None,
				};

				Ok(Some(InterStreamEvent::ToolCallChunk(tc)))
			}
			InProgressBlock::Thinking => {
				if let Ok(thinking) = data.x_take::<String>("/delta/thinking") {
					// Add to the captured_thinking if chat options say so
					if self.options.capture_reasoning_content {
						match self.captured_data.reasoning_content {
							Some(ref mut r) => r.push_str(&thinking),
							None => self.captured_data.reasoning_content = Some(thinking.clone()),
						}
					}

					Ok(Some(InterStreamEvent::ReasoningChunk(thinking)))
				} else if let Ok(signature) = data.x_take::<String>("/delta/signature") {
					Ok(Some(InterStreamEvent::ThoughtSignatureChunk(signature)))
				} else {
					// If it is thinking but no thinking or signature field, we log and skip.
					tracing::warn!(
						"content_block_delta for thinking block but no thinking or signature found: {data:?}"
					);
					Ok(None)
				}
			}
			InProgressBlock::Ignored => Ok(None),
		}
	}

	/// Handles a `content_block_stop` message: captures the finished tool call, if the block was one.
	fn on_content_block_stop(&mut self) {
		match std::mem::replace(&mut self.in_progress_block, InProgressBlock::Text) {
			InProgressBlock::ToolUse { id, name, input } if self.options.capture_tool_calls => {
				// ToolCallChunks were already emitted incrementally
				// during content_block_start and content_block_delta.
				// Here we only finalize capture with parsed arguments.
				let fn_arguments = if input.is_empty() {
					Value::Object(Map::new())
				} else {
					// Arguments that do not parse (cut off, or malformed by the model) are
					// kept as the raw string, as the OpenAI streamer does, so the caller can
					// answer the model about that one call instead of losing the whole turn.
					serde_json::from_str(&input).unwrap_or(Value::String(input))
				};

				let tc = ToolCall {
					call_id: id,
					fn_name: name,
					fn_arguments,
					thought_signatures: None,
				};

				match self.captured_data.tool_calls {
					Some(ref mut t) => t.push(tc),
					None => self.captured_data.tool_calls = Some(vec![tc]),
				}
			}
			InProgressBlock::ToolUse { .. }
			| InProgressBlock::Text
			| InProgressBlock::Thinking
			| InProgressBlock::Ignored => {
				// no-op for remaining block types
			}
		}
	}

	/// Builds the error of an `error` message (`overloaded_error`, `api_error`, ...), whose
	/// body is the message's `error` object, as the OpenAI streamer reports inline errors.
	fn stream_error(&self, message_data: &str) -> Result<Error> {
		let mut data = self.parse_message_data(message_data)?;
		let body = data.x_take::<Value>("error").unwrap_or(data);
		Ok(Error::ChatResponse {
			model_iden: self.options.model_iden.clone(),
			body,
		})
	}

	fn capture_usage(&mut self, message_type: &str, message_data: &str) -> Result<()> {
		if self.options.capture_usage {
			let data = self.parse_message_data(message_data)?;

			let usage_path = if message_type == "message_start" {
				"/message/usage"
			} else if message_type == "message_delta" {
				"/usage"
			} else {
				tracing::debug!(
					"TRACING DEBUG - Anthropic message type not supported for input/output tokens: {message_type}"
				);
				return Ok(()); // For now permissive
			};

			// NOTE: Permissive on this one; if the usage is not found, there is nothing to capture (for now)
			let Ok(usage_data) = data.x_get::<Value>(usage_path) else {
				return Ok(());
			};

			// The counts of `message_delta` are cumulative: they restate those of
			// `message_start` (the input and cache tokens too, when present), so each
			// count replaces the previous one instead of being added to it. Taking the
			// higher of the two also covers the servers that report the input tokens
			// only in one of the two messages and zero in the other.
			let counts = &mut self.usage_counts;
			merge_count(&mut counts.input_tokens, &usage_data, "input_tokens");
			merge_count(&mut counts.output_tokens, &usage_data, "output_tokens");
			merge_count(&mut counts.cache_creation_tokens, &usage_data, "cache_creation_input_tokens");
			merge_count(&mut counts.cache_read_tokens, &usage_data, "cache_read_input_tokens");
			// TTL-specific breakdown of the cache creation tokens
			if let Some(details) = usage_data.get("cache_creation").and_then(parse_cache_creation_details) {
				counts.cache_creation_details = Some(details);
			}

			self.captured_data.usage = Some(self.usage_counts.to_usage());
		}

		Ok(())
	}

	/// Simple wrapper for now, with the corresponding map_err.
	/// Might have more logic later.
	fn parse_message_data(&self, payload: &str) -> Result<Value> {
		serde_json::from_str(payload).map_err(|serde_error| Error::StreamParse {
			model_iden: self.options.model_iden.clone(),
			serde_error,
		})
	}
}

#[cfg(test)]
#[path = "streamer_tests.rs"]
mod tests;
