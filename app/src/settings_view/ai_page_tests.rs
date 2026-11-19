use super::{AISettingsPageAction, RedactedString};

const API_KEY: &str = "sk-test-0123456789abcdef";
const HEADER_VALUE: &str = "Bearer header-secret-value";

fn headers() -> Vec<(String, RedactedString)> {
    vec![(
        "Authorization".to_string(),
        RedactedString::new(HEADER_VALUE),
    )]
}

/// What the log gets when the action is dispatched.
fn logged(action: &AISettingsPageAction) -> String {
    format!("{action:?}")
}

fn assert_no_secret(action: &AISettingsPageAction) {
    let logged = logged(action);
    assert!(!logged.contains(API_KEY), "API key in {logged}");
    assert!(!logged.contains(HEADER_VALUE), "header value in {logged}");
    assert!(logged.contains("[REDACTED]"), "{logged}");
}

#[test]
fn redacted_string_keeps_its_value() {
    let secret = RedactedString::new(API_KEY);
    assert_eq!(secret.expose(), API_KEY);
    assert_eq!(format!("{secret:?}"), "[REDACTED]");
    assert_eq!(format!("{:?}", RedactedString::default()), "[REDACTED]");
}

#[test]
fn update_api_key_action_hides_the_key() {
    assert_no_secret(&AISettingsPageAction::UpdateAgentProviderApiKey {
        provider_id: "provider".to_string(),
        api_key: RedactedString::new(API_KEY),
    });
}

#[test]
fn update_header_action_hides_the_value() {
    let action = AISettingsPageAction::UpdateAgentProviderHeader {
        provider_id: "provider".to_string(),
        header_index: 0,
        key: "Authorization".to_string(),
        value: RedactedString::new(HEADER_VALUE),
    };
    assert_no_secret(&action);
    // The header's name is not a secret and stays readable.
    assert!(logged(&action).contains("Authorization"));
}

#[test]
fn save_edits_action_hides_the_key_and_header_values() {
    let action = AISettingsPageAction::SaveAgentProviderEdits {
        provider_id: "provider".to_string(),
        name: "My provider".to_string(),
        base_url: "https://example.com/v1".to_string(),
        api_key: RedactedString::new(API_KEY),
        headers: headers(),
        models: vec![(0, "Model".to_string(), "model-id".to_string(), 1000, 100)],
    };
    assert_no_secret(&action);
    let logged = logged(&action);
    assert!(logged.contains("https://example.com/v1"), "{logged}");
    assert!(logged.contains("Authorization"), "{logged}");
}

#[test]
fn save_edits_then_action_hides_the_secrets_of_both_actions() {
    assert_no_secret(&AISettingsPageAction::SaveAgentProviderEditsThen {
        provider_id: "provider".to_string(),
        name: "My provider".to_string(),
        base_url: "https://example.com/v1".to_string(),
        api_key: RedactedString::new(API_KEY),
        headers: headers(),
        models: Vec::new(),
        action: Box::new(AISettingsPageAction::UpdateAgentProviderApiKey {
            provider_id: "provider".to_string(),
            api_key: RedactedString::new(API_KEY),
        }),
    });
}
