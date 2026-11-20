// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::{Entity, ModelContext, SingletonEntity};

/// Singleton model that tracks GPU state.
#[derive(Debug, Default, Clone)]
pub struct GPUState {
    has_low_power_gpu: bool,
}

impl GPUState {
    /// Creates a new GPUState with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks the low power GPU as available and stable
    pub(super) fn set_has_lower_power_gpu(
        &mut self,
        has_low_power_gpu: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        self.has_low_power_gpu = has_low_power_gpu;

        if has_low_power_gpu {
            ctx.emit(GPUStateEvent::LowPowerGPUAvailable);
        }
    }

    /// Returns whether the low power GPU is available for use
    pub fn is_low_power_gpu_available(&self) -> bool {
        self.has_low_power_gpu
    }
}

pub enum GPUStateEvent {
    LowPowerGPUAvailable,
}

impl SingletonEntity for GPUState {}

impl Entity for GPUState {
    type Event = GPUStateEvent;
}
