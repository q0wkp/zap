//! Fallback: read tool calls that a model wrote as text in its assistant `content`.
//!
//! Local models (Ollama native) sometimes write a tool call as text instead of
//! emitting structured `message.tool_calls`. When the BYOP stream ends with
//! `tool_bufs` empty, `chat_stream` calls [`extract_tool_calls_from_assistant_text`].
//!
//! What it returns gets executed, so the text is read as a call only when the
//! whole response is one, never when a call merely appears in it:
//!
//! - The response, after any leading `<think>…</think>` blocks, must consist of
//!   calls and nothing else, in one of these forms:
//!   - one or more JSON values separated by whitespace;
//!   - the same inside a single code fence, with no language or `json`;
//!   - one or more `<tool_call>…</tool_call>` blocks, each holding such values.
//! - Each JSON value is a call object, an array of call objects, or
//!   `{"tool_calls": [...]}` with no other key.
//! - A call object is `{"name", "arguments"}` (the arguments may also be under
//!   `parameters` or `input`) or `{"function": {"name", "arguments"}}` (or
//!   `parameters`), optionally with `id`, `index` and a `type` of `function` or
//!   `tool_use`. Any other key rejects it.
//! - The name must be exactly that of a tool offered in this request, and the
//!   arguments a JSON object, or a string holding one.
//!
//! If any part of the response fails this, no call is returned at all.

use std::collections::HashSet;

use genai::chat::ToolCall;
use serde_json::{Map, Value};
use uuid::Uuid;

const THINK_OPEN: &str = "<think>";
const THINK_CLOSE: &str = "</think>";
const TOOL_CALL_OPEN: &str = "<tool_call>";
const TOOL_CALL_CLOSE: &str = "</tool_call>";
const CODE_FENCE: &str = "```";

/// The keys a call object may have beside its name and arguments.
const WRAPPED_CALL_KEYS: &[&str] = &["id", "index", "type", "function"];
const FUNCTION_KEYS: &[&str] = &["name", "arguments", "parameters"];
const FUNCTION_ARGUMENT_KEYS: &[&str] = &["arguments", "parameters"];
const FLAT_CALL_KEYS: &[&str] = &[
    "id",
    "index",
    "type",
    "name",
    "arguments",
    "parameters",
    "input",
];
const FLAT_CALL_ARGUMENT_KEYS: &[&str] = &["arguments", "parameters", "input"];

/// Extract executable tool calls from assistant text when the provider did not
/// emit native `tool_calls` stream events. The module documentation defines
/// which texts are read as calls; any other text yields none.
pub fn extract_tool_calls_from_assistant_text(
    text: &str,
    available_tool_names: &[String],
) -> Vec<ToolCall> {
    let Some(values) = call_values_of_response(text) else {
        return Vec::new();
    };
    let allowed: HashSet<&str> = available_tool_names.iter().map(String::as_str).collect();

    let mut calls: Vec<ToolCall> = Vec::with_capacity(values.len());
    for value in &values {
        let Some((name, arguments)) = parse_call_object(value, &allowed) else {
            return Vec::new();
        };
        // A model that repeats itself asks for the call once.
        if calls
            .iter()
            .any(|call| call.fn_name == name && call.fn_arguments == arguments)
        {
            continue;
        }
        calls.push(ToolCall {
            call_id: Uuid::new_v4().to_string(),
            fn_name: name.to_owned(),
            fn_arguments: arguments,
            thought_signatures: None,
        });
    }
    calls
}

/// The candidate call objects of a response that consists of calls only, or
/// None when the response holds anything else.
fn call_values_of_response(text: &str) -> Option<Vec<Value>> {
    let body = strip_leading_think_blocks(text).trim();
    if body.is_empty() {
        return None;
    }
    let payloads = if body.starts_with(TOOL_CALL_OPEN) {
        tagged_payloads(body)?
    } else if body.starts_with(CODE_FENCE) {
        vec![fenced_payload(body)?]
    } else {
        vec![body]
    };

    let mut values = Vec::new();
    for payload in payloads {
        for value in serde_json::Deserializer::from_str(payload).into_iter::<Value>() {
            collect_call_values(value.ok()?, &mut values)?;
        }
    }
    Some(values)
}

