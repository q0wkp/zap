// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use std::env::current_dir;

use zap_ng_core::ui::appearance::Appearance;
use zap_ng_ui::App;

use crate::{
    cloud_object::model::persistence::ObjectStoreModel,
    cloud_object::update_manager::UpdateManager, network::NetworkStatus,
    workspaces::user_workspaces::UserWorkspaces, GlobalResourceHandles,
    GlobalResourceHandlesProvider,
};

use super::expand_dirs;

#[test]
fn test_expand_directories() {
    App::test((), |mut app| async move {
        app.update(crate::settings::init_and_register_user_preferences);

        let global_resource_handles = GlobalResourceHandles::mock(&mut app);
        app.add_singleton_model(|_| GlobalResourceHandlesProvider::new(global_resource_handles));
        app.add_singleton_model(ObjectStoreModel::mock);
        app.add_singleton_model(UserWorkspaces::default_mock);
        app.add_singleton_model(|_| Appearance::mock());
        app.add_singleton_model(|_| NetworkStatus::new());
        app.add_singleton_model(UpdateManager::mock);

        let directory = current_dir()
            .expect("current directory should exist")
            .parent()
            .expect("parent directory should exist")
            .to_path_buf()
            .join("crates")
            .join("integration");

        // Open a folder and verify we could expand it into the correct folder tree structure.
        assert_eq!(zap_ng_ui::r#async::block_on(expand_dirs([directory].into_iter().collect())).debug_print(), "(integration(tests(INTEGRATION_TESTING, data(test, test_launch_config, test_theme, test_theme_with_name, test_workflow))))");
    });
}
