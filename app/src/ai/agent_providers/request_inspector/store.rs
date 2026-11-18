//! The calls an inspector holds: a buffer bounded in both entries and bytes, the filter the
//! inspector shows it through, and the plain-text form an entry is copied in.

use std::collections::{HashSet, VecDeque};
use std::fmt::Write as _;
use std::sync::Arc;

use chrono::{DateTime, Local};

use super::{CallGroup, CallOutcome, CaptureEvent, CapturedRequest};

/// The most calls an inspector keeps; older ones are dropped first.
pub const MAX_CALLS: usize = 200;
/// The most text an inspector keeps over all its calls; older calls are dropped first.
pub const MAX_BYTES: usize = 32 * 1024 * 1024;
/// The most response text kept for one call; what streams in after that is dropped.
pub const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// Where a call stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallStatus {
    InProgress,
    Finished {
        at: DateTime<Local>,
        outcome: CallOutcome,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCallRecord {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

/// One captured call: its request and the response as far as it has arrived.
#[derive(Debug)]
pub struct CallRecord {
    pub request: Arc<CapturedRequest>,
    pub reasoning: String,
    pub text: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub usage: Option<String>,
    pub status: CallStatus,
    /// Whether response text was dropped for exceeding [`MAX_RESPONSE_BYTES`].
    pub response_truncated: bool,
}

impl CallRecord {
    fn new(request: Arc<CapturedRequest>) -> Self {
        Self {
            request,
            reasoning: String::new(),
            text: String::new(),
            tool_calls: Vec::new(),
            usage: None,
            status: CallStatus::InProgress,
            response_truncated: false,
        }
    }

    fn response_bytes(&self) -> usize {
        self.reasoning.len()
            + self.text.len()
            + self
                .tool_calls
                .iter()
                .map(|call| call.call_id.len() + call.name.len() + call.arguments.len())
                .sum::<usize>()
            + self.usage.as_ref().map_or(0, String::len)
    }

    fn bytes(&self) -> usize {
        self.request.body.len() + self.response_bytes()
    }

    /// Appends streamed text to `field` unless the call's response is already at its limit.
    fn append(&mut self, text: &str, field: fn(&mut Self) -> &mut String) {
        if self.response_bytes() + text.len() > MAX_RESPONSE_BYTES {
            self.response_truncated = true;
            return;
        }
        field(self).push_str(text);
    }
}

/// The calls captured by one inspector, oldest first.
pub struct CallStore {
    calls: VecDeque<CallRecord>,
    max_calls: usize,
    max_bytes: usize,
}

impl Default for CallStore {
    fn default() -> Self {
        Self::with_limits(MAX_CALLS, MAX_BYTES)
    }
}

impl CallStore {
    pub fn with_limits(max_calls: usize, max_bytes: usize) -> Self {
        Self {
            calls: VecDeque::new(),
            max_calls,
            max_bytes,
        }
    }

    pub fn calls(&self) -> impl DoubleEndedIterator<Item = &CallRecord> {
        self.calls.iter()
    }

    pub fn get(&self, id: u64) -> Option<&CallRecord> {
        self.calls.iter().find(|call| call.request.id == id)
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    pub fn clear(&mut self) {
        self.calls.clear();
    }

    fn get_mut(&mut self, id: u64) -> Option<&mut CallRecord> {
        // Events almost always concern one of the newest calls.
        self.calls
            .iter_mut()
            .rev()
            .find(|call| call.request.id == id)
    }

    /// Applies a captured event. Returns whether the store changed: events of calls it does
    /// not hold (dropped, or cleared away) are ignored.
    pub fn apply(&mut self, event: CaptureEvent) -> bool {
        match event {
            CaptureEvent::Started(request) => {
                self.calls.push_back(CallRecord::new(request));
            }
            CaptureEvent::Text { call, text } => {
                let Some(record) = self.get_mut(call) else {
                    return false;
                };
                record.append(&text, |record| &mut record.text);
            }
            CaptureEvent::Reasoning { call, text } => {
                let Some(record) = self.get_mut(call) else {
                    return false;
                };
                record.append(&text, |record| &mut record.reasoning);
            }
            CaptureEvent::ToolCall {
                call,
                call_id,
                name,
                arguments,
            } => {
                let Some(record) = self.get_mut(call) else {
                    return false;
                };
                let tool_call = ToolCallRecord {
                    call_id,
                    name,
                    arguments,
                };
                match record
                    .tool_calls
                    .iter_mut()
                    .find(|existing| existing.call_id == tool_call.call_id)
                {
                    Some(existing) => *existing = tool_call,
                    None => record.tool_calls.push(tool_call),
                }
            }
            CaptureEvent::Usage { call, usage } => {
                let Some(record) = self.get_mut(call) else {
                    return false;
                };
                record.usage = Some(usage);
            }
            CaptureEvent::Finished { call, at, outcome } => {
                let Some(record) = self.get_mut(call) else {
                    return false;
                };
                record.status = CallStatus::Finished { at, outcome };
            }
        }
        self.enforce_limits();
        true
    }

    /// Drops the oldest calls until the store is within its limits; the newest call is kept
    /// even when it alone is over the byte limit.
    fn enforce_limits(&mut self) {
        let mut bytes: usize = self.calls.iter().map(CallRecord::bytes).sum();
        while self.calls.len() > 1 && (self.calls.len() > self.max_calls || bytes > self.max_bytes)
        {
            if let Some(dropped) = self.calls.pop_front() {
                bytes -= dropped.bytes();
            }
        }
    }

    /// The conversations the held calls belong to, in the order they first appear.
    pub fn conversations(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        self.calls
            .iter()
            .filter_map(|call| call.request.conversation.clone())
            .filter(|conversation| seen.insert(conversation.clone()))
            .collect()
    }
}

/// Which calls an inspector shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallFilter {
    pub hidden_groups: HashSet<CallGroup>,
    /// Show only the calls of this conversation (a root task id).
    pub conversation: Option<String>,
}

impl CallFilter {
    pub fn matches(&self, request: &CapturedRequest) -> bool {
        if self.hidden_groups.contains(&request.kind.group()) {
            return false;
        }
        match &self.conversation {
            Some(conversation) => request.conversation.as_ref() == Some(conversation),
            None => true,
        }
    }

    pub fn toggle_group(&mut self, group: CallGroup) {
        if !self.hidden_groups.remove(&group) {
            self.hidden_groups.insert(group);
        }
    }
}

/// The short form of a conversation id, for labels.
pub fn short_id(id: &str) -> &str {
    match id.char_indices().nth(8) {
        Some((end, _)) => &id[..end],
        None => id,
    }
}

/// A conversation named by its short id. Copied text carries it, so it is built without Fluent
/// placeables, which would wrap the id in invisible isolation marks; the same holds for the
/// other labels below.
pub fn conversation_short_label(id: &str) -> String {
    format!(
        "{} {}",
        crate::t!("byop-inspector-conversation"),
        short_id(id)
    )
}

pub fn outcome_label(status: &CallStatus) -> String {
    match status {
        CallStatus::InProgress => crate::t!("byop-inspector-status-in-progress"),
        CallStatus::Finished { outcome, .. } => match outcome {
            CallOutcome::Completed => crate::t!("byop-inspector-status-completed"),
            CallOutcome::Failed(_) => crate::t!("byop-inspector-status-failed"),
            CallOutcome::Interrupted => crate::t!("byop-inspector-status-interrupted"),
        },
    }
}

/// The one-line summary of a call: number, time, kind, model, conversation and status.
pub fn summary_line(record: &CallRecord) -> String {
    let request = &record.request;
    let mut line = format!(
        "#{} {} · {} · {}",
        request.id,
        request.started_at.format("%H:%M:%S"),
        request.kind.label(),
        request.model_id,
    );
    if let Some(conversation) = &request.conversation {
        let _ = write!(line, " · {}", conversation_short_label(conversation));
    }
    let _ = write!(line, " · {}", outcome_label(&record.status));
    if let CallStatus::Finished { at, .. } = &record.status {
        let millis = (*at - request.started_at).num_milliseconds().max(0);
        let _ = write!(line, " ({:.1}s)", millis as f64 / 1000.0);
    }
    line
}

/// The request part of a call as text: endpoint line and body.
pub fn request_text(record: &CallRecord) -> String {
    let request = &record.request;
    let mut text = String::new();
    if let Some(url) = &request.url {
        let _ = writeln!(text, "POST {url}");
    }
    text.push_str(&request.body);
    text
}

/// The response part of a call as text: reasoning, text, tool calls, usage and error, each
/// under its own heading, leaving out what did not arrive.
pub fn response_text(record: &CallRecord) -> String {
    let mut sections: Vec<(String, &str)> = Vec::new();
    if !record.reasoning.is_empty() {
        sections.push((
            crate::t!("byop-inspector-section-reasoning"),
            &record.reasoning,
        ));
    }
    if !record.text.is_empty() {
        sections.push((crate::t!("byop-inspector-section-text"), &record.text));
    }
    for call in &record.tool_calls {
        sections.push((
            format!(
                "{} {} ({})",
                crate::t!("byop-inspector-section-tool-call"),
                call.name,
                call.call_id
            ),
            &call.arguments,
        ));
    }
    if let Some(usage) = &record.usage {
        sections.push((crate::t!("byop-inspector-section-usage"), usage));
    }
    if let CallStatus::Finished {
        outcome: CallOutcome::Failed(error),
        ..
    } = &record.status
    {
        sections.push((crate::t!("byop-inspector-section-error"), error));
    }

    let mut text = String::new();
    for (heading, body) in sections {
        if !text.is_empty() {
            text.push('\n');
        }
        let _ = writeln!(text, "[{heading}]");
        text.push_str(body);
        if !body.ends_with('\n') {
            text.push('\n');
        }
    }
    if record.response_truncated {
        let _ = writeln!(text, "[{}]", crate::t!("byop-inspector-response-truncated"));
    }
    text
}

/// A whole call as text, the form it is copied in.
pub fn record_text(record: &CallRecord) -> String {
    format!(
        "{}\n\n== {} ==\n{}\n\n== {} ==\n{}",
        summary_line(record),
        crate::t!("byop-inspector-request"),
        request_text(record),
        crate::t!("byop-inspector-response"),
        response_text(record),
    )
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
