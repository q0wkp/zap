//! The pane holding the BYOP request inspector. Not persisted: the calls it shows are those it
//! captured while open.

use warpui::{AppContext, ModelHandle, View, ViewContext, ViewHandle};

use crate::ai::agent_providers::request_inspector::view::{
    RequestInspectorEvent, RequestInspectorView,
};
use crate::app_state::LeafContents;

use super::{
    view::PaneView, DetachType, PaneConfiguration, PaneContent, PaneGroup, PaneId, ShareableLink,
    ShareableLinkError,
};

pub struct RequestInspectorPane {
    view: ViewHandle<PaneView<RequestInspectorView>>,
    pane_configuration: ModelHandle<PaneConfiguration>,
}

impl RequestInspectorPane {
    pub fn new<V: View>(ctx: &mut ViewContext<V>) -> Self {
        let inspector = ctx.add_typed_action_view(RequestInspectorView::new);
        let pane_configuration = inspector.as_ref(ctx).pane_configuration();
        let view = ctx.add_typed_action_view(|ctx| {
            let pane_id = PaneId::from_request_inspector_pane_ctx(ctx);
            PaneView::new(pane_id, inspector, (), pane_configuration.clone(), ctx)
        });
        Self {
            view,
            pane_configuration,
        }
    }

    fn inspector(&self, ctx: &AppContext) -> ViewHandle<RequestInspectorView> {
        self.view.as_ref(ctx).child(ctx)
    }
}

impl PaneContent for RequestInspectorPane {
    fn id(&self) -> PaneId {
        PaneId::from_request_inspector_pane_view(&self.view)
    }

    fn attach(
        &self,
        _group: &PaneGroup,
        focus_handle: crate::pane_group::focus_state::PaneFocusHandle,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        self.view
            .update(ctx, |view, ctx| view.set_focus_handle(focus_handle, ctx));

        let pane_id = self.id();
        ctx.subscribe_to_view(
            &self.inspector(ctx),
            move |pane_group, _, event, ctx| match event {
                RequestInspectorEvent::Pane(pane_event) => {
                    pane_group.handle_pane_event(pane_id, pane_event, ctx)
                }
            },
        );
        ctx.subscribe_to_view(&self.view, move |group, _, event, ctx| {
            group.handle_pane_view_event(pane_id, event, ctx);
        });
    }

    fn detach(
        &self,
        _group: &PaneGroup,
        _detach_type: DetachType,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        let inspector = self.inspector(ctx);
        ctx.unsubscribe_to_view(&inspector);
        ctx.unsubscribe_to_view(&self.view);
    }

    fn snapshot(&self, _app: &AppContext) -> LeafContents {
        LeafContents::RequestInspector
    }

    fn has_application_focus(&self, ctx: &mut ViewContext<PaneGroup>) -> bool {
        self.view.is_self_or_child_focused(ctx)
    }

    fn focus(&self, ctx: &mut ViewContext<PaneGroup>) {
        self.inspector(ctx)
            .update(ctx, |inspector, ctx| inspector.focus(ctx));
    }

    fn shareable_link(
        &self,
        _ctx: &mut ViewContext<PaneGroup>,
    ) -> Result<ShareableLink, ShareableLinkError> {
        Ok(ShareableLink::Base)
    }

    fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    fn is_pane_being_dragged(&self, ctx: &AppContext) -> bool {
        self.view.as_ref(ctx).is_being_dragged()
    }
}
