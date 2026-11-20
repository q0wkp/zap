//! Live capture of the traffic between zap-ng and BYOP providers, shown by the request inspector
//! pane ([`view::RequestInspectorView`]).
//!
//! Nothing is captured unless an inspector is listening: every capture point starts with
//! [`begin`], which returns `None` after a single atomic load when no inspector is open, so the
//! provider path then does no extra work at all. This module keeps no buffer: captured events go
//! straight to the open inspectors through channels, and each inspector keeps its own bounded
//! [`store::CallStore`].
//!
//! A request is captured at the last point before it leaves. Its body is built from the final
//! `ChatRequest` and `ChatOptions` by the same genai adapter that then sends them
//! ([`genai::Client::chat_web_request`]), so the inspector shows the request as sent, including
//! whatever was done to it before that point. The capture points are the `exec_chat_stream` call
//! of `chat_stream::generate_byop_output` and the `exec_chat` / `exec_chat_stream` calls of
//! `oneshot`.

pub mod store;
pub mod view;

use std::fmt::Display;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Local};
use genai::chat::{ChatOptions, ChatRequest, ToolCall, Usage};
use genai::Client;
use serde_json::Value;

/// What a captured call was made for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CallKind {
    /// A turn of an agent conversation (the streamed main request).
    AgentTurn,
    /// Generating a conversation's title.
    Title,
    /// Prompt suggestions after a command finishes.
    PromptSuggestions,
    /// Completing what is typed in the agent input.
    InputPrediction,
    /// Picking the files relevant to a query.
    RelevantFiles,
    /// Filling in a workflow's metadata.
    WorkflowMetadata,
    /// Suggesting the next command.
    NextCommand,
    /// Explaining a command the agent asks to run.
    ExplainCommand,
    /// A one-shot call whose caller did not say what it is for.
    #[default]
    Other,
}

/// The groups the inspector filters calls by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CallGroup {
    AgentTurns,
    Titles,
    ActiveAi,
}

impl CallGroup {
    pub const ALL: [CallGroup; 3] = [
        CallGroup::AgentTurns,
        CallGroup::Titles,
        CallGroup::ActiveAi,
    ];

    pub fn label(self) -> String {
        match self {
            CallGroup::AgentTurns => crate::t!("byop-inspector-group-agent"),
            CallGroup::Titles => crate::t!("byop-inspector-group-title"),
            CallGroup::ActiveAi => crate::t!("byop-inspector-group-active-ai"),
        }
    }
}

impl CallKind {
    pub fn group(self) -> CallGroup {
        match self {
            CallKind::AgentTurn => CallGroup::AgentTurns,
            CallKind::Title => CallGroup::Titles,
            CallKind::PromptSuggestions
            | CallKind::InputPrediction
            | CallKind::RelevantFiles
            | CallKind::WorkflowMetadata
            | CallKind::NextCommand
            | CallKind::ExplainCommand
            | CallKind::Other => CallGroup::ActiveAi,
        }
    }

    pub fn label(self) -> String {
        match self {
            CallKind::AgentTurn => crate::t!("byop-inspector-kind-agent-turn"),
            CallKind::Title => crate::t!("byop-inspector-kind-title"),
            CallKind::PromptSuggestions => crate::t!("byop-inspector-kind-prompt-suggestions"),
            CallKind::InputPrediction => crate::t!("byop-inspector-kind-input-prediction"),
            CallKind::RelevantFiles => crate::t!("byop-inspector-kind-relevant-files"),
            CallKind::WorkflowMetadata => crate::t!("byop-inspector-kind-workflow-metadata"),
            CallKind::NextCommand => crate::t!("byop-inspector-kind-next-command"),
            CallKind::ExplainCommand => crate::t!("byop-inspector-kind-explain-command"),
            CallKind::Other => crate::t!("byop-inspector-kind-other"),
        }
    }
}

/// Who made a call: the caller's side of a capture.
#[derive(Clone, Copy, Debug)]
pub struct CallOrigin<'a> {
    pub kind: CallKind,
    /// The root task id of the conversation the call belongs to, when it belongs to one.
    pub conversation: Option<&'a str>,
}

impl<'a> CallOrigin<'a> {
    pub fn new(kind: CallKind) -> Self {
        Self {
            kind,
            conversation: None,
        }
    }

    pub fn in_conversation(mut self, conversation: &'a str) -> Self {
        if !conversation.is_empty() {
            self.conversation = Some(conversation);
        }
        self
    }
}

