//! The request inspector pane: a live list of the calls zap makes to BYOP providers, with the
//! request and the response of the selected one.
//!
//! The view listens for captured calls from the moment it is created until it is dropped or
//! paused (see [`super::subscribe`]); it polls nothing and runs no timer. What it captured
//! lives in its own bounded [`CallStore`] and goes away with the pane.

use std::collections::HashMap;

use warp_core::ui::appearance::Appearance;
use warpui::clipboard::ClipboardContent;
use warpui::elements::{
    Border, ChildView, ClippedScrollStateHandle, ClippedScrollable, Container, CrossAxisAlignment,
    DispatchEventResult, EventHandler, Fill, Flex, Hoverable, MainAxisSize, MouseStateHandle,
    ParentElement, ScrollbarWidth, Shrinkable, Text, Wrap,
};
use warpui::{AppContext, Element, Entity, ModelHandle, SingletonEntity, TypedActionView, View};
use warpui::{ViewContext, ViewHandle};

use super::store::{
    conversation_short_label, record_text, request_text, response_text, summary_line, CallFilter,
    CallRecord, CallStore,
};
use super::{subscribe, CallGroup, CaptureEvent, Subscription};
use crate::ai::blocklist::BlocklistAIHistoryModel;
use crate::pane_group::focus_state::PaneFocusHandle;
use crate::pane_group::pane::view;
use crate::pane_group::{BackingView, PaneConfiguration, PaneEvent};
use crate::view_components::action_button::{ActionButton, ButtonSize, SecondaryTheme};

/// The most characters of one request or response the pane lays out; copying gets all of it.
const MAX_DISPLAYED_CHARS: usize = 40_000;

#[derive(Debug, Clone)]
pub enum RequestInspectorEvent {
    Pane(PaneEvent),
}

impl From<PaneEvent> for RequestInspectorEvent {
    fn from(event: PaneEvent) -> Self {
        RequestInspectorEvent::Pane(event)
    }
}

#[derive(Debug, Clone)]
pub enum RequestInspectorAction {
    Focus,
    TogglePaused,
    Clear,
    CopyShown,
    ToggleGroup(CallGroup),
    Select(u64),
    CopyRequest,
    CopyResponse,
    FilterToSelectedConversation,
    ClearConversationFilter,
}

pub struct RequestInspectorView {
    store: CallStore,
    filter: CallFilter,
    /// `None` while paused: no new calls are captured for this view then.
    subscription: Option<Subscription>,
    selected: Option<u64>,
    row_mouse_states: HashMap<u64, MouseStateHandle>,
    list_scroll_state: ClippedScrollStateHandle,
    detail_scroll_state: ClippedScrollStateHandle,
    pause_button: ViewHandle<ActionButton>,
    clear_button: ViewHandle<ActionButton>,
    copy_shown_button: ViewHandle<ActionButton>,
    group_buttons: Vec<(CallGroup, ViewHandle<ActionButton>)>,
    conversation_filter_button: ViewHandle<ActionButton>,
    copy_request_button: ViewHandle<ActionButton>,
    copy_response_button: ViewHandle<ActionButton>,
    filter_to_conversation_button: ViewHandle<ActionButton>,
    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
}

fn button(
    label: String,
    action: RequestInspectorAction,
    ctx: &mut ViewContext<RequestInspectorView>,
) -> ViewHandle<ActionButton> {
    ctx.add_typed_action_view(move |_| {
        ActionButton::new(label, SecondaryTheme)
            .with_size(ButtonSize::Small)
            .on_click(move |ctx| ctx.dispatch_typed_action(action.clone()))
    })
}

