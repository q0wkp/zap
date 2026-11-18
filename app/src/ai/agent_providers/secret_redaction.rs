//! Secret redaction for everything BYOP sends to a provider.
//!
//! With secret redaction on (Settings → Privacy), no detected secret may leave in a request to
//! the user's model provider or to a web tool. Every text a request carries is passed through
//! the detector the UI masks secrets with (`find_secrets_in_text`: the built-in patterns plus
//! the user's and the enterprise's custom regexes), and each match is masked in place.
//!
//! The chat request is redacted as a whole once it is built, right before it is handed to the
//! client, rather than input by input: the history rebuilt from the local task messages,
//! compaction summaries, tool-call arguments and tool results are then covered whichever part
//! of the builder produced them, including what was stored before redaction was turned on.
//! The conversation as stored and shown locally is left as it is.

use genai::chat::{ChatRequest, ContentPart};
use serde_json::Value;

use crate::ai::agent::redaction::redact_secrets;

/// Masks every detected secret in the texts of `req`: the system prompt, every message part
/// that holds text (user and assistant text, reasoning, tool-call arguments, tool results) and
/// the tool definitions.
pub(super) fn redact_chat_request(req: &mut ChatRequest) {
    if let Some(system) = req.system.as_mut() {
        redact_secrets(system);
    }
    for message in &mut req.messages {
        for part in message.content.iter_mut() {
            redact_content_part(part);
        }
    }
    for tool in req.tools.iter_mut().flatten() {
        if let Some(description) = tool.description.as_mut() {
            redact_secrets(description);
        }
        if let Some(schema) = tool.schema.as_mut() {
            redact_json_strings(schema);
        }
    }
}

fn redact_content_part(part: &mut ContentPart) {
    match part {
        ContentPart::Text(text) | ContentPart::ReasoningContent(text) => redact_secrets(text),
        ContentPart::ToolCall(call) => redact_json_strings(&mut call.fn_arguments),
        ContentPart::ToolResponse(response) => redact_secrets(&mut response.content),
        // Binary attachments, provider signatures and raw provider parts are opaque: changing
        // them would break the request without hiding any text.
        ContentPart::Binary(_) | ContentPart::ThoughtSignature(_) | ContentPart::Custom(_) => {}
    }
}

/// Masks secrets in every string inside `value`, keeping the JSON structure (object keys are
/// left as they are) so that the result still parses as the same shape.
pub(super) fn redact_json_strings(value: &mut Value) {
    match value {
        Value::String(text) => redact_secrets(text),
        Value::Array(items) => items.iter_mut().for_each(redact_json_strings),
        Value::Object(fields) => fields.values_mut().for_each(redact_json_strings),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

#[cfg(test)]
#[path = "secret_redaction_tests.rs"]
pub(super) mod tests;