/// A request as it was sent.
#[derive(Debug)]
pub struct CapturedRequest {
    pub id: u64,
    pub started_at: DateTime<Local>,
    pub kind: CallKind,
    pub conversation: Option<String>,
    pub model_id: String,
    /// The endpoint, without credentials or query values (see [`display_url`]).
    pub url: Option<String>,
    /// The request body, pretty-printed JSON with inline binary data elided.
    pub body: String,
}

/// How a call ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallOutcome {
    Completed,
    Failed(String),
    /// The call stopped before it completed or failed: cancelled, or its stream was dropped.
    Interrupted,
}

/// One step of a call, as sent to the inspectors.
#[derive(Clone, Debug)]
pub enum CaptureEvent {
    Started(Arc<CapturedRequest>),
    Text {
        call: u64,
        text: String,
    },
    Reasoning {
        call: u64,
        text: String,
    },
    /// A tool call as far as it has arrived; a later event for the same `call_id` replaces it.
    ToolCall {
        call: u64,
        call_id: String,
        name: String,
        arguments: String,
    },
    Usage {
        call: u64,
        usage: String,
    },
    Finished {
        call: u64,
        at: DateTime<Local>,
        outcome: CallOutcome,
    },
}

struct Listener {
    id: u64,
    sender: async_channel::Sender<CaptureEvent>,
}

static LISTENERS: Mutex<Vec<Listener>> = Mutex::new(Vec::new());
static LISTENER_COUNT: AtomicUsize = AtomicUsize::new(0);
static NEXT_LISTENER_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_CALL_ID: AtomicU64 = AtomicU64::new(1);

fn listeners() -> std::sync::MutexGuard<'static, Vec<Listener>> {
    LISTENERS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Whether an inspector is listening, so that calls are captured.
pub fn is_listening() -> bool {
    LISTENER_COUNT.load(Ordering::Relaxed) > 0
}

/// Keeps a listener registered; dropping it stops new calls being captured for that listener.
/// Calls already started keep sending to its receiver until they end.
pub struct Subscription {
    id: u64,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let mut listeners = listeners();
        listeners.retain(|listener| listener.id != self.id);
        LISTENER_COUNT.store(listeners.len(), Ordering::Relaxed);
    }
}

/// Registers a listener: every call that starts from now on is captured and sent to the
/// returned receiver, until the subscription is dropped.
pub fn subscribe() -> (Subscription, async_channel::Receiver<CaptureEvent>) {
    let (sender, receiver) = async_channel::unbounded();
    let id = NEXT_LISTENER_ID.fetch_add(1, Ordering::Relaxed);
    let mut listeners = listeners();
    listeners.push(Listener { id, sender });
    LISTENER_COUNT.store(listeners.len(), Ordering::Relaxed);
    (Subscription { id }, receiver)
}

/// The capture of one call, held by the code making it. Dropping it before
/// [`CallCapture::complete`] or [`CallCapture::fail`] marks the call as interrupted.
pub struct CallCapture {
    id: u64,
    senders: Vec<async_channel::Sender<CaptureEvent>>,
    finished: bool,
}

impl CallCapture {
    fn send(&self, event: CaptureEvent) {
        for sender in &self.senders {
            // The receiver is gone when its inspector was closed mid-call; nothing to do then.
            let _ = sender.try_send(event.clone());
        }
    }

    pub fn text(&self, text: &str) {
        if !text.is_empty() {
            self.send(CaptureEvent::Text {
                call: self.id,
                text: text.to_owned(),
            });
        }
    }

    pub fn reasoning(&self, text: &str) {
        if !text.is_empty() {
            self.send(CaptureEvent::Reasoning {
                call: self.id,
                text: text.to_owned(),
            });
        }
    }

    pub fn tool_call(&self, call: &ToolCall) {
        self.send(CaptureEvent::ToolCall {
            call: self.id,
            call_id: call.call_id.clone(),
            name: call.fn_name.clone(),
            arguments: pretty_json(&call.fn_arguments),
        });
    }

    pub fn usage(&self, usage: &Usage) {
        let usage = serde_json::to_value(usage)
            .map(|value| pretty_json(&value))
            .unwrap_or_else(|err| err.to_string());
        self.send(CaptureEvent::Usage {
            call: self.id,
            usage,
        });
    }

