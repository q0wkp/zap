// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

pub mod frame_capture;
mod renderer;
mod renderer_manager;

pub use renderer_manager::RendererManager;

/// Returns `true` if the given metal Device corresponds to the low power/integrated GPU.
///
/// In dual GPU Macs, this is `false` for the discrete high-performance GPU.
#[cfg_attr(wgpu, allow(dead_code))]
pub fn is_integrated_gpu(device: &metal::Device) -> bool {
    device.is_low_power() && !device.is_removable()
}
