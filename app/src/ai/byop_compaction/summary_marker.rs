//! The marker the agent transcript shows in place of a compaction summary.
//!
//! The summary comes back from the model as an ordinary text message, so shown as it is it
//! reads like the agent's answer. The transcript shows a collapsed marker instead, which
//! expands to the summary. The message itself is unchanged: later requests read it from
//! history.

use super::state::{CompactionState, CompactionTrigger};

/// What the marker of a compaction summary says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryMarker {
    /// The summary is still streaming.
    Compacting,
    /// The compaction completed.
    Compacted(CompactionTrigger),
    /// The summary stopped (cancelled or failed) before the compaction completed.
    Stopped,
}

impl SummaryMarker {
    /// The marker for the output message `msg_id`, or `None` if it is not a compaction summary.
    ///
    /// A message is a summary if a completed compaction recorded it as one (this also holds for
    /// a restored conversation), or if it is the answer to a summarize request
    /// (`in_summarize_exchange`), which covers the summary until the compaction completes.
    /// `is_streaming` is whether that answer is still streaming.
    pub fn for_message(
        state: Option<&CompactionState>,
        msg_id: &str,
        in_summarize_exchange: bool,
        is_streaming: bool,
    ) -> Option<Self> {
        if let Some(trigger) = state.and_then(|state| state.summary_trigger(msg_id)) {
            Some(Self::Compacted(trigger))
        } else if !in_summarize_exchange {
            None
        } else if is_streaming {
            Some(Self::Compacting)
        } else {
            Some(Self::Stopped)
        }
    }

    pub fn header_text(self) -> String {
        match self {
            Self::Compacting => crate::t!("ai-block-compaction-summary-compacting"),
            Self::Compacted(CompactionTrigger::Auto) => {
                crate::t!("ai-block-compaction-summary-compacted-auto")
            }
            Self::Compacted(CompactionTrigger::Manual) => {
                crate::t!("ai-block-compaction-summary-compacted")
            }
            Self::Stopped => crate::t!("ai-block-compaction-summary-stopped"),
        }
    }
}

#[cfg(test)]
mod summary_marker_tests {
    use super::super::state::CompletedCompaction;
    use super::*;

    fn state_with_summary(assistant_msg_id: &str, auto: bool) -> CompactionState {
        let mut state = CompactionState::default();
        state.push_completed(CompletedCompaction {
            user_msg_id: "user".to_string(),
            assistant_msg_id: assistant_msg_id.to_string(),
            head_message_ids: Vec::new(),
            tail_start_id: None,
            summary_text: Some("summary".to_string()),
            auto,
            overflow: auto,
        });
        state
    }

    #[test]
    fn completed_compaction_names_its_trigger() {
        let manual = state_with_summary("a1", false);
        let auto = state_with_summary("a1", true);
        assert_eq!(
            SummaryMarker::for_message(Some(&manual), "a1", true, false),
            Some(SummaryMarker::Compacted(CompactionTrigger::Manual))
        );
        // A restored conversation has no summarize request left in the exchange.
        assert_eq!(
            SummaryMarker::for_message(Some(&auto), "a1", false, false),
            Some(SummaryMarker::Compacted(CompactionTrigger::Auto))
        );
    }

    #[test]
    fn summarize_answer_before_the_compaction_completes() {
        let state = CompactionState::default();
        assert_eq!(
            SummaryMarker::for_message(Some(&state), "a1", true, true),
            Some(SummaryMarker::Compacting)
        );
        assert_eq!(
            SummaryMarker::for_message(Some(&state), "a1", true, false),
            Some(SummaryMarker::Stopped)
        );
        assert_eq!(
            SummaryMarker::for_message(None, "a1", true, true),
            Some(SummaryMarker::Compacting)
        );
    }

    #[test]
    fn other_messages_are_not_summaries() {
        let state = state_with_summary("a1", false);
        assert_eq!(
            SummaryMarker::for_message(Some(&state), "a2", false, false),
            None
        );
        assert_eq!(SummaryMarker::for_message(None, "a2", false, true), None);
    }
}
