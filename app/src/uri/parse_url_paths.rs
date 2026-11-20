// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::cloud_object::extract_server_id_and_object_type_from_zap_ng_drive_link;
use crate::drive::ZapNGDriveObjectArgs;
use crate::ChannelState;
use url::Url;

#[derive(PartialEq, Debug)]
pub enum ZapNGWebLink {
    Session,
    DriveObject(Box<ZapNGDriveObjectArgs>),
}

pub fn get_item_data_from_zap_ng_link(url: &Url) -> Option<ZapNGWebLink> {
    if url.origin() == ChannelState::server_root_domain() {
        url.path_segments().and_then(|mut path_segments| {
            path_segments.next().and_then(|segment| match segment {
                "drive" => extract_server_id_and_object_type_from_zap_ng_drive_link(url)
                    .map(|args| ZapNGWebLink::DriveObject(Box::new(args))),
                "session" => Some(ZapNGWebLink::Session),
                _ => None,
            })
        })
    } else {
        None
    }
}
