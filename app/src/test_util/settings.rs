#[cfg(test)]
use zap_ng_ui::App;

#[cfg(test)]
pub fn initialize_settings_for_tests(app: &mut App) {
    use zap_ng_core::execution_mode::ExecutionMode;
    initialize_settings_for_tests_with_mode(app, ExecutionMode::App, false);
}

#[cfg(test)]
pub fn initialize_settings_for_tests_with_mode(
    app: &mut App,
    mode: zap_ng_core::execution_mode::ExecutionMode,
    is_sandboxed: bool,
) {
    use crate::{
        drive::settings::ZapNGDriveSettings,
        search::command_search::settings::CommandSearchSettings,
        settings::{
            app_icon::AppIconSettings, init_and_register_user_preferences,
            manager::SettingsManager, AISettings, AccessibilitySettings, AliasExpansionSettings,
            AppEditorSettings, BlockVisibilitySettings, CodeSettings, DebugSettings,
            EmacsBindingsSettings, FontSettings, GPUSettings, InputModeSettings, InputSettings,
            NativePreferenceSettings, PaneSettings, PreferencesSettings,
            SameLinePromptBlockSettings, ScrollSettings, SelectionSettings, SshSettings,
            ThemeSettings, VimBannerSettings,
        },
        terminal::{
            general_settings::GeneralSettings, keys_settings::KeysSettings,
            ligature_settings::LigatureSettings, safe_mode_settings::SafeModeSettings,
            session_settings::SessionSettings, settings::TerminalSettings,
            shared_session::settings::SharedSessionSettings, zapngify::settings::ZapNGifySettings,
            BlockListSettings,
        },
        undo_close::UndoCloseSettings,
        user_config::ZapNGConfig,
        window_settings::WindowSettings,
        workspace::tab_settings::TabSettings,
    };
    use zap_ng_core::{execution_mode::AppExecutionMode, semantic_selection::SemanticSelection};
    app.add_singleton_model(|ctx| AppExecutionMode::new(mode, is_sandboxed, ctx));

    app.update(init_and_register_user_preferences);
    app.add_singleton_model(|_ctx| SettingsManager::default());
    app.add_singleton_model(ZapNGConfig::mock);

    AccessibilitySettings::register(app);
    app.update(AISettings::register_and_subscribe_to_events);
    AliasExpansionSettings::register(app);
    // ZapNG Wave 7-3:`AmbientAgentSettings` 随 ambient-agent UI 子系统物理删。
    AppEditorSettings::register(app);
    BlockVisibilitySettings::register(app);
    BlockListSettings::register(app);
    PreferencesSettings::register(app);
    CommandSearchSettings::register(app);
    DebugSettings::register(app);
    AppIconSettings::register(app);
    EmacsBindingsSettings::register(app);

    #[cfg(feature = "local_fs")]
    {
        crate::util::file::external_editor::EditorSettings::register(app);
    }

    FontSettings::register(app);
    GeneralSettings::register(app);
    GPUSettings::register(app);
    InputModeSettings::register(app);
    InputSettings::register(app);
    KeysSettings::register(app);
    LigatureSettings::register(app);

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        use crate::settings::LinuxAppConfiguration;
        LinuxAppConfiguration::register(app);
    }

    NativePreferenceSettings::register(app);
    SafeModeSettings::register(app);
    SameLinePromptBlockSettings::register(app);
    ScrollSettings::register(app);
    SelectionSettings::register(app);
    app.update(|ctx| {
        ZapNGifySettings::register(ctx);
    });
    SessionSettings::register(app);
    SshSettings::register(app);
    TabSettings::register(app);
    TerminalSettings::register(app);
    PaneSettings::register(app);
    ThemeSettings::register(app);
    UndoCloseSettings::register(app);
    VimBannerSettings::register(app);
    ZapNGDriveSettings::register(app);
    WindowSettings::register(app);
    SharedSessionSettings::register(app);
    CodeSettings::register(app);
    SemanticSelection::register(app);

    app.update(|ctx| {
        // Register a no-op secure storage provider for testing.
        zap_ng_ui_extras::secure_storage::register_noop("test", ctx);

        // Add settings models that are backed by secure storage, not user preferences.
        ctx.add_singleton_model(ai::api_keys::ApiKeyManager::new);
    });
}
