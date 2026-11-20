// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::{elements::MouseStateHandle, AppContext, Element};

use crate::{
    appearance::Appearance,
    cloud_object::{Space, StoredObjectMetadata},
    themes::theme::Fill,
    ui_components::icons::Icon,
};

use super::{
    cloud_object_styling::zap_ng_drive_icon_color,
    index::{zap_ng_drive_section_header_position_id, DriveIndexAction, DriveIndexSection},
    DriveObjectType, ObjectTypeAndId,
};

pub mod ai_fact;
pub mod ai_fact_collection;
pub mod env_var_collection;
pub mod folder;
pub mod item;
pub mod mcp_server;
pub mod mcp_server_collection;
pub mod notebook;
pub mod space;
pub mod workflow;

pub trait ZapNGDriveItem {
    /// The display name of the item. If the item is unnamed, this may return `None` - implementations
    /// should prefer this over `Some("")`, as it lets the index view use alternate styling.
    fn display_name(&self) -> Option<String>;
    fn metadata(&self) -> Option<&StoredObjectMetadata>;
    fn object_type(&self) -> Option<DriveObjectType>;
    fn secondary_icon(&self, color: Option<Fill>) -> Option<Box<dyn Element>>; // The optional icon to the right of the name
    fn click_action(&self) -> Option<DriveIndexAction>;
    fn preview(&self, appearance: &Appearance) -> Option<Box<dyn Element>>;
    fn zap_ng_drive_id(&self) -> ZapNGDriveItemId;
    fn sync_status_icon(
        &self,
        sync_queue_is_dequeueing: bool,
        hover_state: MouseStateHandle,
        appearance: &Appearance,
    ) -> Option<Box<dyn Element>>;

    fn icon(&self, appearance: &Appearance, color: Option<Fill>) -> Option<Box<dyn Element>> {
        let object_type = self.object_type()?;
        let icon_fill = color.unwrap_or(zap_ng_drive_icon_color(appearance, object_type).into());
        Some(Icon::from(object_type).to_zap_ng_ui_icon(icon_fill).finish())
    }

    /// If implemented, returns a string that summarizes the primary action history. For example, "Run 2 times in the last week"
    fn action_summary(&self, app: &AppContext) -> Option<String>;

    /// Returns Some(true) if this is an open folder, Some(false) if closed folder, None if not a folder
    fn is_folder_open(&self) -> Option<bool> {
        None
    }

    fn clone_box(&self) -> Box<dyn ZapNGDriveItem>;
}

impl ZapNGDriveItemId {
    pub fn drive_row_position_id(&self) -> String {
        match self {
            Self::AIFactCollection => "AI_fact_collection".to_string(),
            Self::MCPServerCollection => "MCP_server_collection".to_string(),
            Self::Object(object_id) => object_id.drive_row_position_id(),
            Self::Space(space) => {
                zap_ng_drive_section_header_position_id(&DriveIndexSection::Space(*space))
            }
            Self::Trash => "Trash".to_string(),
        }
    }
}
/// This uniquely identifies an item in ZapNG Drive index
/// Includes spaces (which ObjectTypeAndId does not entail)
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum ZapNGDriveItemId {
    AIFactCollection,
    MCPServerCollection,
    Object(ObjectTypeAndId),
    Space(Space),
    Trash,
}

impl Clone for Box<dyn ZapNGDriveItem> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