/// Reasoning is never searched for calls: leading `<think>` blocks are skipped
/// whole. An unterminated block is left in place, which makes the text fail.
fn strip_leading_think_blocks(text: &str) -> &str {
    let mut rest = text.trim_start();
    while let Some(after_open) = rest.strip_prefix(THINK_OPEN) {
        let Some((_reasoning, after_close)) = after_open.split_once(THINK_CLOSE) else {
            return rest;
        };
        rest = after_close.trim_start();
    }
    rest
}

/// The contents of the `<tool_call>` blocks of a text made of such blocks only.
fn tagged_payloads(body: &str) -> Option<Vec<&str>> {
    let mut payloads = Vec::new();
    let mut rest = body;
    while !rest.is_empty() {
        let after_open = rest.strip_prefix(TOOL_CALL_OPEN)?;
        let (payload, after_close) = after_open.split_once(TOOL_CALL_CLOSE)?;
        payloads.push(payload);
        rest = after_close.trim_start();
    }
    Some(payloads)
}

/// The content of a text that is one code fence, with no language or `json`.
fn fenced_payload(body: &str) -> Option<&str> {
    let after_open = body.strip_prefix(CODE_FENCE)?;
    let (language, rest) = after_open.split_once('\n')?;
    let language = language.trim();
    if !language.is_empty() && !language.eq_ignore_ascii_case("json") {
        return None;
    }
    let payload = rest.strip_suffix(CODE_FENCE)?;
    (!payload.contains(CODE_FENCE)).then_some(payload)
}

/// Appends the call objects a top-level JSON value stands for: the object
/// itself, the items of an array, or the items of a `tool_calls` wrapper.
fn collect_call_values(value: Value, values: &mut Vec<Value>) -> Option<()> {
    match value {
        Value::Array(items) => {
            if !items.iter().all(Value::is_object) {
                return None;
            }
            values.extend(items);
        }
        Value::Object(mut object) => match object.remove("tool_calls") {
            Some(Value::Array(items)) if object.is_empty() => values.extend(items),
            // A wrapper with other keys, or whose `tool_calls` is not a list.
            Some(_) => return None,
            None => values.push(Value::Object(object)),
        },
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => return None,
    }
    Some(())
}

/// The name and arguments of a call object, when it has one of the accepted
/// shapes and names an offered tool.
fn parse_call_object<'a>(value: &'a Value, allowed: &HashSet<&str>) -> Option<(&'a str, Value)> {
    let object = value.as_object()?;
    if !has_call_type(object) {
        return None;
    }
    let (name, arguments) = match object.get("function") {
        Some(function) => {
            if !has_only_keys(object, WRAPPED_CALL_KEYS) {
                return None;
            }
            let function = function.as_object()?;
            if !has_only_keys(function, FUNCTION_KEYS) {
                return None;
            }
            (
                function.get("name")?.as_str()?,
                arguments_of(function, FUNCTION_ARGUMENT_KEYS)?,
            )
        }
        None => {
            if !has_only_keys(object, FLAT_CALL_KEYS) {
                return None;
            }
            (
                object.get("name")?.as_str()?,
                arguments_of(object, FLAT_CALL_ARGUMENT_KEYS)?,
            )
        }
    };
    allowed.contains(name).then_some((name, arguments))
}

fn has_call_type(object: &Map<String, Value>) -> bool {
    match object.get("type") {
        Some(call_type) => matches!(call_type.as_str(), Some("function" | "tool_use")),
        None => true,
    }
}

fn has_only_keys(object: &Map<String, Value>, keys: &[&str]) -> bool {
    object.keys().all(|key| keys.contains(&key.as_str()))
}

/// The arguments of a call: the value under the one of `keys` that is present,
/// which must be a JSON object or a string holding one.
fn arguments_of(object: &Map<String, Value>, keys: &[&str]) -> Option<Value> {
    let mut present = keys.iter().filter_map(|key| object.get(*key));
    let arguments = present.next()?;
    if present.next().is_some() {
        return None;
    }
    match arguments {
        Value::Object(_) => Some(arguments.clone()),
        Value::String(raw) => serde_json::from_str::<Value>(raw)
            .ok()
            .filter(Value::is_object),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => None,
    }
}

#[cfg(test)]
#[path = "content_tool_calls_tests.rs"]
mod tests;
