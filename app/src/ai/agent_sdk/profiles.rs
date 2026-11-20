// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use comfy_table::Cell;
use serde::Serialize;
use zap_ng_cli::{agent::AgentProfileCommand, GlobalOptions};
use zap_ng_ui::{AppContext, ModelContext, SingletonEntity};

use crate::ai::agent_sdk::output::{self, TableFormat};
use crate::ai::execution_profiles::profiles::AIExecutionProfilesModel;
use crate::cloud_object::model::generic_string_model::StringModel;
use crate::cloud_object::model::persistence::ObjectStoreModel;
use crate::server::ids::SyncId;

/// Handle Agent Profile-related CLI commands.
pub fn run(
    ctx: &mut AppContext,
    global_options: GlobalOptions,
    command: AgentProfileCommand,
) -> anyhow::Result<()> {
    let runner = ctx.add_singleton_model(|_ctx| ProfilesCommandRunner);
    match command {
        AgentProfileCommand::List => {
            runner.update(ctx, |runner, ctx| runner.list(global_options, ctx));
            Ok(())
        }
    }
}

/// Singleton model that runs async work for profile CLI commands.
struct ProfilesCommandRunner;

impl ProfilesCommandRunner {
    fn list(&self, global_options: GlobalOptions, ctx: &mut ModelContext<Self>) {
        // Ensure locally persisted profiles have completed their initial load.
        let initial_sync = ObjectStoreModel::as_ref(ctx).initial_load_complete();

        ctx.spawn(initial_sync, move |_, _, ctx| {
            let profiles_model = AIExecutionProfilesModel::as_ref(ctx);

            let profile_ids = profiles_model.get_all_profile_ids();

            let profiles: Vec<_> = profile_ids
                .iter()
                .flat_map(|id| profiles_model.get_profile_by_id(*id, ctx))
                .map(|profile| {
                    let name = profile.data().display_name().to_string();
                    let id = match profile.sync_id() {
                        Some(SyncId::ServerId(server_id)) => server_id.to_string(),
                        _ => "Unsynced".to_string(),
                    };
                    ProfileInfo { id, name }
                })
                .collect();

            output::print_list(profiles, global_options.output_format);

            ctx.terminate_app(zap_ng_ui::platform::TerminationMode::ForceTerminate, None);
        });
    }
}

impl zap_ng_ui::Entity for ProfilesCommandRunner {
    type Event = ();
}
impl SingletonEntity for ProfilesCommandRunner {}

/// Profile information that's shown in the `list` command.
#[derive(Serialize)]
struct ProfileInfo {
    id: String,
    name: String,
}

impl TableFormat for ProfileInfo {
    fn header() -> Vec<Cell> {
        vec![Cell::new("ID"), Cell::new("Name")]
    }

    fn row(&self) -> Vec<Cell> {
        vec![Cell::new(&self.id), Cell::new(&self.name)]
    }
}
