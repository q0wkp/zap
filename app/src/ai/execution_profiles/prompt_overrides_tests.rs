use super::*;
use crate::ai::execution_profiles::AIExecutionProfile;

#[test]
fn choice_round_trips_through_its_string_form() {
    for choice in [
        PromptChoice::Auto,
        PromptChoice::Builtin("anthropic".to_owned()),
        PromptChoice::Custom("team/agent.j2".to_owned()),
    ] {
        let json = serde_json::to_string(&choice).unwrap();
        let back: PromptChoice = serde_json::from_str(&json).unwrap();
        assert_eq!(back, choice, "{json}");
    }
}

#[test]
fn unrecognized_choice_reads_as_auto() {
    let choice: PromptChoice = serde_json::from_str("\"somethingnew:value\"").unwrap();
    assert_eq!(choice, PromptChoice::Auto);
}

#[test]
fn set_reports_whether_the_choice_changed() {
    let mut overrides = PromptOverrides::default();
    assert!(overrides.is_empty());

    let custom = PromptChoice::Custom("title.md".to_owned());
    assert!(overrides.set(PromptSlot::Title, custom.clone()));
    assert!(!overrides.set(PromptSlot::Title, custom.clone()));
    assert_eq!(overrides.get(PromptSlot::Title), &custom);
    assert_eq!(overrides.get(PromptSlot::Agent), &PromptChoice::Auto);
    assert!(!overrides.is_empty());

    assert!(overrides.set(PromptSlot::Title, PromptChoice::Auto));
    assert!(overrides.is_empty());
}

#[test]
fn every_slot_is_stored_separately() {
    let mut overrides = PromptOverrides::default();
    for (i, slot) in PromptSlot::ALL.iter().enumerate() {
        overrides.set(*slot, PromptChoice::Custom(format!("{i}.j2")));
    }
    for (i, slot) in PromptSlot::ALL.iter().enumerate() {
        assert_eq!(
            overrides.get(*slot),
            &PromptChoice::Custom(format!("{i}.j2"))
        );
    }
}

#[test]
fn profile_without_overrides_serializes_as_before() {
    let profile = AIExecutionProfile::default();
    let json = serde_json::to_value(&profile).unwrap();
    assert!(json.get("prompt_overrides").is_none(), "{json}");
}

#[test]
fn profile_saved_before_overrides_existed_loads_with_auto_everywhere() {
    let mut json = serde_json::to_value(AIExecutionProfile::default()).unwrap();
    json.as_object_mut().unwrap().remove("prompt_overrides");
    let profile: AIExecutionProfile = serde_json::from_value(json).unwrap();
    assert!(profile.prompt_overrides.is_empty());
}

#[test]
fn profile_overrides_round_trip_and_skip_auto_slots() {
    let mut profile = AIExecutionProfile::default();
    profile
        .prompt_overrides
        .set(PromptSlot::Agent, PromptChoice::Builtin("local".to_owned()));
    profile.prompt_overrides.set(
        PromptSlot::NextCommand,
        PromptChoice::Custom("next.j2".to_owned()),
    );

    let json = serde_json::to_value(&profile).unwrap();
    assert_eq!(
        json["prompt_overrides"],
        serde_json::json!({ "agent": "builtin:local", "next_command": "custom:next.j2" })
    );
    let back: AIExecutionProfile = serde_json::from_value(json).unwrap();
    assert_eq!(back.prompt_overrides, profile.prompt_overrides);
}

#[test]
fn menu_offers_builtins_only_for_the_agent_slot() {
    let builtins = ["default", "anthropic"];
    let files = vec!["mine.j2".to_owned()];

    assert_eq!(
        menu_choices(PromptSlot::Agent, &builtins, &files, &PromptChoice::Auto),
        vec![
            PromptChoice::Auto,
            PromptChoice::Builtin("default".to_owned()),
            PromptChoice::Builtin("anthropic".to_owned()),
            PromptChoice::Custom("mine.j2".to_owned()),
        ]
    );
    assert_eq!(
        menu_choices(PromptSlot::Title, &builtins, &files, &PromptChoice::Auto),
        vec![
            PromptChoice::Auto,
            PromptChoice::Custom("mine.j2".to_owned()),
        ]
    );
}

#[test]
fn menu_keeps_a_current_choice_that_is_no_longer_available() {
    let missing = PromptChoice::Custom("deleted.j2".to_owned());
    let choices = menu_choices(PromptSlot::NextCommand, &[], &[], &missing);
    assert_eq!(choices, vec![PromptChoice::Auto, missing]);
}
