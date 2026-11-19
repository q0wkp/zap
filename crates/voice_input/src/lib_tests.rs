use warpui::App;

use super::{StartListeningError, VoiceInput, VoiceInputState, VoiceInputToggledFrom};

#[test]
fn recorder_rejects_a_new_session_while_transcribing() {
    App::test((), |mut app| async move {
        let voice_input = app.add_model(VoiceInput::new);
        voice_input.update(&mut app, |voice_input, ctx| {
            voice_input.state = VoiceInputState::Transcribing;
            assert!(matches!(
                voice_input.start_listening(ctx, VoiceInputToggledFrom::Button),
                Err(StartListeningError::AlreadyRunning)
            ));
        });
    });
}
