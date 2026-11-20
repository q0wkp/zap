// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use std::path::PathBuf;

use crate::workspace::view::global_search::view::GlobalSearchEvent;
use crate::workspace::view::global_search::SearchConfig;
use zap_ng_ui::{Entity, ModelContext};

pub struct GlobalSearch {}

impl Entity for GlobalSearch {
    type Event = GlobalSearchEvent;
}

impl GlobalSearch {
    pub fn new() -> Self {
        GlobalSearch {}
    }

    pub fn abort_search(&mut self) {}

    pub fn run_search(
        &mut self,
        _pattern: String,
        _root: Vec<PathBuf>,
        _search_config: SearchConfig,
        _ctx: &mut ModelContext<Self>,
    ) {
    }
}

impl Default for GlobalSearch {
    fn default() -> Self {
        Self::new()
    }
}