impl RequestInspectorView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let pane_configuration =
            ctx.add_model(|_| PaneConfiguration::new(crate::t!("byop-inspector-title")));

        let group_buttons = CallGroup::ALL
            .into_iter()
            .map(|group| {
                let button = button(
                    group.label(),
                    RequestInspectorAction::ToggleGroup(group),
                    ctx,
                );
                button.update(ctx, |button, ctx| button.set_active(true, ctx));
                (group, button)
            })
            .collect();

        let mut me = Self {
            store: CallStore::default(),
            filter: CallFilter::default(),
            subscription: None,
            selected: None,
            row_mouse_states: HashMap::new(),
            list_scroll_state: ClippedScrollStateHandle::default(),
            detail_scroll_state: ClippedScrollStateHandle::default(),
            pause_button: button(
                crate::t!("byop-inspector-pause"),
                RequestInspectorAction::TogglePaused,
                ctx,
            ),
            clear_button: button(
                crate::t!("byop-inspector-clear"),
                RequestInspectorAction::Clear,
                ctx,
            ),
            copy_shown_button: button(
                crate::t!("byop-inspector-copy-shown"),
                RequestInspectorAction::CopyShown,
                ctx,
            ),
            group_buttons,
            conversation_filter_button: button(
                String::new(),
                RequestInspectorAction::ClearConversationFilter,
                ctx,
            ),
            copy_request_button: button(
                crate::t!("byop-inspector-copy-request"),
                RequestInspectorAction::CopyRequest,
                ctx,
            ),
            copy_response_button: button(
                crate::t!("byop-inspector-copy-response"),
                RequestInspectorAction::CopyResponse,
                ctx,
            ),
            filter_to_conversation_button: button(
                crate::t!("byop-inspector-only-this-conversation"),
                RequestInspectorAction::FilterToSelectedConversation,
                ctx,
            ),
            pane_configuration,
            focus_handle: None,
        };
        me.start_listening(ctx);
        me
    }

    fn start_listening(&mut self, ctx: &mut ViewContext<Self>) {
        let (subscription, events) = subscribe();
        self.subscription = Some(subscription);
        // The stream ends once the subscription is dropped and the calls it captured have
        // ended, so pausing and resuming leaves no stream behind.
        let _ = ctx.spawn_stream_local(events, Self::on_event, |_, _| {});
    }

    fn on_event(&mut self, event: CaptureEvent, ctx: &mut ViewContext<Self>) {
        let started = match &event {
            CaptureEvent::Started(request) => Some((request.id, self.filter.matches(request))),
            CaptureEvent::Text { .. }
            | CaptureEvent::Reasoning { .. }
            | CaptureEvent::ToolCall { .. }
            | CaptureEvent::Usage { .. }
            | CaptureEvent::Finished { .. } => None,
        };
        // Keep following the newest shown call while the selection is on it (or on nothing).
        let follow = self.selected.is_none() || self.selected == self.newest_shown_id();

        if !self.store.apply(event) {
            return;
        }
        if let Some((id, shown)) = started {
            self.row_mouse_states
                .insert(id, MouseStateHandle::default());
            if self.row_mouse_states.len() > self.store.len() {
                let store = &self.store;
                self.row_mouse_states
                    .retain(|id, _| store.get(*id).is_some());
            }
            if shown && follow {
                self.select(Some(id), ctx);
            }
        }
        ctx.notify();
    }

    fn shown_calls(&self) -> impl DoubleEndedIterator<Item = &CallRecord> {
        self.store
            .calls()
            .filter(|call| self.filter.matches(&call.request))
    }

    fn newest_shown_id(&self) -> Option<u64> {
        self.shown_calls().next_back().map(|call| call.request.id)
    }

    fn selected_call(&self) -> Option<&CallRecord> {
        let call = self.store.get(self.selected?)?;
        self.filter.matches(&call.request).then_some(call)
    }

    fn select(&mut self, id: Option<u64>, ctx: &mut ViewContext<Self>) {
        self.selected = id;
        ctx.notify();
    }

    fn copy(&self, text: String, ctx: &mut ViewContext<Self>) {
        ctx.clipboard().write(ClipboardContent::plain_text(text));
    }

    fn refresh_filter_buttons(&mut self, ctx: &mut ViewContext<Self>) {
        for (group, button) in &self.group_buttons {
            let shown = !self.filter.hidden_groups.contains(group);
            button.update(ctx, |button, ctx| button.set_active(shown, ctx));
        }
        if let Some(conversation) = &self.filter.conversation {
            let label = crate::t!(
                "byop-inspector-conversation-filter",
                name = conversation_label(conversation, ctx)
            );
            self.conversation_filter_button
                .update(ctx, |button, ctx| button.set_label(label, ctx));
        }
        ctx.notify();
    }

    pub fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    pub fn focus(&self, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
    }

    fn render_toolbar(&self, appearance: &Appearance) -> Box<dyn Element> {
        let mut toolbar = Wrap::row()
            .with_spacing(6.)
            .with_run_spacing(6.)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(ChildView::new(&self.pause_button).finish())
            .with_child(ChildView::new(&self.clear_button).finish())
            .with_child(ChildView::new(&self.copy_shown_button).finish());
        for (_, button) in &self.group_buttons {
            toolbar.add_child(ChildView::new(button).finish());
        }
        if self.filter.conversation.is_some() {
            toolbar.add_child(ChildView::new(&self.conversation_filter_button).finish());
        }

        let theme = appearance.theme();
        let mut column = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(toolbar.finish());
        let note = if self.subscription.is_none() {
            Some(crate::t!("byop-inspector-paused-note"))
        } else if self.store.is_empty() {
            Some(crate::t!("byop-inspector-empty"))
        } else {
            None
        };
        if let Some(note) = note {
            column.add_child(
                Container::new(
                    Text::new(note, appearance.ui_font_family(), appearance.ui_font_size())
                        .with_color(theme.nonactive_ui_text_color().into_solid())
                        .finish(),
                )
                .with_padding_top(6.)
                .finish(),
            );
        }

        Container::new(column.finish())
            .with_uniform_padding(8.)
            .with_border(Border::bottom(1.).with_border_fill(theme.outline()))
            .finish()
    }

    fn render_list(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let mut rows = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
        for call in self.shown_calls().rev() {
            let id = call.request.id;
            let Some(mouse_state) = self.row_mouse_states.get(&id) else {
                continue;
            };
            let is_selected = self.selected == Some(id);
            let text = summary_line(call);
            let font_family = appearance.ui_font_family();
            let font_size = appearance.ui_font_size();
            let text_color = theme.active_ui_text_color().into_solid();
            let selected_fill = theme.accent_overlay();
            let hover_fill = theme.surface_2();
            let row = Hoverable::new(mouse_state.clone(), move |mouse_state| {
                let mut row = Container::new(
                    Text::new(text.clone(), font_family, font_size)
                        .with_color(text_color)
                        .soft_wrap(false)
                        .finish(),
                )
                .with_horizontal_padding(8.)
                .with_vertical_padding(3.);
                if is_selected {
                    row = row.with_background(selected_fill);
                } else if mouse_state.is_hovered() {
                    row = row.with_background(hover_fill);
                }
                row.finish()
            })
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(RequestInspectorAction::Select(id));
            })
            .finish();
            rows.add_child(row);
        }

        ClippedScrollable::vertical(
            self.list_scroll_state.clone(),
            rows.finish(),
            ScrollbarWidth::Auto,
            theme.nonactive_ui_detail().into(),
            theme.active_ui_detail().into(),
            Fill::None,
        )
        .finish()
    }

    fn render_detail(&self, call: &CallRecord, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let heading = |text: String| {
            Container::new(
                Text::new(text, appearance.ui_font_family(), appearance.ui_font_size())
                    .with_color(theme.active_ui_text_color().into_solid())
                    .finish(),
            )
            .with_padding_top(10.)
            .with_padding_bottom(4.)
            .finish()
        };
        let body = |text: String| {
            Text::new(
                displayed(text),
                appearance.monospace_font_family(),
                appearance.monospace_font_size(),
            )
            .with_color(theme.foreground().into_solid())
            .finish()
        };

        let mut actions = Wrap::row()
            .with_spacing(6.)
            .with_run_spacing(6.)
            .with_child(ChildView::new(&self.copy_request_button).finish())
            .with_child(ChildView::new(&self.copy_response_button).finish());
        if call.request.conversation.is_some() && self.filter.conversation.is_none() {
            actions.add_child(ChildView::new(&self.filter_to_conversation_button).finish());
        }

        let response = response_text(call);
        let response = if response.is_empty() {
            crate::t!("byop-inspector-no-response-yet")
        } else {
            response
        };
        let content = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(heading(summary_line(call)))
            .with_child(actions.finish())
            .with_child(heading(crate::t!("byop-inspector-request")))
            .with_child(body(request_text(call)))
            .with_child(heading(crate::t!("byop-inspector-response")))
            .with_child(body(response))
            .finish();

        ClippedScrollable::vertical(
            self.detail_scroll_state.clone(),
            Container::new(content).with_uniform_padding(8.).finish(),
            ScrollbarWidth::Auto,
            theme.nonactive_ui_detail().into(),
            theme.active_ui_detail().into(),
            Fill::None,
        )
        .finish()
    }
}

