// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

//! Inline conversation menu for selecting AI conversations, enabled
//! when `FeatureFlag::AgentView` is enabled.
mod data_source;
mod search_item;
mod view;

pub use view::{InlineConversationMenuEvent, InlineConversationMenuView};

use pathfinder_color::ColorU;
use zap_ng_core::ui::appearance::Appearance;
use zap_ng_ui::{keymap::Keystroke, SingletonEntity};

use crate::ai::conversation_navigation::ConversationNavigationData;
use crate::terminal::input::inline_menu::{
    default_navigation_message_items, InlineMenuAction, InlineMenuMessageArgs, InlineMenuRowAction,
    InlineMenuType,
};
use crate::terminal::input::message_bar::{Message, MessageItem};

/// Tab identifiers for the inline conversation menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineConversationMenuTab {
    /// Show all conversations.
    All,
    /// Show only conversations whose most recent directory matches the session's CWD.
    CurrentDirectory,
}

/// Action emitted when enter is hit on a conversation the inline conversation menu.
#[derive(Clone, Debug)]
pub struct AcceptConversation {
    pub navigation_data: ConversationNavigationData,
}

impl InlineMenuAction for AcceptConversation {
    const MENU_TYPE: InlineMenuType = InlineMenuType::ConversationMenu;

    fn produce_inline_menu_message<T>(args: InlineMenuMessageArgs<'_, Self, T>) -> Option<Message> {
        let InlineMenuMessageArgs {
            inline_menu_model,
            app,
        } = args;

        let mut items = Vec::new();

        if let Some(item) = inline_menu_model.selected_item() {
            let data = &item.navigation_data;
            let text = " continue in this pane";
            let navigation_data = data.clone();
            items.push(MessageItem::clickable(
                vec![
                    MessageItem::keystroke(Keystroke {
                        key: "enter".to_owned(),
                        ..Default::default()
                    }),
                    MessageItem::text(text),
                ],
                move |ctx| {
                    ctx.dispatch_typed_action(InlineMenuRowAction::Accept {
                        item: AcceptConversation {
                            navigation_data: navigation_data.clone(),
                        },
                        cmd_or_ctrl_enter: false,
                    });
                },
                inline_menu_model.mouse_states().accept.clone(),
            ));
        } else {
            let theme = Appearance::as_ref(app).theme();
            let disabled_color = theme.disabled_text_color(theme.background()).into_solid();
            items.extend([
                MessageItem::Keystroke {
                    keystroke: Keystroke {
                        key: "enter".to_owned(),
                        ..Default::default()
                    },
                    color: Some(disabled_color),
                    background_color: Some(ColorU::transparent_black()),
                },
                MessageItem::Text {
                    content: " continue in this pane".into(),
                    color: Some(disabled_color),
                },
            ]);
        }

        items.extend(default_navigation_message_items(&args));
        Some(Message::new(items))
    }
}
