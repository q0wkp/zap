// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

mod coalesce;
mod context;
mod describe;
mod engine;
mod matchers;
mod suggest;
pub use suggest::alias::*;

#[cfg(feature = "test-util")]
pub mod testing;

pub use context::{
    CommandExitStatus, CommandOutput, CompletionContext, GeneratorContext, PathCompletionContext,
    PathSeparators,
};
pub use describe::{describe, describe_given_token, Description, TopLevelCommandCaseSensitivity};
pub use engine::{EngineDirEntry, EngineFileType, LocationType};
pub use matchers::{Match, MatchStrategy, MatchType};
pub use suggest::{
    suggestions, CompleterOptions, CompletionsFallbackStrategy, MatchedSuggestion, Priority,
    Suggestion, SuggestionResults, SuggestionType, SuggestionTypeName,
};

fn get_path_separators(ctx: &dyn CompletionContext) -> PathSeparators {
    ctx.path_completion_context()
        .map(|ctx| ctx.path_separators())
        .unwrap_or(PathSeparators::for_os())
}