    fn finish(mut self, outcome: CallOutcome) {
        self.finished = true;
        self.send(CaptureEvent::Finished {
            call: self.id,
            at: Local::now(),
            outcome,
        });
    }

    pub fn complete(self) {
        self.finish(CallOutcome::Completed);
    }

    pub fn fail(self, error: impl Display) {
        self.finish(CallOutcome::Failed(format!("{error:#}")));
    }
}

impl Drop for CallCapture {
    fn drop(&mut self) {
        if !self.finished {
            self.send(CaptureEvent::Finished {
                call: self.id,
                at: Local::now(),
                outcome: CallOutcome::Interrupted,
            });
        }
    }
}

/// Starts capturing a call that is about to send `chat_req` with `chat_opts` through `client`.
/// Returns `None`, having done nothing else, when no inspector is listening.
pub async fn begin(
    origin: CallOrigin<'_>,
    client: &Client,
    model_id: &str,
    chat_req: &ChatRequest,
    chat_opts: &ChatOptions,
    streamed: bool,
) -> Option<CallCapture> {
    if !is_listening() {
        return None;
    }

    let (url, body) = match client
        .chat_web_request(model_id, chat_req.clone(), Some(chat_opts), streamed)
        .await
    {
        Ok(request) => (
            Some(display_url(&request.url)),
            render_body(request.payload),
        ),
        // Sending will fail the same way; show the error with the request it was built from.
        Err(err) => {
            let chat_req = serde_json::to_value(chat_req)
                .map(render_body)
                .unwrap_or_else(|err| err.to_string());
            (None, format!("{err}\n\n{chat_req}"))
        }
    };

    let senders: Vec<_> = listeners()
        .iter()
        .map(|listener| listener.sender.clone())
        .collect();
    if senders.is_empty() {
        return None;
    }

    let capture = CallCapture {
        id: NEXT_CALL_ID.fetch_add(1, Ordering::Relaxed),
        senders,
        finished: false,
    };
    capture.send(CaptureEvent::Started(Arc::new(CapturedRequest {
        id: capture.id,
        started_at: Local::now(),
        kind: origin.kind,
        conversation: origin.conversation.map(str::to_owned),
        model_id: model_id.to_owned(),
        url,
        body,
    })));
    Some(capture)
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|err| err.to_string())
}

/// Strings at least this long that are entirely base64 are taken for inline binary data.
const MIN_ELIDED_BASE64_LEN: usize = 512;

/// Pretty-prints a request body with inline binary data (images, PDFs, audio) replaced by a
/// note of its size: the bytes say nothing to a reader and would swamp the view.
fn render_body(mut payload: Value) -> String {
    elide_binary_data(&mut payload);
    pretty_json(&payload)
}

fn elide_binary_data(value: &mut Value) {
    match value {
        Value::String(text) => {
            if let Some(elided) = elide_base64(text) {
                *text = elided;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(elide_binary_data),
        Value::Object(fields) => fields.values_mut().for_each(elide_binary_data),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// Returns the replacement for `text` when it is base64 data, optionally as a `data:` URL.
fn elide_base64(text: &str) -> Option<String> {
    if text.len() < MIN_ELIDED_BASE64_LEN {
        return None;
    }
    let (prefix, data) = match text.strip_prefix("data:") {
        Some(rest) => {
            let comma = rest.find(";base64,")? + ";base64,".len();
            text.split_at("data:".len() + comma)
        }
        None => ("", text),
    };
    let is_base64 = data.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=' | b'-' | b'_')
    });
    if !is_base64 || data.len() < MIN_ELIDED_BASE64_LEN {
        return None;
    }
    // Copied text carries this note, so it is built without Fluent placeables, which would
    // wrap the number in invisible isolation marks.
    Some(format!(
        "{prefix}<{}: {}>",
        crate::t!("byop-inspector-binary-elided"),
        data.len()
    ))
}

/// The URL of a request as the inspector shows it: without user info, and with the values of
/// its query parameters masked, since either may hold a credential.
pub fn display_url(raw: &str) -> String {
    let Ok(mut url) = url::Url::parse(raw) else {
        return raw.to_owned();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_fragment(None);
    let masked_query = url.query().map(|_| {
        url.query_pairs()
            .map(|(name, _)| format!("{name}=…"))
            .collect::<Vec<_>>()
            .join("&")
    });
    url.set_query(None);
    match masked_query {
        Some(query) => format!("{url}?{query}"),
        None => url.to_string(),
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
