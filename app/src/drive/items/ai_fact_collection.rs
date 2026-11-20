// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::{elements::MouseStateHandle, AppContext, Element};

use crate::{
    appearance::Appearance,
    cloud_object::StoredObjectMetadata,
    drive::{index::DriveIndexAction, DriveObjectType},
    server::ids::ClientId,
    themes::theme::Fill,
};

use super::{ZapNGDriveItem, ZapNGDriveItemId};

#[derive(Clone)]
pub struct ZapNGDriveAIFactCollection {
    id: ClientId,
}

impl ZapNGDriveAIFactCollection {
    pub fn new(id: ClientId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> ClientId {
        self.id
    }
}

impl ZapNGDriveItem for ZapNGDriveAIFactCollection {
    fn display_name(&self) -> Option<String> {
        Some(crate::t!("rules-collection-name"))
    }

    fn metadata(&self) -> Option<&StoredObjectMetadata> {
        None
    }

    fn object_type(&self) -> Option<DriveObjectType> {
        Some(DriveObjectType::AIFactCollection)
    }

    fn secondary_icon(&self, _color: Option<Fill>) -> Option<Box<dyn Element>> {
        None
    }

    fn click_action(&self) -> Option<DriveIndexAction> {
        Some(DriveIndexAction::OpenAIFactCollection)
    }

    fn preview(&self, _appearance: &Appearance) -> Option<Box<dyn Element>> {
        None
    }

    fn zap_ng_drive_id(&self) -> ZapNGDriveItemId {
        ZapNGDriveItemId::AIFactCollection
    }

    fn sync_status_icon(
        &self,
        _sync_queue_is_dequeueing: bool,
        _hover_state: MouseStateHandle,
        _appearance: &Appearance,
    ) -> Option<Box<dyn Element>> {
        None
    }

    fn action_summary(&self, _app: &AppContext) -> Option<String> {
        None
    }

    fn clone_box(&self) -> Box<dyn ZapNGDriveItem> {
        Box::new(self.clone())
    }
}
