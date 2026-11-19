use settings::Setting;
use warpui::App;

use super::CustomSecretRegexUpdater;
use crate::settings::PrivacySettings;
use crate::test_util::settings::initialize_settings_for_tests;

#[test]
fn seeds_the_recommended_patterns_on_first_start() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        let privacy_settings = app.add_singleton_model(PrivacySettings::mock);
        privacy_settings.read(&app, |settings, _| {
            assert!(settings.user_secret_regex_list.value().is_empty());
            assert!(!*settings.has_initialized_default_secret_regexes.value());
        });

        app.add_singleton_model(CustomSecretRegexUpdater::new);

        privacy_settings.read(&app, |settings, _| {
            assert!(!settings.user_secret_regex_list.value().is_empty());
            assert!(*settings.has_initialized_default_secret_regexes.value());
        });
    });
}

#[test]
fn leaves_the_list_alone_once_initialized() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        let privacy_settings = app.add_singleton_model(PrivacySettings::mock);
        // What a user who removed every pattern has: the flag set and an empty list.
        privacy_settings.update(&mut app, |settings, ctx| {
            settings.disable_default_regex_trigger(ctx);
        });

        app.add_singleton_model(CustomSecretRegexUpdater::new);

        privacy_settings.read(&app, |settings, _| {
            assert!(settings.user_secret_regex_list.value().is_empty());
        });
    });
}
