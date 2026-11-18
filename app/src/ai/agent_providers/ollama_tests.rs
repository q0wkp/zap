use super::*;

fn show(json: &str) -> ShowResponse {
    serde_json::from_str(json).expect("fixture should parse")
}

#[test]
fn num_ctx_wins_over_the_trained_length() {
    let response = show(
        r#"{
            "parameters": "num_ctx 8192\nstop \"<|im_end|>\"",
            "model_info": {"general.architecture": "qwen2", "qwen2.context_length": 32768}
        }"#,
    );
    assert_eq!(context_window(&response), Some(8192));
}

#[test]
fn trained_length_of_the_model_architecture() {
    let response = show(
        r#"{
            "parameters": "stop \"<|eot_id|>\"",
            "model_info": {
                "general.architecture": "llama",
                "llama.context_length": 131072,
                "clip.context_length": 77
            }
        }"#,
    );
    assert_eq!(context_window(&response), Some(131072));
}

#[test]
fn any_context_length_key_without_an_architecture() {
    let response = show(r#"{"model_info": {"gptoss.context_length": 131072}}"#);
    assert_eq!(context_window(&response), Some(131072));
}

#[test]
fn nothing_usable_gives_no_window() {
    assert_eq!(context_window(&show("{}")), None);
    assert_eq!(
        context_window(&show(
            r#"{"parameters": "num_ctx 0", "model_info": {"llama.rope.freq_base": 500000}}"#
        )),
        None
    );
    assert_eq!(
        context_window(&show(r#"{"parameters": "num_ctx lots"}"#)),
        None
    );
}

#[test]
fn tags_give_sorted_unique_ids() {
    let tags: TagsResponse = serde_json::from_str(
        r#"{"models": [
            {"name": "qwen3:8b", "model": "qwen3:8b", "size": 1},
            {"name": "llama3.2:latest", "model": "llama3.2:latest"},
            {"name": "only-name:latest"},
            {"name": "qwen3:8b", "model": "qwen3:8b"},
            {"name": "", "model": ""}
        ]}"#,
    )
    .expect("fixture should parse");
    assert_eq!(
        model_ids(tags),
        vec!["llama3.2:latest", "only-name:latest", "qwen3:8b"]
    );
}

#[test]
fn api_root_is_the_server_root() {
    assert_eq!(
        api_root("http://localhost:11434").unwrap(),
        "http://localhost:11434/"
    );
    assert_eq!(
        api_root("http://localhost:11434/v1/").unwrap(),
        "http://localhost:11434/"
    );
    assert_eq!(
        api_root("https://box.example/ollama").unwrap(),
        "https://box.example/ollama/"
    );
    assert!(api_root("").is_err());
    assert!(api_root("localhost:11434").is_err());
}
