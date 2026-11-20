// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use anyhow::{anyhow, Result};
use std::borrow::Cow;
pub mod root_view;

extern crate zap_ng_ui;
use rust_embed::RustEmbed;
use zap_ng_ui::{platform, AssetProvider};

#[derive(Clone, Copy, RustEmbed)]
#[folder = "examples/assets"]
pub struct Assets;

// The static assets we need to load in app.
pub static ASSETS: Assets = Assets;

// Implement the AssetProvider trait here (required by App::new).
impl AssetProvider for Assets {
    fn get(&self, path: &str) -> Result<Cow<'_, [u8]>> {
        <Assets as RustEmbed>::get(path)
            .map(|f| f.data)
            .ok_or_else(|| anyhow!("no asset exists at path {}", path))
    }
}

// Create a view where we could use keybindings to change its state.
// In this example, we are going to have two simple bindings:
// 1. Cmd-Enter to toggle showing / hiding a solid red rect
// 2. Enter to toggle showing / hiding some texts
fn main() -> Result<()> {
    let app_builder =
        platform::AppBuilder::new(platform::AppCallbacks::default(), Box::new(ASSETS), None);
    let _ = app_builder.run(move |ctx| {
        root_view::init(ctx);
        ctx.add_window(
            zap_ng_ui::AddWindowOptions::default(),
            root_view::RootView::new,
        );
    });

    Ok(())
}