/// `text` cut to what the pane lays out, with a note of how much was left out.
fn displayed(text: String) -> String {
    match text.char_indices().nth(MAX_DISPLAYED_CHARS) {
        Some((cut, _)) => {
            let omitted = text[cut..].chars().count();
            format!(
                "{}\n… {}",
                &text[..cut],
                crate::t!("byop-inspector-display-truncated", count = omitted)
            )
        }
        None => text,
    }
}

/// A conversation's title, or its short id when it has no title or is no longer loaded.
fn conversation_label(root_task_id: &str, app: &AppContext) -> String {
    BlocklistAIHistoryModel::as_ref(app)
        .all_live_conversations()
        .into_iter()
        .find(|(_, conversation)| conversation.get_root_task_id().to_string() == root_task_id)
        .and_then(|(_, conversation)| conversation.title())
        .unwrap_or_else(|| conversation_short_label(root_task_id))
}

impl Entity for RequestInspectorView {
    type Event = RequestInspectorEvent;
}

impl View for RequestInspectorView {
    fn ui_name() -> &'static str {
        "RequestInspectorView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();

        let mut column = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(self.render_toolbar(appearance))
            .with_child(Shrinkable::new(1., self.render_list(appearance)).finish());
        if let Some(call) = self.selected_call() {
            column.add_child(
                Shrinkable::new(
                    2.,
                    Container::new(self.render_detail(call, appearance))
                        .with_border(Border::top(1.).with_border_fill(theme.outline()))
                        .finish(),
                )
                .finish(),
            );
        }

