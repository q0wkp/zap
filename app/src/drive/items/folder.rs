// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_core::features::FeatureFlag;
use zap_ng_ui::{elements::MouseStateHandle, AppContext, Element};

use crate::{
    appearance::Appearance,
    cloud_object::StoredObjectMetadata,
    drive::{
        cloud_object_styling::zap_ng_drive_icon_color, folders::FolderObject,
        index::DriveIndexAction, DriveObjectType, ObjectTypeAndId,
    },
    themes::theme::Fill,
    ui_components::icons::Icon,
};

use super::{ZapNGDriveItem, ZapNGDriveItemId};

#[derive(Clone)]
pub struct ZapNGDriveFolder {
    id: ObjectTypeAndId,
    folder: FolderObject,
}

impl ZapNGDriveFolder {
    pub fn new(id: ObjectTypeAndId, folder: FolderObject) -> Self {
        Self { id, folder }
    }
}

impl ZapNGDriveItem for ZapNGDriveFolder {
    fn display_name(&self) -> Option<String> {
        if self.folder.model().name.is_empty() {
            None
        } else {
            Some(self.folder.model().name.clone())
        }
    }

    fn metadata(&self) -> Option<&StoredObjectMetadata> {
        Some(&self.folder.metadata)
    }

    fn object_type(&self) -> Option<DriveObjectType> {
        Some(DriveObjectType::Folder)
    }

    fn icon(&self, appearance: &Appearance, color: Option<Fill>) -> Option<Box<dyn Element>> {
        let icon_fill =
            color.unwrap_or(zap_ng_drive_icon_color(appearance, DriveObjectType::Folder).into());
        let icon = if FeatureFlag::WarpPacks.is_enabled() && self.folder.model().is_warp_pack {
            Icon::PackageCheck
        } else {
            Icon::from(DriveObjectType::Folder)
        };

        Some(icon.to_zap_ng_ui_icon(icon_fill).finish())
    }

    fn secondary_icon(&self, _color: Option<Fill>) -> Option<Box<dyn Element>> {
        None
    }

    fn is_folder_open(&self) -> Option<bool> {
        Some(self.folder.model().is_open)
    }

    fn click_action(&self) -> Option<DriveIndexAction> {
        Some(DriveIndexAction::ToggleFolderOpen(self.folder.id))
    }

    fn preview(&self, _: &Appearance) -> Option<Box<dyn Element>> {
        None
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
        self.folder.metadata.pending_changes_statuses.render_icon(
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
