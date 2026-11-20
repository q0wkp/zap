// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::terminal::HistoryEntry;
use zap_ng_ui::Entity;

/// Responsible for managing the history of a shared session for a viewer.
#[derive(Default)]
pub struct SharedSessionHistoryModel {
    entries: Vec<HistoryEntry>,
}

impl SharedSessionHistoryModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entries(&self) -> impl Iterator<Item = &HistoryEntry> {
        self.entries.iter()
    }

    pub fn push(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
    }
}

impl Entity for SharedSessionHistoryModel {
    type Event = ();
}
