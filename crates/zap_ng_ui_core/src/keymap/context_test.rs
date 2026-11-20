// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::{macros::*, *};

#[test]
fn test_context_predicate_eval() -> anyhow::Result<()> {
    let predicate = id!("a") & id!("b") | eq!("c", "d");

    let mut context = Context::default();
    context.set.insert("a");
    assert!(!predicate.eval(&context));

    context.set.insert("b");
    assert!(predicate.eval(&context));

    context.set.remove("b");
    context.map.insert("c", "x");
    assert!(!predicate.eval(&context));

    context.map.insert("c", "d");
    assert!(predicate.eval(&context));

    let predicate = !id!("a");
    assert!(predicate.eval(&Context::default()));

    Ok(())
}
