// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

//! Inline plan menu for selecting among multiple AI document plans.
mod data_source;
mod search_item;
mod view;

pub use view::{InlinePlanMenuEvent, InlinePlanMenuView};

use ai::document::AIDocumentId;
use zap_ng_ui::keymap::Keystroke;

use crate::ai::document::ai_document_model::AIDocumentVersion;
use crate::terminal::input::inline_menu::{
    default_navigation_message_items, InlineMenuAction, InlineMenuMessageArgs, InlineMenuRowAction,
    InlineMenuType,
};
use crate::terminal::input::message_bar::{Message, MessageItem};

/// Action emitted when a plan is selected from the inline plan menu.
#[derive(Clone, Debug)]
pub struct AcceptPlan {
    pub document_id: AIDocumentId,
    pub document_version: AIDocumentVersion,
}

impl InlineMenuAction for AcceptPlan {
    const MENU_TYPE: InlineMenuType = InlineMenuType::PlanMenu;

    fn produce_inline_menu_message<T>(args: InlineMenuMessageArgs<'_, Self, T>) -> Option<Message> {
        let InlineMenuMessageArgs {
            inline_menu_model, ..
        } = args;

        let mut items = Vec::new();

        if let Some(item) = inline_menu_model.selected_item() {
            let accept_item = item.clone();
            items.push(MessageItem::clickable(
                vec![
                    MessageItem::keystroke(Keystroke {
                        key: "enter".to_owned(),
                        ..Default::default()
                    }),
                    MessageItem::text(crate::t!("terminal-message-open-plan")),
                ],
                move |ctx| {
                    ctx.dispatch_typed_action(InlineMenuRowAction::Accept {
                        item: accept_item.clone(),
                        cmd_or_ctrl_enter: false,
                    });
                },
                inline_menu_model.mouse_states().accept.clone(),
            ));
        }

        items.extend(default_navigation_message_items(&args));
        Some(Message::new(items))
    }
}
