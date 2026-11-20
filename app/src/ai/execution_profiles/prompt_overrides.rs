//! Per-profile choices of where each prompt comes from.
//!
//! Every prompt a profile sends to a BYOP model has a slot here. A slot left on
//! [`PromptChoice::Auto`] keeps the built-in prompt; it can instead name another built-in
//! template (only the agent slot has more than one) or a file in the custom prompts directory
//! (see `agent_providers::custom_prompts`). Resolving a choice, and falling back when it
//! can't be used, is the prompt renderer's job: nothing here touches the disk.

use serde::{Deserialize, Serialize};

/// A prompt whose source a profile can choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptSlot {
    /// The system prompt of an agent conversation.
    Agent,
    /// The system prompt that names a new conversation.
    Title,
    /// The system prompt that suggests asking the agent after a command finishes.
    PromptSuggestions,
    /// The system prompt that completes what is typed into the agent input.
    InputCompletion,
    /// The system prompt that picks the files relevant to a query.
    RelevantFiles,
    /// The system prompt that fills in a workflow's metadata from its command.
    WorkflowMetadata,
    /// The system prompt that predicts the next shell command.
    NextCommand,
}

impl PromptSlot {
    /// Every slot, in the order the profile editor shows them.
    pub const ALL: [PromptSlot; 7] = [
        PromptSlot::Agent,
        PromptSlot::Title,
        PromptSlot::PromptSuggestions,
        PromptSlot::InputCompletion,
        PromptSlot::RelevantFiles,
        PromptSlot::WorkflowMetadata,
        PromptSlot::NextCommand,
    ];

    /// Whether the slot can be pointed at another built-in template. The agent prompt has one
    /// built-in per model family; every other slot has a single built-in, which is its Auto.
    pub fn has_builtin_alternatives(self) -> bool {
        match self {
            PromptSlot::Agent => true,
            PromptSlot::Title
            | PromptSlot::PromptSuggestions
            | PromptSlot::InputCompletion
            | PromptSlot::RelevantFiles
            | PromptSlot::WorkflowMetadata
            | PromptSlot::NextCommand => false,
        }
    }
}

const BUILTIN_PREFIX: &str = "builtin:";
const CUSTOM_PREFIX: &str = "custom:";

/// Where the prompt of one slot comes from.
///
/// Stored as a string (`builtin:<name>`, `custom:<path>`) so that a value this version doesn't
/// understand, from a newer ZapNG or a hand-edited profile, reads as [`PromptChoice::Auto`]
/// instead of failing to load the whole profile.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum PromptChoice {
    /// The built-in prompt; for the agent slot, the template picked by the model's family.
    #[default]
    Auto,
    /// A built-in agent template by name: `anthropic` renders `system/anthropic.j2`.
    Builtin(String),
    /// A file in the custom prompts directory, by its path relative to that directory.
    Custom(String),
}

impl PromptChoice {
    pub fn is_auto(&self) -> bool {
        matches!(self, PromptChoice::Auto)
    }
}

impl From<String> for PromptChoice {
    fn from(value: String) -> Self {
        if value.is_empty() {
            return PromptChoice::Auto;
        }
        if let Some(name) = value.strip_prefix(BUILTIN_PREFIX) {
            return PromptChoice::Builtin(name.to_owned());
        }
        if let Some(path) = value.strip_prefix(CUSTOM_PREFIX) {
            return PromptChoice::Custom(path.to_owned());
        }
        log::warn!("[prompt overrides] unrecognized prompt choice {value:?}; using Auto");
        PromptChoice::Auto
    }
}

impl From<PromptChoice> for String {
    fn from(value: PromptChoice) -> Self {
        match value {
            PromptChoice::Auto => String::new(),
            PromptChoice::Builtin(name) => format!("{BUILTIN_PREFIX}{name}"),
            PromptChoice::Custom(path) => format!("{CUSTOM_PREFIX}{path}"),
        }
    }
}

/// The prompt choices of one profile. Slots on Auto are left out when serialized, so a profile
/// that overrides nothing serializes exactly as it did before overrides existed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptOverrides {
    #[serde(skip_serializing_if = "PromptChoice::is_auto")]
    agent: PromptChoice,
    #[serde(skip_serializing_if = "PromptChoice::is_auto")]
    title: PromptChoice,
    #[serde(skip_serializing_if = "PromptChoice::is_auto")]
    prompt_suggestions: PromptChoice,
    #[serde(skip_serializing_if = "PromptChoice::is_auto")]
    input_completion: PromptChoice,
    #[serde(skip_serializing_if = "PromptChoice::is_auto")]
    relevant_files: PromptChoice,
    #[serde(skip_serializing_if = "PromptChoice::is_auto")]
    workflow_metadata: PromptChoice,
    #[serde(skip_serializing_if = "PromptChoice::is_auto")]
    next_command: PromptChoice,
}

impl PromptOverrides {
    pub fn get(&self, slot: PromptSlot) -> &PromptChoice {
        match slot {
            PromptSlot::Agent => &self.agent,
            PromptSlot::Title => &self.title,
            PromptSlot::PromptSuggestions => &self.prompt_suggestions,
            PromptSlot::InputCompletion => &self.input_completion,
            PromptSlot::RelevantFiles => &self.relevant_files,
            PromptSlot::WorkflowMetadata => &self.workflow_metadata,
            PromptSlot::NextCommand => &self.next_command,
        }
    }

    /// Sets the choice of `slot`, returning whether it changed.
    pub fn set(&mut self, slot: PromptSlot, choice: PromptChoice) -> bool {
        let current = match slot {
            PromptSlot::Agent => &mut self.agent,
            PromptSlot::Title => &mut self.title,
            PromptSlot::PromptSuggestions => &mut self.prompt_suggestions,
            PromptSlot::InputCompletion => &mut self.input_completion,
            PromptSlot::RelevantFiles => &mut self.relevant_files,
            PromptSlot::WorkflowMetadata => &mut self.workflow_metadata,
            PromptSlot::NextCommand => &mut self.next_command,
        };
        if *current == choice {
            return false;
        }
        *current = choice;
        true
    }

    pub fn is_empty(&self) -> bool {
        PromptSlot::ALL.iter().all(|slot| self.get(*slot).is_auto())
    }
}

/// The choices the profile editor offers for `slot`, in menu order: Auto, the other built-in
/// templates (agent slot only), then the custom files found on disk. A current choice that is
/// no longer available (a deleted file, a built-in this version doesn't have) is kept at the
/// end, so the menu still shows what the profile holds.
pub fn menu_choices(
    slot: PromptSlot,
    builtin_templates: &[&str],
    custom_files: &[String],
    current: &PromptChoice,
) -> Vec<PromptChoice> {
    let mut choices = vec![PromptChoice::Auto];
    if slot.has_builtin_alternatives() {
        choices.extend(
            builtin_templates
                .iter()
                .map(|name| PromptChoice::Builtin((*name).to_owned())),
        );
    }
    choices.extend(
        custom_files
            .iter()
            .map(|path| PromptChoice::Custom(path.clone())),
    );
    if !choices.contains(current) {
        choices.push(current.clone());
    }
    choices
}

#[cfg(test)]
#[path = "prompt_overrides_tests.rs"]
mod tests;
