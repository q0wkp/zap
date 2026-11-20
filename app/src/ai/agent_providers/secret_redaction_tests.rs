// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::Path;

use genai::chat::{
    ChatMessage, ChatRequest, ContentPart, MessageContent, Tool, ToolCall, ToolResponse,
};
use serde_json::json;
use serial_test::serial;

use super::{redact_chat_request, redact_json_strings};
use crate::ai::agent_providers::oneshot::{build_oneshot_request, OneshotConfig, OneshotOptions};
use crate::settings::{AgentProviderApiType, ReasoningEffortSetting};

/// A value the custom regex set by [`SecretPattern`] detects.
pub(in crate::ai::agent_providers) const SECRET: &str = "zap-test-secret-314159";

/// [`SECRET`] as redaction leaves it.
fn masked() -> String {
    "*".repeat(SECRET.len())
}

/// Sets a custom secret regex that detects [`SECRET`] for as long as it lives, and clears the
/// regexes again when dropped, so that tests run afterwards see the default (empty) set.
/// Tests that use it are `#[serial]`, since the regex set is global.
pub(in crate::ai::agent_providers) struct SecretPattern;

impl SecretPattern {
    pub(in crate::ai::agent_providers) fn set() -> Self {
        let regex = regex::Regex::new(r"zap-test-secret-[0-9]{6}").expect("valid regex");
        crate::terminal::model::set_user_and_enterprise_secret_regexes(
            [&regex],
            std::iter::empty(),
        );
        SecretPattern
    }
}

impl Drop for SecretPattern {
    fn drop(&mut self) {
        crate::terminal::model::set_user_and_enterprise_secret_regexes(
            std::iter::empty(),
            std::iter::empty(),
        );
    }
}

fn wire(req: &ChatRequest) -> String {
    serde_json::to_string(req).expect("request serializes")
}

fn request_with_secret_everywhere() -> ChatRequest {
    let call = ToolCall {
        call_id: "call-1".to_owned(),
        fn_name: "run_shell_command".to_owned(),
        fn_arguments: json!({
            "command": format!("curl -H 'Authorization: {SECRET}' example.com"),
            "env": { "TOKEN": SECRET, "LIST": [SECRET, 3] },
            "timeout": 30,
        }),
        thought_signatures: None,
    };
    ChatRequest::from_messages(vec![
        ChatMessage::user(format!("my key is {SECRET}")),
        ChatMessage::assistant(MessageContent::from_parts(vec![
            ContentPart::ReasoningContent(format!("the user gave {SECRET}")),
            ContentPart::Text(format!("I will use {SECRET}")),
            ContentPart::ToolCall(call),
        ])),
        ChatMessage::user(ToolResponse::new(
            "call-1",
            format!("{{\"output\":\"{SECRET}\"}}"),
        )),
        ChatMessage::assistant(format!("summary of earlier turns: {SECRET}")),
    ])
    .with_system(format!("rules: never print {SECRET}"))
    .with_tools(vec![Tool::new("mcp_tool")
        .with_description(format!("calls a server with {SECRET}"))
        .with_schema(json!({
            "type": "object",
            "properties": { "key": { "type": "string", "default": SECRET } },
        }))])
}

#[test]
#[serial]
fn every_text_in_a_chat_request_is_masked() {
    let _pattern = SecretPattern::set();
    let mut req = request_with_secret_everywhere();
    assert_eq!(wire(&req).matches(SECRET).count(), 11, "{}", wire(&req));

    redact_chat_request(&mut req);

    let wire = wire(&req);
    assert!(!wire.contains(SECRET), "a secret is left in {wire}");
    assert_eq!(
        req.system.as_deref(),
        Some(format!("rules: never print {}", masked()).as_str())
    );
}

#[test]
#[serial]
fn tool_call_arguments_keep_their_json_shape() {
    let _pattern = SecretPattern::set();
    let mut args = json!({
        "command": format!("echo {SECRET}"),
        "env": { "TOKEN": SECRET, "LIST": [SECRET, 3, true, null] },
        "timeout": 30,
    });

    redact_json_strings(&mut args);

    let masked = masked();
    assert_eq!(
        args,
        json!({
            "command": format!("echo {masked}"),
            "env": { "TOKEN": masked, "LIST": [masked, 3, true, null] },
            "timeout": 30,
        })
    );
}

