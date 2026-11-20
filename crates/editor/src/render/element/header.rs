// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::{
    content::text::{BlockHeaderSize, BufferBlockStyle},
    extract_block,
    render::model::{BlockItem, RenderState, viewport::ViewportItem},
};

use super::{
    RenderContext, RenderableBlock,
    placeholder::{BlockPlaceholder, Options},
};

pub struct RenderableHeader {
    viewport_item: ViewportItem,
    placeholder: BlockPlaceholder,
}

impl RenderableHeader {
    pub fn new(viewport_item: ViewportItem) -> Self {
        Self {
            viewport_item,
            placeholder: BlockPlaceholder::new(true),
        }
    }
}

impl RenderableBlock for RenderableHeader {
    fn viewport_item(&self) -> &ViewportItem {
        &self.viewport_item
    }

    fn layout(
        &mut self,
        model: &RenderState,
        ctx: &mut zap_ng_ui::LayoutContext,
        app: &zap_ng_ui::AppContext,
    ) {
        self.placeholder
            .layout(&self.viewport_item, model, ctx, app, |block| {
                let header_size = match block {
                    BlockItem::Header { header_size, .. } => *header_size,
                    other => {
                        if cfg!(debug_assertions) {
                            panic!("Expected a header, got {other:?}");
                        }
                        BlockHeaderSize::Header6
                    }
                };

                Options {
                    text: header_size.label(),
                    block_style: BufferBlockStyle::Header { header_size },
                }
            });
    }

    fn paint(&mut self, model: &RenderState, ctx: &mut RenderContext, _app: &zap_ng_ui::AppContext) {
        let content = model.content();
        let (paragraph, header_size) = extract_block!(
            self.viewport_item, content,
            (block, BlockItem::Header { header_size, paragraph }) => (block.header(paragraph), header_size)
        );

        if self
            .placeholder
            .paint(paragraph.content_origin(), model, ctx)
        {
            return;
        }

        let header_style = &model.styles().paragraph_styles(&BufferBlockStyle::Header {
            header_size: *header_size,
        });
        ctx.draw_paragraph(&paragraph, header_style, model);
    }
}
