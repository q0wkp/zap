// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::rendering::wgpu::{Renderer, Resources};
use crate::rendering::GlyphConfig;
use pathfinder_geometry::vector::Vector2F;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use wgpu::Device;

pub struct RendererManager {
    renderers: HashMap<DeviceID, Renderer>,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
struct DeviceID(u64);

impl From<&Device> for DeviceID {
    fn from(value: &Device) -> Self {
        let mut s = DefaultHasher::new();
        value.hash(&mut s);
        DeviceID(s.finish())
    }
}

impl RendererManager {
    pub fn new() -> Self {
        Self {
            renderers: Default::default(),
        }
    }

    /// Returns a [`Renderer`] identified by the device contained in [`Resources`].
    pub fn renderer_for_resources(
        &mut self,
        resources: &Resources,
        _window_size: Vector2F,
    ) -> &mut Renderer {
        use std::collections::hash_map::Entry::*;
        match self.renderers.entry((&resources.device).into()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(Renderer::new(resources, GlyphConfig::default())),
        }
    }
}
