// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

//! Explaining a shell command the agent asks to run, so that the user can decide whether to
//! approve it.
//!
//! The explanation is a one-shot request to a BYOP model, separate from the conversation: the
//! agent never sees the question or the answer, and nothing is approved or run by it.

use anyhow::anyhow;
use zap_ng_ui::{AppContext, SingletonEntity as _};

use super::oneshot::{
    byop_oneshot_completion, resolve_active_ai_oneshot, OneshotConfig, OneshotOptions,
};
use super::request_inspector::CallKind;
use crate::ai::llms::{LLMId, LLMPreferences};
use crate::terminal::safe_mode_settings::get_secret_obfuscation_mode;

/// The most characters of a command that are sent to the model.
const MAX_COMMAND_CHARS: usize = 12_000;

/// How dangerous the model judges a command to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRisk {
    Low,
    Medium,
    High,
}

/// A model's explanation of a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandExplanation {
    /// The risk the model gave, or `None` when its reply did not start with one.
    pub risk: Option<CommandRisk>,
    /// The reason given with the risk, if any.
    pub risk_reason: Option<String>,
    /// The rest of the reply: what each part of the command does.
    pub details: String,
}

/// The model that explains a command an exchange asked for: the exchange's own model when it is
/// a BYOP model, otherwise the active-AI model. `None` when neither is.
pub fn resolve_model(app: &AppContext, exchange_model: Option<&LLMId>) -> Option<OneshotConfig> {
    let Some((provider, api_key, model_id)) =
        exchange_model.and_then(|id| super::lookup_byop(app, id))
    else {
        return resolve_active_ai_oneshot(app, None);
    };
    let reasoning_effort =
        LLMPreferences::as_ref(app).get_reasoning_effort(None, provider.api_type, &model_id);
    Some(OneshotConfig {
        base_url: provider.base_url,
        api_key,
        model_id,
        api_type: provider.api_type,
        reasoning_effort,
        redact_secrets: get_secret_obfuscation_mode(app).should_redact_secret(),
    })
}

/// Asks the model to explain `command`, replying in `language`.
pub async fn explain(
    cfg: OneshotConfig,
    command: String,
    language: &'static str,
) -> anyhow::Result<CommandExplanation> {
    let system =
        include_str!("prompts/tasks/explain_command_system.md").replace("{{language}}", language);
    let user = user_prompt(&command);
    let opts = OneshotOptions {
        // Leave room for the text around the command, so that only the command is cut.
        max_chars: Some(user.chars().count()),
        temperature: Some(0.2),
        kind: CallKind::ExplainCommand,
        ..Default::default()
    };
    let reply = byop_oneshot_completion(&cfg, &system, &user, &opts).await?;
    parse_reply(&reply).ok_or_else(|| anyhow!("the model returned an empty reply"))
}

fn user_prompt(command: &str) -> String {
    let shown: String = command.chars().take(MAX_COMMAND_CHARS).collect();
    let note = if shown.len() < command.len() {
        "\nThe command was cut short here: the rest is not shown."
    } else {
        ""
    };
    format!("Explain this command:\n<command>\n{shown}\n</command>{note}")
}

fn parse_reply(reply: &str) -> Option<CommandExplanation> {
    let reply = strip_reasoning(reply);
    let mut lines = reply.lines().skip_while(|line| line.trim().is_empty());
    let first_line = lines.next()?.trim();

    let Some((risk, risk_reason)) = parse_risk_line(first_line) else {
        return Some(CommandExplanation {
            risk: None,
            risk_reason: None,
            details: reply.trim().to_owned(),
        });
    };
    let details = lines.collect::<Vec<_>>().join("\n").trim().to_owned();
    Some(CommandExplanation {
        risk: Some(risk),
        risk_reason,
        details,
    })
}

/// Parses `RISK: HIGH - reason`, allowing Markdown emphasis and other separators.
fn parse_risk_line(line: &str) -> Option<(CommandRisk, Option<String>)> {
    let line = line.trim_matches(|c: char| c == '*' || c == '_' || c.is_whitespace());
    let (keyword, rest) = line.split_at_checked(4)?;
    if !keyword.eq_ignore_ascii_case("risk") {
        return None;
    }
    let rest = rest
        .trim_start_matches(|c: char| c == '*' || c == ':' || c.is_whitespace())
        .trim_start();
    let level_end = rest
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(rest.len());
    let (level, reason) = rest.split_at(level_end);
    let risk = match level.to_ascii_uppercase().as_str() {
        "LOW" => CommandRisk::Low,
        "MEDIUM" => CommandRisk::Medium,
        "HIGH" => CommandRisk::High,
        _ => return None,
    };
    let reason = reason
        .trim_start_matches(|c: char| {
            c == '*'
                || c == '-'
                || c == ':'
                || c == '\u{2013}'
                || c == '\u{2014}'
                || c.is_whitespace()
        })
        .trim_end_matches(|c: char| c == '*' || c.is_whitespace());
    Some((risk, (!reason.is_empty()).then(|| reason.to_owned())))
}

/// Removes the reasoning blocks some models put before their answer.
fn strip_reasoning(reply: &str) -> String {
    let mut reply = reply.to_owned();
    for tag in ["think", "thinking", "reasoning"] {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        while let Some(start) = reply.find(&open) {
            let Some(end) = reply[start..].find(&close) else {
                // An unclosed block is all reasoning.
                reply.truncate(start);
                break;
            };
            reply.replace_range(start..start + end + close.len(), "");
        }
    }
    reply
}

#[cfg(test)]
#[path = "explain_command_tests.rs"]
mod tests;
