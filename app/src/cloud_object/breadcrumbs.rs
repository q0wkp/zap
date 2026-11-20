// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::{Space, StoredObject};
use crate::{
    drive::{folders::FolderObject, items::ZapNGDriveItemId, ObjectTypeAndId},
    ui_components::breadcrumb::Breadcrumb,
};
use zap_ng_ui::AppContext;

// Encapsulates an object that can contain other objects, and keeps
// information necessary to show breadcrumbs.
#[derive(Clone, Debug)]
pub struct ContainingObject {
    pub name: String,
    pub kind: ContainingObjectKind,
}

impl Breadcrumb for ContainingObject {
    fn label(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self) -> bool {
        true
    }
}

impl From<&FolderObject> for ContainingObject {
    fn from(folder: &FolderObject) -> Self {
        Self {
            name: folder.display_name().clone(),
            kind: ContainingObjectKind::Object(ObjectTypeAndId::Folder(folder.id)),
        }
    }
}

impl Space {
    pub fn into_containing_object(self, app: &AppContext) -> ContainingObject {
        ContainingObject {
            name: self.name(app).clone(),
            kind: ContainingObjectKind::Space(self),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ContainingObjectKind {
    Space(Space),
    Object(ObjectTypeAndId),
}

impl ContainingObjectKind {
    pub fn into_item_id(self) -> ZapNGDriveItemId {
        match self {
            ContainingObjectKind::Space(space) => ZapNGDriveItemId::Space(space),
            ContainingObjectKind::Object(object) => ZapNGDriveItemId::Object(object),
        }
    }
}
