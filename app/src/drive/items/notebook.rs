// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::{
    elements::{Flex, MouseStateHandle, ParentElement},
    fonts::Weight,
    ui_components::components::{UiComponent, UiComponentStyles},
    AppContext, Element,
};

use crate::{
    appearance::Appearance,
    cloud_object::StoredObjectMetadata,
    drive::{index::DriveIndexAction, DriveObjectType, ObjectTypeAndId},
    notebooks::NotebookObject,
    themes::theme::Fill,
};

use super::{ZapNGDriveItem, ZapNGDriveItemId};

#[derive(Clone)]
pub struct ZapNGDriveNotebook {
    id: ObjectTypeAndId,
    notebook: NotebookObject,
    is_ai_document: bool,
}

impl ZapNGDriveNotebook {
    pub fn new(id: ObjectTypeAndId, notebook: NotebookObject, is_ai_document: bool) -> Self {
        Self {
            id,
            notebook,
            is_ai_document,
        }
    }
}

impl ZapNGDriveItem for ZapNGDriveNotebook {
    fn display_name(&self) -> Option<String> {
        if self.notebook.model().title.is_empty() {
            None
        } else {
            Some(self.notebook.model().title.clone())
        }
    }

    fn metadata(&self) -> Option<&StoredObjectMetadata> {
        Some(&self.notebook.metadata)
    }

    fn object_type(&self) -> Option<DriveObjectType> {
        Some(DriveObjectType::Notebook {
            is_ai_document: self.is_ai_document,
        })
    }

    fn secondary_icon(&self, _color: Option<Fill>) -> Option<Box<dyn Element>> {
        None
    }

    fn click_action(&self) -> Option<DriveIndexAction> {
        Some(DriveIndexAction::OpenObject(self.id))
    }

    fn preview(&self, appearance: &Appearance) -> Option<Box<dyn Element>> {
        let title_text = self.notebook.model().title.clone();
        let title_to_render = if title_text.is_empty() {
            "Untitled".to_string()
        } else {
            title_text
        };
        let title = appearance
            .ui_builder()
            .wrappable_text(title_to_render, true)
            .with_style(UiComponentStyles {
                font_color: Some(
                    appearance
                        .theme()
                        .main_text_color(appearance.theme().background())
                        .into(),
                ),
                font_size: Some(14.),
                font_weight: Some(Weight::Bold),
                ..Default::default()
            })
            .build()
            .finish();

        Some(Flex::column().with_child(title).finish())
    }

    fn zap_ng_drive_id(&self) -> ZapNGDriveItemId {
        ZapNGDriveItemId::Object(self.id)
    }

    fn sync_status_icon(
        &self,
        sync_queue_is_dequeueing: bool,
        hover_state: MouseStateHandle,
        appearance: &Appearance,
    ) -> Option<Box<dyn Element>> {
        self.notebook.metadata.pending_changes_statuses.render_icon(
            sync_queue_is_dequeueing,
            hover_state,
            appearance,
        )
    }

    fn action_summary(&self, _app: &AppContext) -> Option<String> {
        None
    }

    fn clone_box(&self) -> Box<dyn ZapNGDriveItem> {
        Box::new(self.clone())
    }
}
