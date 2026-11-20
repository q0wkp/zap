// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::{
    settings_page::{
        render_body_item, AdditionalInfo, MatchData, PageType, SettingsPageMeta,
        SettingsPageViewHandle, SettingsWidget,
    },
    LocalOnlyIconState, SettingsSection, ToggleState,
};
use crate::{appearance::Appearance, drive::settings::ZapNGDriveSettings};
use zap_ng_core::{features::FeatureFlag, report_if_error, settings::ToggleableSetting as _};
use zap_ng_ui::{
    elements::{Element, MouseStateHandle},
    ui_components::{components::UiComponent, switch::SwitchStateHandle},
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

#[derive(Debug, Clone)]
pub enum ZapNGDriveSettingsPageAction {
    ToggleShowZapNGDrive,
    OpenUrl(String),
}

pub struct ZapNGDriveSettingsPageView {
    page: PageType<Self>,
}

impl ZapNGDriveSettingsPageView {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        Self {
            page: PageType::new_uncategorized(
                vec![Box::new(ZapNGDriveToggleWidget::default())],
                None,
            ),
        }
    }
}

impl Entity for ZapNGDriveSettingsPageView {
    type Event = ();
}

impl TypedActionView for ZapNGDriveSettingsPageView {
    type Action = ZapNGDriveSettingsPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ZapNGDriveSettingsPageAction::ToggleShowZapNGDrive => {
                ZapNGDriveSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.enable_zap_ng_drive.toggle_and_save_value(ctx));
                });
                ctx.notify();
            }
            ZapNGDriveSettingsPageAction::OpenUrl(url) => {
                ctx.open_url(url.as_str());
            }
        }
    }
}

impl View for ZapNGDriveSettingsPageView {
    fn ui_name() -> &'static str {
        "WarpDrivePage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl SettingsPageMeta for ZapNGDriveSettingsPageView {
    fn section() -> SettingsSection {
        SettingsSection::ZapNGDrive
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        FeatureFlag::ZapNewSettingsModes.is_enabled()
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<ZapNGDriveSettingsPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<ZapNGDriveSettingsPageView>) -> Self {
        SettingsPageViewHandle::ZapNGDrive(view_handle)
    }
}

#[derive(Default)]
struct ZapNGDriveToggleWidget {
    switch_state: SwitchStateHandle,
    info_icon_mouse_state: MouseStateHandle,
}

impl SettingsWidget for ZapNGDriveToggleWidget {
    type View = ZapNGDriveSettingsPageView;

    fn search_terms(&self) -> &str {
        "zapng drive tools panel command palette search workflows prompts notebooks environment variables"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let settings = ZapNGDriveSettings::as_ref(app);

        render_body_item::<ZapNGDriveSettingsPageAction>(
            "ZapNG Drive".into(),
            Some(AdditionalInfo {
                mouse_state: self.info_icon_mouse_state.clone(),
                on_click_action: Some(ZapNGDriveSettingsPageAction::OpenUrl(
                    "".to_string(),
                )),
                secondary_text: None,
                tooltip_override_text: None,
            }),
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            appearance
                .ui_builder()
                .switch(self.switch_state.clone())
                .check(*settings.enable_zap_ng_drive)
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(ZapNGDriveSettingsPageAction::ToggleShowZapNGDrive);
                })
                .finish(),
            Some("ZapNG Drive is a local workspace in your terminal where you can save Workflows, Notebooks, Prompts, and Environment Variables on this device.".into()),
        )
    }
}
