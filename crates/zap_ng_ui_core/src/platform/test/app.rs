// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use futures_util::future::LocalBoxFuture;

use crate::{assets::AssetProvider, integration::TestDriver, platform, AppContext};

pub struct App;

impl App {
    #[allow(dead_code)]
    pub(in crate::platform) fn new(
        _callbacks: platform::app::AppCallbacks,
        _assets: Box<dyn AssetProvider>,
        _test_driver: Option<&TestDriver>,
    ) -> Self {
        unimplemented!();
    }

    #[allow(dead_code)]
    pub(in crate::platform) fn run(
        self,
        _init_fn: impl FnOnce(&mut AppContext, LocalBoxFuture<'static, crate::App>),
    ) {
        unimplemented!();
    }
}
