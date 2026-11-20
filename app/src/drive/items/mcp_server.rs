use super::{ZapNGDriveItem, ZapNGDriveItemId};
use crate::{
    ai::mcp::MCPServerObject,
    appearance::Appearance,
    cloud_object::StoredObjectMetadata,
    drive::{index::DriveIndexAction, DriveObjectType, ObjectTypeAndId},
    themes::theme::Fill,
};
use zap_ng_ui::{elements::MouseStateHandle, AppContext, Element};

#[derive(Clone)]
pub struct ZapNGDriveMCPServer {
    id: ObjectTypeAndId,
    mcp_server: MCPServerObject,
}

impl ZapNGDriveMCPServer {
    pub fn new(id: ObjectTypeAndId, mcp_server: MCPServerObject) -> Self {
        Self { id, mcp_server }
    }
}

impl ZapNGDriveItem for ZapNGDriveMCPServer {
    fn display_name(&self) -> Option<String> {
        Some(self.mcp_server.model().string_model.name.clone())
    }
    fn metadata(&self) -> Option<&StoredObjectMetadata> {
        Some(&self.mcp_server.metadata)
    }

    fn object_type(&self) -> Option<DriveObjectType> {
        Some(DriveObjectType::MCPServer)
    }

    fn secondary_icon(&self, _color: Option<Fill>) -> Option<Box<dyn Element>> {
        None
    }

    fn click_action(&self) -> Option<DriveIndexAction> {
        Some(DriveIndexAction::OpenMCPServerCollection)
    }

    fn preview(&self, _appearance: &Appearance) -> Option<Box<dyn Element>> {
        // TODO
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
        self.mcp_server
            .metadata
            .pending_changes_statuses
            .render_icon(sync_queue_is_dequeueing, hover_state, appearance)
    }

    fn action_summary(&self, _app: &AppContext) -> Option<String> {
        None
    }

    fn clone_box(&self) -> Box<dyn ZapNGDriveItem> {
        Box::new(self.clone())
    }
}
