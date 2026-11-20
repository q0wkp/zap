// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

pub use pathfinder_color::ColorU;
pub use pathfinder_geometry::{
    rect::RectF,
    vector::{vec2f, Vector2F},
};

pub use crate::{
    core::{
        AppContext, Entity, GetSingletonModelHandle as _, ModelContext, ModelHandle,
        SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
    },
    elements::{
        Align, Border, ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment,
        DropShadow, Element, Empty, Fill, Flex, Hoverable, MainAxisAlignment, MainAxisSize,
        MinSize, MouseStateHandle, Padding, ParentElement as _, Radius, SavePosition, Text,
    },
    platform::Cursor,
    presenter::EventContext,
    ui_components::components::Coords,
};

pub mod stack {
    pub use crate::elements::{
        AnchorPair, ChildAnchor, OffsetPositioning, OffsetType, ParentAnchor, ParentOffsetBounds,
        PositioningAxis, Stack, XAxisAnchor, YAxisAnchor,
    };
}
