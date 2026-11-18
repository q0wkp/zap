//! Asks the user before a long multi-line command is sent to a shell without bracketed paste,
//! which runs each of its lines as a separate command.

use pathfinder_geometry::vector::vec2f;
use warp_core::ui::theme::Fill;
use warpui::elements::{
    Align, ChildAnchor, ChildView, Container, OffsetPositioning, ParentAnchor, ParentOffsetBounds,
    Stack,
};
use warpui::keymap::{FixedBinding, Keystroke};
use warpui::ui_components::components::{UiComponent, UiComponentStyles};
use warpui::{
    AppContext, Element, Entity, EntityId, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

use crate::appearance::Appearance;
use crate::ui_components::dialog::{Dialog, dialog_styles};
use crate::view_components::action_button::{
    ActionButton, KeystrokeSource, NakedTheme, PrimaryTheme,
};

pub fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.register_fixed_bindings([
        FixedBinding::new(
            "escape",
            MultilineCommandConfirmationAction::Cancel,
            id!(MultilineCommandConfirmationDialog::ui_name()),
        ),
        FixedBinding::new(
            "enter",
            MultilineCommandConfirmationAction::Confirm,
            id!(MultilineCommandConfirmationDialog::ui_name()),
        ),
    ]);
}

const DIALOG_WIDTH: f32 = 460.;

/// The command waiting for the user's answer.
#[derive(Clone, Copy)]
pub struct MultilineCommandConfirmationSource {
    /// The terminal view whose input holds the command.
    pub terminal_view_id: EntityId,
    pub line_count: usize,
}

pub struct MultilineCommandConfirmationDialog {
    cancel_button: ViewHandle<ActionButton>,
    run_button: ViewHandle<ActionButton>,
    source: Option<MultilineCommandConfirmationSource>,
}

impl MultilineCommandConfirmationDialog {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let cancel_button = ctx.add_typed_action_view(|_| {
            ActionButton::new(crate::t!("common-cancel"), NakedTheme).on_click(|ctx| {
                ctx.dispatch_typed_action(MultilineCommandConfirmationAction::Cancel);
            })
        });

        let enter_keystroke = Keystroke::parse("enter").expect("Valid keystroke");
        let run_button = ctx.add_typed_action_view(|ctx| {
            ActionButton::new(crate::t!("multiline-command-dialog-run"), PrimaryTheme)
                .with_keybinding(KeystrokeSource::Fixed(enter_keystroke), ctx)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(MultilineCommandConfirmationAction::Confirm);
                })
        });

        Self {
            cancel_button,
            run_button,
            source: None,
        }
    }

    pub fn set_source(&mut self, source: MultilineCommandConfirmationSource) {
        self.source = Some(source);
    }
}

impl Entity for MultilineCommandConfirmationDialog {
    type Event = MultilineCommandConfirmationEvent;
}

impl View for MultilineCommandConfirmationDialog {
    fn ui_name() -> &'static str {
        "MultilineCommandConfirmationDialog"
    }

    fn on_focus(&mut self, _focus_ctx: &warpui::FocusContext, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);

        let cancel_button = Container::new(ChildView::new(&self.cancel_button).finish())
            .with_margin_right(12.)
            .finish();

        let line_count = self.source.map_or(0, |source| source.line_count);
        let dialog = Dialog::new(
            crate::t!("multiline-command-dialog-title", count = line_count),
            Some(crate::t!("multiline-command-dialog-body")),
            UiComponentStyles {
                width: Some(DIALOG_WIDTH),
                ..dialog_styles(appearance)
            },
        )
        .with_bottom_row_child(cancel_button)
        .with_bottom_row_child(ChildView::new(&self.run_button).finish())
        .build()
        .finish();

        let mut stack = Stack::new();
        stack.add_positioned_child(
            dialog,
            OffsetPositioning::offset_from_parent(
                vec2f(0., 0.),
                ParentOffsetBounds::WindowByPosition,
                ParentAnchor::Center,
                ChildAnchor::Center,
            ),
        );

        Container::new(Align::new(stack.finish()).finish())
            .with_background_color(Fill::blur().into())
            .with_corner_radius(app.windows().window_corner_radius())
            .finish()
    }
}

pub enum MultilineCommandConfirmationEvent {
    /// Run the command in the given terminal view.
    Confirm { terminal_view_id: EntityId },
    /// Leave the command in the given terminal view's input.
    Cancel { terminal_view_id: EntityId },
    /// The dialog was answered without a command waiting.
    Dismiss,
}

#[derive(Debug)]
pub enum MultilineCommandConfirmationAction {
    Confirm,
    Cancel,
}

impl TypedActionView for MultilineCommandConfirmationDialog {
    type Action = MultilineCommandConfirmationAction;

    fn handle_action(
        &mut self,
        action: &MultilineCommandConfirmationAction,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(source) = self.source.take() else {
            ctx.emit(MultilineCommandConfirmationEvent::Dismiss);
            return;
        };
        let terminal_view_id = source.terminal_view_id;
        match action {
            MultilineCommandConfirmationAction::Confirm => {
                ctx.emit(MultilineCommandConfirmationEvent::Confirm { terminal_view_id });
            }
            MultilineCommandConfirmationAction::Cancel => {
                ctx.emit(MultilineCommandConfirmationEvent::Cancel { terminal_view_id });
            }
        }
    }
}
