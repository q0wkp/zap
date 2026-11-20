// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use std::collections::HashMap;
use zap_ng::settings::INPUT_MODE;
use zap_ng::terminal::block_list_viewport::InputMode;

/// Returns a user defaults map with the `InputMode` set to `input_mode`.
#[allow(dead_code)]
pub fn input_mode(input_mode: InputMode) -> HashMap<String, String> {
    HashMap::from_iter([(
        INPUT_MODE.to_owned(),
        serde_json::to_string(&input_mode).expect("input_mode value should convert to json string"),
    )])
}
