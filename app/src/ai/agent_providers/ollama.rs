//! "Fetch from API" for Ollama providers, through Ollama's native API.
//!
//! The OpenAI-compatible `/models` list carries model ids only, and Ollama serves it under
//! `/v1`, while an Ollama provider's base URL is the server root that `chat_stream` sends
//! `api/chat` to. So an Ollama provider lists its models with `GET api/tags` and asks
//! `POST api/show` for each model's context window.

use std::collections::HashMap;
use std::time::Duration;

use futures::StreamExt;
use http_client::Client;
use serde::Deserialize;

use super::openai_compatible::{normalize_base_url, OpenAiCompatibleError};
use crate::settings::AgentProviderApiType;

/// How many `api/show` requests run at once. Ollama is usually one local process.
const SHOW_CONCURRENCY: usize = 4;
const SHOW_TIMEOUT: Duration = Duration::from_secs(15);

/// A model an Ollama server has, with its context window when the server reports one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OllamaModel {
    pub id: String,
    pub context_window: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    #[serde(default)]
    models: Vec<TagsEntry>,
}

#[derive(Debug, Deserialize)]
struct TagsEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    model: String,
}

#[derive(Debug, Default, Deserialize)]
struct ShowResponse {
    /// The Modelfile's `PARAMETER` lines, one `<name> <value>` a line.
    #[serde(default)]
    parameters: Option<String>,
    /// GGUF metadata, keyed `<architecture>.<field>`.
    #[serde(default)]
    model_info: HashMap<String, serde_json::Value>,
}

/// The sorted, deduplicated model ids of an `api/tags` response.
fn model_ids(tags: TagsResponse) -> Vec<String> {
    let mut ids: Vec<String> = tags
        .models
        .into_iter()
        .map(|entry| {
            if entry.model.is_empty() {
                entry.name
            } else {
                entry.model
            }
        })
        .filter(|id| !id.trim().is_empty())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// The context window an `api/show` response reports. The Modelfile's `num_ctx` comes
/// first: it is the window the server runs the model with, which can be smaller than the
/// length the model was trained for (`<architecture>.context_length`).
fn context_window(show: &ShowResponse) -> Option<u32> {
    let num_ctx = show.parameters.as_deref().and_then(|parameters| {
        parameters.lines().find_map(|line| {
            let mut fields = line.split_whitespace();
            match (fields.next(), fields.next()) {
                (Some("num_ctx"), Some(value)) => value.parse::<u32>().ok(),
                _ => None,
            }
        })
    });
    num_ctx
        .filter(|n| *n > 0)
        .or_else(|| trained_context_length(&show.model_info))
}

/// `<architecture>.context_length` from the model metadata, the architecture taken from
/// `general.architecture`; failing that, any `*.context_length` key.
fn trained_context_length(model_info: &HashMap<String, serde_json::Value>) -> Option<u32> {
    let as_u32 = |value: &serde_json::Value| value.as_u64().and_then(|n| u32::try_from(n).ok());
    let by_architecture = model_info
        .get("general.architecture")
        .and_then(|arch| arch.as_str())
        .and_then(|arch| model_info.get(&format!("{arch}.context_length")))
        .and_then(as_u32);
    by_architecture
        .or_else(|| {
            model_info
                .iter()
                .filter(|(key, _)| key.ends_with(".context_length"))
                .find_map(|(_, value)| as_u32(value))
        })
        .filter(|n| *n > 0)
}

/// The server root that Ollama's native API hangs off, with a trailing `/`, the same way
/// `chat_stream` derives it for `api/chat`.
fn api_root(base_url: &str) -> Result<String, OpenAiCompatibleError> {
    let base = normalize_base_url(base_url)?;
    Ok(super::chat_stream::normalize_endpoint_url(
        AgentProviderApiType::Ollama,
        &base,
    ))
}

async fn show_context_window(
    client: &Client,
    root: &str,
    api_key: Option<&str>,
    model_id: &str,
) -> Option<u32> {
    // `name` is what Ollama versions before `model` was introduced read.
    let mut request = client
        .post(format!("{root}api/show"))
        .json(&serde_json::json!({ "model": model_id, "name": model_id }))
        .timeout(SHOW_TIMEOUT);
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        request = request.bearer_auth(key);
    }
    let response = match request.send().await {
        Ok(response) => response,
        Err(e) => {
            log::warn!("[ollama] api/show for {model_id} failed: {e}");
            return None;
        }
    };
    if !response.status().is_success() {
        log::warn!(
            "[ollama] api/show for {model_id} returned HTTP {}",
            response.status()
        );
        return None;
    }
    match response.json::<ShowResponse>().await {
        Ok(show) => context_window(&show),
        Err(e) => {
            log::warn!("[ollama] api/show for {model_id} didn't parse: {e}");
            None
        }
    }
}

/// Lists the models of an Ollama server with their context windows. A model whose
/// `api/show` fails is still listed, without a context window.
pub async fn fetch_ollama_models(
    client: Client,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<OllamaModel>, OpenAiCompatibleError> {
    let root = api_root(base_url)?;

    let mut request = client.get(format!("{root}api/tags"));
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        request = request.bearer_auth(key);
    }
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(OpenAiCompatibleError::Status {
            status: status.as_u16(),
            body,
        });
    }
    let tags: TagsResponse = response
        .json()
        .await
        .map_err(|e| OpenAiCompatibleError::Decode(e.to_string()))?;

    let client = &client;
    let root = root.as_str();
    let models: Vec<OllamaModel> = futures::stream::iter(model_ids(tags))
        .map(|id| async move {
            let context_window = show_context_window(client, root, api_key, &id).await;
            OllamaModel { id, context_window }
        })
        .buffered(SHOW_CONCURRENCY)
        .collect()
        .await;
    Ok(models)
}

#[cfg(test)]
#[path = "ollama_tests.rs"]
mod tests;
