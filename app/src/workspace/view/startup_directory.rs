//! Logic to determine the working directory for new terminal sessions.

use super::Workspace;
use crate::terminal::session_settings::{NewSessionSource, SessionSettings};
use std::path::PathBuf;
use warpui::SingletonEntity;
use warpui::{AppContext, ViewContext, WindowId};

impl Workspace {
    /// Helper function to compute the initial directory for a new session
    /// that is inheriting its initial directory from the active session in
    /// the given workspace.
    fn initial_directory_from_active_session(&self, ctx: &AppContext) -> Option<PathBuf> {
        (!self.tabs.is_empty())
            .then(|| {
                self.active_tab_pane_group().read(ctx, |pane_group, ctx| {
                    pane_group.active_session_id(ctx).and_then(|base_pane_id| {
                        pane_group.startup_path_for_new_session(Some(base_pane_id), ctx)
                    })
                })
            })
            .flatten()
    }

    /// Helper function to compute the initial directory for a new session.
    /// Returns Some(path) if inheriting the initial directory from an active
    /// session or using the user's custom path setting,
    /// and None if the default startup directory (the user's home directory) should be used.
    pub(super) fn get_new_tab_startup_directory(
        &mut self,
        new_session_source: NewSessionSource,
        previous_session_window_id: Option<WindowId>,
        ctx: &mut ViewContext<Self>,
    ) -> Option<PathBuf> {
        // Get the Workspace from the window that hosted the previously-active
        // session.
        let prev_session_working_directory = match previous_session_window_id {
            // If the previous window is the one hosting this workspace, don't
            // do any indirection through AppContext.
            Some(window_id) if window_id == ctx.window_id() => {
                self.initial_directory_from_active_session(ctx)
            }
            // Otherwise, lookup the Workspace in that window and query it.
            Some(window_id) => {
                let workspace_handle = ctx
                    .views_of_type::<Workspace>(window_id)
                    .and_then(|views| views.first().cloned());
                workspace_handle.and_then(|workspace| {
                    workspace.read(ctx, |workspace, ctx| {
                        workspace.initial_directory_from_active_session(ctx)
                    })
                })
            }
            None => None,
        };

        compute_startup_directory_from_prev_session(
            new_session_source,
            prev_session_working_directory,
            ctx,
        )
    }
}

/// Helper function to compute the actual startup directory for the
/// new session based on the user's settings.
fn compute_startup_directory_from_prev_session(
    new_session_source: NewSessionSource,
    initial_directory_from_prev_session: Option<PathBuf>,
    ctx: &ViewContext<Workspace>,
) -> Option<PathBuf> {
    SessionSettings::handle(ctx).read(ctx, |settings, _ctx| {
        settings
            .working_directory_config
            .initial_directory_for_new_session(
                new_session_source,
                initial_directory_from_prev_session,
            )
    })
}