#[test]
#[serial]
fn opaque_parts_are_left_as_they_are() {
    let _pattern = SecretPattern::set();
    let mut req = ChatRequest::from_messages(vec![ChatMessage::assistant(
        MessageContent::from_parts(vec![ContentPart::ThoughtSignature(SECRET.to_owned())]),
    )]);

    redact_chat_request(&mut req);

    assert!(matches!(
        req.messages[0].content.parts().as_slice(),
        [ContentPart::ThoughtSignature(signature)] if signature == SECRET
    ));
}

fn oneshot_config(redact_secrets: bool) -> OneshotConfig {
    OneshotConfig {
        base_url: "https://api.example.com/v1".to_owned(),
        api_key: String::new(),
        model_id: "model".to_owned(),
        api_type: AgentProviderApiType::OpenAi,
        reasoning_effort: ReasoningEffortSetting::default(),
        redact_secrets,
    }
}

#[test]
#[serial]
fn one_shot_requests_are_masked_only_when_redaction_is_on() {
    let _pattern = SecretPattern::set();
    let system = format!("system {SECRET}");
    let user = format!("explain `export TOKEN={SECRET}`");
    let opts = OneshotOptions::default();

    let (on, _) = build_oneshot_request(&oneshot_config(true), &system, &user, &opts);
    assert!(!wire(&on).contains(SECRET), "{}", wire(&on));

    let (off, _) = build_oneshot_request(&oneshot_config(false), &system, &user, &opts);
    assert_eq!(wire(&off).matches(SECRET).count(), 2, "{}", wire(&off));
}

/// Every call in `app/src` that sends a request to a model provider, and every HTTP request the
/// BYOP code makes, with how many times it occurs in its file. Whatever such a call sends has to
/// have detected secrets masked when secret redaction is on:
///
/// - `chat_stream.rs`: the agent conversation, built by `build_outbound_chat_request`.
/// - `oneshot.rs`: one-shot completions (titles, active AI, next command, workflow metadata),
///   built by `build_oneshot_request`.
/// - `tools/web_runtime.rs`: web fetch (sends only the URL) and web search (its query is masked
///   by `dispatch_byop_web_tool`).
/// - `models_dev.rs`, `openai_compatible.rs`: model catalogue and model list lookups, which send
///   no user content.
///
/// A new outbound call fails `no_outbound_call_is_left_unlisted` until it is listed here, after
/// its content has been routed through `secret_redaction`.
const OUTBOUND_CALLS: &[(&str, &str, usize)] = &[
    ("ai/agent_providers/chat_stream.rs", ".exec_chat_stream(", 1),
    ("ai/agent_providers/oneshot.rs", ".exec_chat(", 1),
    ("ai/agent_providers/oneshot.rs", ".exec_chat_stream(", 1),
    ("ai/agent_providers/tools/web_runtime.rs", ".send()", 2),
    ("ai/agent_providers/models_dev.rs", ".send()", 1),
    ("ai/agent_providers/openai_compatible.rs", ".send()", 1),
];

/// genai client calls, looked for in all of `app/src`.
const PROVIDER_CALLS: &[&str] = &[
    ".exec_chat(",
    ".exec_chat_stream(",
    ".exec_embed(",
    ".embed(",
    ".embed_batch(",
];

/// HTTP sends, looked for in the BYOP code only.
const HTTP_CALLS: &[&str] = &[".send()"];

fn collect_rust_sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("source directory is readable") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if !name.ends_with("_tests.rs") && !name.ends_with("_test.rs") {
                out.push(path);
            }
        }
    }
}

#[test]
fn no_outbound_call_is_left_unlisted() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rust_sources(&src, &mut files);

    let mut found: Vec<(String, &str, usize)> = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&src)
            .expect("file is under src")
            .to_string_lossy()
            .replace('\\', "/");
        let source = std::fs::read_to_string(&file).expect("source file is readable");
        let code_lines: Vec<&str> = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        let patterns = PROVIDER_CALLS.iter().chain(
            HTTP_CALLS
                .iter()
                .filter(|_| relative.starts_with("ai/agent_providers/")),
        );
        for pattern in patterns {
            let count = code_lines
                .iter()
                .map(|line| line.matches(pattern).count())
                .sum::<usize>();
            if count > 0 {
                found.push((relative.clone(), *pattern, count));
            }
        }
    }
    found.sort();

    let mut expected: Vec<(String, &str, usize)> = OUTBOUND_CALLS
        .iter()
        .map(|(file, pattern, count)| ((*file).to_owned(), *pattern, *count))
        .collect();
    expected.sort();

    assert_eq!(
        found, expected,
        "the calls that send requests out changed; mask detected secrets in what a new call \
         sends when secret redaction is on, then update OUTBOUND_CALLS"
    );
}
