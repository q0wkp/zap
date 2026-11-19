use crate::settings::{CustomSecretRegex, PrivacySettings, PrivacySettingsChangedEvent};
use crate::terminal::model::set_user_and_enterprise_secret_regexes;
use warpui::{Entity, ModelContext, SingletonEntity};

/// Dummy singleton model that is used to update the current set of custom regexes within the
/// terminal model. We do this via a singleton model since we only want to do this once any time
/// the custom secret regex list changes, which must be done independent of any view.
pub struct CustomSecretRegexUpdater;

impl CustomSecretRegexUpdater {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let updater = CustomSecretRegexUpdater;
        let privacy_settings = PrivacySettings::handle(ctx);
        // Seed the recommended patterns on the first start. Nothing else does: the settings are
        // no longer fetched from a server, which is where this used to happen. The persisted
        // flag makes this a no-op from then on, so a list the user edited or emptied stays as
        // it is.
        privacy_settings.update(ctx, |settings, ctx| {
            settings.initialize_default_regexes_once(ctx);
        });
        // Initialize with the current custom regexes.
        updater.update_custom_secret_regex_list(ctx);

        ctx.subscribe_to_model(&privacy_settings, |me, evt, ctx| {
            if let PrivacySettingsChangedEvent::CustomSecretRegexList { .. } = evt {
                me.update_custom_secret_regex_list(ctx);
            }
        });
        updater
    }

    fn update_custom_secret_regex_list(&self, ctx: &mut ModelContext<Self>) {
        let privacy_settings = PrivacySettings::as_ref(ctx);

        // Get enterprise and user secrets separately
        let enterprise_secrets = privacy_settings
            .enterprise_secret_regex_list
            .iter()
            .map(CustomSecretRegex::pattern);

        let user_secrets = privacy_settings
            .user_secret_regex_list
            .iter()
            .map(CustomSecretRegex::pattern);

        set_user_and_enterprise_secret_regexes(user_secrets, enterprise_secrets);

        // Zap(Wave1-S4):原 telemetry-side `update_telemetry_secrets_regex` 调用
        // 已随 `server/telemetry/secret_redaction.rs` 整体删除。安全模式的视觉模糊
        // 走 `set_user_and_enterprise_secret_regexes` 已完整覆盖;telemetry-side
        // defence-in-depth 的 redact 因不再有任何外发路径而失去意义。
    }
}

impl Entity for CustomSecretRegexUpdater {
    type Event = ();
}

impl SingletonEntity for CustomSecretRegexUpdater {}

#[cfg(test)]
#[path = "secret_regex_updater_tests.rs"]
mod tests;
