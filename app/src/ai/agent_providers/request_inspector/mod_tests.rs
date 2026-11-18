use futures::executor::block_on;
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use serde_json::json;
use serial_test::serial;

use super::*;

fn openai_client() -> Client {
    let resolver = ServiceTargetResolver::from_resolver_fn(
        |target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
            Ok(ServiceTarget {
                endpoint: Endpoint::from_static("https://user:pw@example.test/v1/"),
                auth: AuthData::from_single("sk-not-shown"),
                model: ModelIden::new(AdapterKind::OpenAI, target.model.model_name),
            })
        },
    );
    Client::builder()
        .with_service_target_resolver(resolver)
        .build()
}

fn begin_title_call(client: &Client) -> Option<CallCapture> {
    let chat_req = ChatRequest::from_messages(vec![ChatMessage::user("name this")])
        .with_system("you write titles");
    block_on(begin(
        CallOrigin::new(CallKind::Title).in_conversation("root-task"),
        client,
        "test-model",
        &chat_req,
        &ChatOptions::default().with_temperature(0.5),
        false,
    ))
}

#[test]
#[serial]
fn nothing_is_captured_without_a_listener() {
    assert!(!is_listening());
    assert!(begin_title_call(&openai_client()).is_none());
}

#[test]
#[serial]
fn a_listener_receives_the_request_as_sent_and_its_response() {
    let client = openai_client();
    let (subscription, receiver) = subscribe();
    assert!(is_listening());

    let capture = begin_title_call(&client).expect("captured while listening");
    capture.text("A title");
    capture.complete();

    let Ok(CaptureEvent::Started(request)) = receiver.try_recv() else {
        panic!("the request comes first");
    };
    assert_eq!(request.kind, CallKind::Title);
    assert_eq!(request.conversation.as_deref(), Some("root-task"));
    assert_eq!(request.model_id, "test-model");
    // The endpoint loses its user info; the key is in neither the URL nor the body.
    assert_eq!(
        request.url.as_deref(),
        Some("https://example.test/v1/chat/completions")
    );
    assert!(request.body.contains("you write titles"));
    assert!(request.body.contains("\"temperature\": 0.5"));
    assert!(!request.body.contains("sk-not-shown"));

    assert!(matches!(
        receiver.try_recv(),
        Ok(CaptureEvent::Text { text, .. }) if text == "A title"
    ));
    assert!(matches!(
        receiver.try_recv(),
        Ok(CaptureEvent::Finished {
            outcome: CallOutcome::Completed,
            ..
        })
    ));

    drop(subscription);
    assert!(!is_listening());
    assert!(begin_title_call(&client).is_none());
}

#[test]
#[serial]
fn a_dropped_capture_is_reported_as_interrupted() {
    let (_subscription, receiver) = subscribe();
    drop(begin_title_call(&openai_client()));

    assert!(matches!(receiver.try_recv(), Ok(CaptureEvent::Started(_))));
    assert!(matches!(
        receiver.try_recv(),
        Ok(CaptureEvent::Finished {
            outcome: CallOutcome::Interrupted,
            ..
        })
    ));
}

#[test]
fn inline_binary_data_is_elided_from_the_body() {
    let image = "A".repeat(4096);
    let mut payload = json!({
        "messages": [{
            "content": [
                { "type": "text", "text": "short text stays" },
                { "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{image}") } },
                { "type": "input_file", "data": image },
            ]
        }],
        "prose": format!("{} with spaces", "word ".repeat(200)),
    });
    elide_binary_data(&mut payload);

    let content = &payload["messages"][0]["content"];
    assert_eq!(content[0]["text"], "short text stays");
    let url = content[1]["image_url"]["url"].as_str().unwrap();
    assert!(url.starts_with("data:image/png;base64,<"), "{url}");
    assert!(!url.contains("AAAA"));
    assert!(!content[2]["data"].as_str().unwrap().contains("AAAA"));
    assert!(payload["prose"].as_str().unwrap().starts_with("word word"));
}

#[test]
fn display_url_hides_credentials() {
    assert_eq!(
        display_url("https://user:secret@host.test/v1/chat?api-version=1&key=abc#frag"),
        "https://host.test/v1/chat?api-version=…&key=…"
    );
    assert_eq!(
        display_url("http://localhost:11434/api/chat"),
        "http://localhost:11434/api/chat"
    );
    assert_eq!(display_url("not a url"), "not a url");
}

#[test]
fn every_kind_belongs_to_a_group() {
    for kind in [
        CallKind::AgentTurn,
        CallKind::Title,
        CallKind::PromptSuggestions,
        CallKind::InputPrediction,
        CallKind::RelevantFiles,
        CallKind::WorkflowMetadata,
        CallKind::NextCommand,
        CallKind::Other,
    ] {
        assert!(CallGroup::ALL.contains(&kind.group()));
    }
}