        EventHandler::new(column.finish())
            .on_left_mouse_down(|ctx, _, _| {
                ctx.dispatch_typed_action(RequestInspectorAction::Focus);
                DispatchEventResult::PropagateToParent
            })
            .finish()
    }
}

impl TypedActionView for RequestInspectorView {
    type Action = RequestInspectorAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            RequestInspectorAction::Focus => ctx.focus_self(),
            RequestInspectorAction::TogglePaused => {
                let label = if self.subscription.take().is_some() {
                    crate::t!("byop-inspector-resume")
                } else {
                    self.start_listening(ctx);
                    crate::t!("byop-inspector-pause")
                };
                self.pause_button
                    .update(ctx, |button, ctx| button.set_label(label, ctx));
                ctx.notify();
            }
            RequestInspectorAction::Clear => {
                self.store.clear();
                self.row_mouse_states.clear();
                self.select(None, ctx);
            }
            RequestInspectorAction::CopyShown => {
                let text = self
                    .shown_calls()
                    .map(record_text)
                    .collect::<Vec<_>>()
                    .join("\n\n----------------\n\n");
                self.copy(text, ctx);
            }
            RequestInspectorAction::ToggleGroup(group) => {
                self.filter.toggle_group(*group);
                self.refresh_filter_buttons(ctx);
            }
            RequestInspectorAction::Select(id) => self.select(Some(*id), ctx),
            RequestInspectorAction::CopyRequest => {
                if let Some(call) = self.selected_call() {
                    let text = request_text(call);
                    self.copy(text, ctx);
                }
            }
            RequestInspectorAction::CopyResponse => {
                if let Some(call) = self.selected_call() {
                    let text = response_text(call);
                    self.copy(text, ctx);
                }
            }
            RequestInspectorAction::FilterToSelectedConversation => {
                self.filter.conversation = self
                    .selected_call()
                    .and_then(|call| call.request.conversation.clone());
                self.refresh_filter_buttons(ctx);
            }
            RequestInspectorAction::ClearConversationFilter => {
                self.filter.conversation = None;
                self.refresh_filter_buttons(ctx);
            }
        }
    }
}

impl BackingView for RequestInspectorView {
    type PaneHeaderOverflowMenuAction = RequestInspectorAction;
    type CustomAction = ();
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        action: &Self::PaneHeaderOverflowMenuAction,
        ctx: &mut ViewContext<Self>,
    ) {
        self.handle_action(action, ctx);
    }

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(RequestInspectorEvent::Pane(PaneEvent::Close));
    }

    fn focus_contents(&mut self, ctx: &mut ViewContext<Self>) {
        self.focus(ctx);
    }

    fn render_header_content(
        &self,
        _ctx: &view::HeaderRenderContext<'_>,
        _app: &AppContext,
    ) -> view::HeaderContent {
        view::HeaderContent::simple(crate::t!("byop-inspector-title"))
    }

    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, _ctx: &mut ViewContext<Self>) {
        self.focus_handle = Some(focus_handle);
    }
}
