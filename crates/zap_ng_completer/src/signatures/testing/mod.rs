// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::CommandRegistry;
pub(crate) mod legacy;

pub use legacy::*;

pub fn create_test_command_registry(
    signatures: impl IntoIterator<Item = warp_command_signatures::Signature>,
) -> CommandRegistry {
    use std::collections::HashMap;

    let generators = HashMap::from([test_generators().into()]);
    CommandRegistry::new_for_test(signatures, generators)
}

pub(crate) const TEST_GENERATOR_1_COMMAND: &str = "echo 1";
pub(crate) const TEST_GENERATOR_2_COMMAND: &str = "echo 2";
pub(crate) const TEST_ALIAS_COMMAND: &str = "echo alias";
