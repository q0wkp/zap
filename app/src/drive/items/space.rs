use zap_ng_ui::{elements::MouseStateHandle, Element};

use crate::{
    appearance::Appearance,
    cloud_object::{Space, StoredObjectMetadata},
    drive::{index::DriveIndexAction, DriveObjectType},
    themes::theme::Fill,
};

use super::{ZapNGDriveItem, ZapNGDriveItemId};

#[derive(Clone)]
pub struct ZapNGDriveSpace {
    space: Space,
}

impl ZapNGDriveSpace {
    #[allow(dead_code)]
    pub fn new(space: Space) -> Self {
        Self { space }
    }
}

impl ZapNGDriveItem for ZapNGDriveSpace {
    fn display_name(&self) -> Option<String> {
        None
    }

    fn metadata(&self) -> Option<&StoredObjectMetadata> {
        None
    }

    fn object_type(&self) -> Option<DriveObjectType> {
        None
    }

    fn secondary_icon(&self, _color: Option<Fill>) -> Option<Box<dyn Element>> {
        None
    }

    fn click_action(&self) -> Option<DriveIndexAction> {
        None
    }

    fn preview(&self, _appearance: &Appearance) -> Option<Box<dyn Element>> {
        None
    }

    fn zap_ng_drive_id(&self) -> ZapNGDriveItemId {
        ZapNGDriveItemId::Space(self.space)
    }

    fn sync_status_icon(
        &self,
        _sync_queue_is_dequeueing: bool,
        _hover_state: MouseStateHandle,
        _appearance: &Appearance,
    ) -> Option<Box<dyn Element>> {
        None
    }

    fn clone_box(&self) -> Box<dyn ZapNGDriveItem> {
        Box::new(self.clone())
    }

    fn action_summary(&self, _app: &zap_ng_ui::AppContext) -> Option<String> {
        None
    }
}
