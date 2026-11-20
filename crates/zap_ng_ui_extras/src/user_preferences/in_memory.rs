// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use std::{cell::RefCell, collections::HashMap};

use serde::{Deserialize, Serialize};

/// A non-persisted, memory-backed user preferences store.
#[derive(Default, Serialize, Deserialize)]
pub struct InMemoryPreferences {
    prefs: RefCell<HashMap<String, String>>,
}

impl super::UserPreferences for InMemoryPreferences {
    fn write_value(&self, key: &str, value: String) -> Result<(), super::Error> {
        self.prefs.borrow_mut().insert(key.to_owned(), value);
        Ok(())
    }

    fn read_value(&self, key: &str) -> Result<Option<String>, super::Error> {
        Ok(self.prefs.borrow().get(key).map(ToOwned::to_owned))
    }

    fn remove_value(&self, key: &str) -> Result<(), super::Error> {
        let _ = self.prefs.borrow_mut().remove(key);
        Ok(())
    }
}
