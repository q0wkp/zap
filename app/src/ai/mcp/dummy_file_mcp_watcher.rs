// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::{Entity, ModelContext, SingletonEntity};

pub struct FileMCPWatcher {}

impl FileMCPWatcher {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {}
    }
}

impl Entity for FileMCPWatcher {
    type Event = ();
}

impl SingletonEntity for FileMCPWatcher {}
