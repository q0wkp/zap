// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use crate::external_secrets::ExternalSecret;
use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::{DataSourceRunErrorWrapper, SyncDataSource};
use itertools::Itertools;
use zap_ng_ui::AppContext;

use super::external_secret_fuzzy_match::FuzzyMatchExternalSecretResult;
use super::external_secret_search_item::ExternalSecretSearchItem;
use super::searcher::ExternalSecretSearchItemAction;

pub struct ExternalSecretDataSource {
    secrets: Vec<ExternalSecret>,
}

impl ExternalSecretDataSource {
    pub fn new(secrets: Vec<ExternalSecret>) -> Self {
        Self { secrets }
    }
}

impl SyncDataSource for ExternalSecretDataSource {
    type Action = ExternalSecretSearchItemAction;

    fn run_query(
        &self,
        query: &Query,
        _app: &AppContext,
    ) -> Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper> {
        let query_str = query.text.as_str();
        Ok(self
            .secrets
            .clone()
            .into_iter()
            .filter_map(move |secret| -> Option<QueryResult<Self::Action>> {
                FuzzyMatchExternalSecretResult::try_match(query_str, &secret.get_display_name())
                    .map(|match_result| {
                        ExternalSecretSearchItem {
                            external_secret: secret,
                            fuzzy_matched_secret: match_result,
                        }
                        .into()
                    })
            })
            .collect_vec())
    }
}
