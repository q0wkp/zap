// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::platform::mac::rendering::metal::renderer::Renderer;
use std::collections::HashMap;

use zap_ng_ui_core::rendering;

pub struct RendererManager {
    /// Maps a device's registry ID to its renderer (collection of state related
    /// to rendering on a particular device).
    renderers: HashMap<u64, Renderer>,
}

impl RendererManager {
    pub fn new() -> Self {
        Self {
            renderers: Default::default(),
        }
    }

    pub fn renderer_for_device(&mut self, device: &metal::Device) -> &mut Renderer {
        use std::collections::hash_map::Entry::*;
        match self.renderers.entry(device.registry_id()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(Renderer::new(
                device,
                metal::MTLPixelFormat::BGRA8Unorm,
                rendering::GlyphConfig::default(),
            )),
        }
    }
}
