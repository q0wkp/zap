//! 主动式 AI(active AI)子链路的 BYOP 适配。
//!
//! 涵盖三类:
//! - `prompt_suggestions`:命令完成后给出"问问 Agent"建议(Simple/Coding)
//! - `nld_predict`:Agent 输入框打字时实时补全
//! - `relevant_files`:从给定文件列表中筛选与 query 相关的子集
//!
//! 共同模式:
//! 1. 调用方在 spawn 之前(还有 `&AppContext`)调 `dispatch::*` 系列 helper,
//!    解出 `OneshotConfig` + 渲染好的 system/user prompt → `RenderedRequest`
//! 2. spawn 闭包内调 `run_*(req)` 发请求 + 解析,返回各子链路对应的 response 类型
//! 3. UI 回调里直接消费返回的 response,与原 `ServerApi` 路径完全等价
//!
//! 没有 BYOP 配置(`active_ai_model` 解码失败)→ `dispatch::*` 返回 `None`,
//! 调用方静默 no-op(Zap 已剥云,不再 fallback ServerApi)。

use minijinja::context;
use serde::Serialize;
use zap_ng_ui::{AppContext, EntityId, SingletonEntity};

use super::oneshot::{
    byop_oneshot_completion, resolve_active_ai_oneshot, resolve_next_command_oneshot,
    OneshotConfig, OneshotOptions,
};
use super::request_inspector::CallKind;
use super::prompt_renderer;
use crate::ai::execution_profiles::profiles::AIExecutionProfilesModel;
use crate::ai::execution_profiles::prompt_overrides::{PromptChoice, PromptSlot};
use crate::ai::predict::generate_am_query_suggestions::GenerateAMQuerySuggestionsResponse;
use crate::settings::language::{Language, LanguageSettings};

pub mod parsing;

// ---------------------------------------------------------------------------
// 模板
// ---------------------------------------------------------------------------

// The templates live in the prompt renderer's environment, under `active_ai/`.
fn render(template: &str, ctx: minijinja::Value) -> String {
    prompt_renderer::env()
        .get_template(&format!("active_ai/{template}"))
        .and_then(|t| t.render(ctx))
        .unwrap_or_else(|e| {
            log::warn!("[active_ai] render {template} failed: {e}");
            String::new()
        })
}

/// Renders a system prompt: the profile's override for `slot` when it has a usable one,
/// otherwise the built-in `template`.
fn render_system(
    slot: PromptSlot,
    choice: &PromptChoice,
    template: &str,
    ctx: minijinja::Value,
) -> String {
    prompt_renderer::render_override(slot, choice, &ctx).unwrap_or_else(|| render(template, ctx))
}

/// The prompt choice the active profile of `terminal_view_id` makes for `slot`.
fn prompt_choice(
    app: &AppContext,
    terminal_view_id: Option<EntityId>,
    slot: PromptSlot,
) -> PromptChoice {
    AIExecutionProfilesModel::as_ref(app)
        .active_profile(terminal_view_id, app)
        .data()
        .prompt_overrides
        .get(slot)
        .clone()
}

// ---------------------------------------------------------------------------
// 公共上下文片段
// ---------------------------------------------------------------------------

/// 单条已完成命令块的精简上下文(供 prompt_suggestions / nld_predict 消费)。
#[derive(Debug, Clone, Serialize, Default)]
pub struct BlockSnippet {
    pub command: String,
    pub output_summary: String,
    pub exit_code: i32,
    pub pwd: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LastBlockSnippet {
    pub command: String,
    pub exit_code: i32,
    pub pwd: String,
}

/// The English name of the language Zap's UI is shown in, for asking a model to reply in it.
pub(crate) fn ui_language_name(app: &AppContext) -> &'static str {
    match *LanguageSettings::as_ref(app).language {
        Language::English => "English",
        Language::SimplifiedChinese => "Simplified Chinese",
        Language::Japanese => "Japanese",
        Language::Russian => "Russian",
        // Language::System follows the OS locale; resolve via the active i18n loader
        // so Chinese/Japanese/Russian system-locale users still get localized suggestions.
        Language::System => {
            let locale = crate::i18n::current_languages()
                .into_iter()
                .next()
                .map(|l| l.to_string())
                .unwrap_or_default();
            if locale.starts_with("zh") {
                "Simplified Chinese"
            } else if locale.starts_with("ja") {
                "Japanese"
            } else if locale.starts_with("ru") {
                "Russian"
            } else {
                "English"
            }
        }
    }
}

/// 已渲染好 prompt + 解析好 OneshotConfig 的请求 — 跨 spawn 边界传递。
pub struct RenderedRequest {
    pub cfg: OneshotConfig,
    pub system: String,
    pub user: String,
    pub opts: OneshotOptions,
}

// ---------------------------------------------------------------------------
// prompt_suggestions
// ---------------------------------------------------------------------------

pub mod prompt_suggestions {
    use super::*;

    pub struct Input {
        pub recent_blocks: Vec<BlockSnippet>,
        pub system_context: Option<String>,
        pub last_exit_code: i32,
    }

    /// Spawn 前调用:解 BYOP 配置 + 渲染 prompt。`None` ⇒ 静默 no-op。
    pub fn dispatch(
        app: &AppContext,
        terminal_view_id: Option<EntityId>,
        input: Input,
    ) -> Option<RenderedRequest> {
        let cfg = resolve_active_ai_oneshot(app, terminal_view_id)?;
        let language = ui_language_name(app);
        let system = render_system(
            PromptSlot::PromptSuggestions,
            &prompt_choice(app, terminal_view_id, PromptSlot::PromptSuggestions),
            "prompt_suggestions_system.j2",
            context! { language => language },
        );
        let user = render(
            "prompt_suggestions_user.j2",
            context! {
                recent_blocks => input.recent_blocks,
                system_context => input.system_context,
                last_exit_code => input.last_exit_code,
            },
        );
        Some(RenderedRequest {
            cfg,
            system,
            user,
            opts: OneshotOptions {
                kind: CallKind::PromptSuggestions,
                response_format_json: true,
                max_chars: Some(6000),
                ..Default::default()
            },
        })
    }

    /// Spawn 内执行:发请求 + 解析。失败 → `None`(调用方映射为 Error)。
    pub async fn run(req: RenderedRequest) -> Option<GenerateAMQuerySuggestionsResponse> {
        let raw = match byop_oneshot_completion(&req.cfg, &req.system, &req.user, &req.opts).await {
            Ok(s) => s,
            Err(e) => {
                log::debug!("[active_ai] prompt_suggestions oneshot failed: {e:#}");
                return None;
            }
        };
        log::debug!(
            "[active_ai] prompt_suggestions raw response ({} chars): {raw}",
            raw.len()
        );
        parsing::parse_suggestion(&raw)
    }
}

// ---------------------------------------------------------------------------
// nld_predict
// ---------------------------------------------------------------------------

pub mod nld_predict {
    use super::*;

    pub struct Input {
        pub partial_query: String,
        pub last_block: Option<LastBlockSnippet>,
        pub system_context: Option<String>,
    }

    pub fn dispatch(
        app: &AppContext,
        terminal_view_id: Option<EntityId>,
        input: Input,
    ) -> Option<RenderedRequest> {
        let cfg = resolve_active_ai_oneshot(app, terminal_view_id)?;
        let system = render_system(
            PromptSlot::InputCompletion,
            &prompt_choice(app, terminal_view_id, PromptSlot::InputCompletion),
            "nld_predict_system.j2",
            context! {},
        );
        let user = render(
            "nld_predict_user.j2",
            context! {
                partial_query => input.partial_query,
                last_block => input.last_block,
                system_context => input.system_context,
            },
        );
        Some(RenderedRequest {
            cfg,
            system,
            user,
            opts: OneshotOptions {
                kind: CallKind::InputPrediction,
                response_format_json: false,
                max_chars: Some(4000),
                ..Default::default()
            },
        })
    }

    pub async fn run(req: RenderedRequest) -> Option<String> {
        let raw = match byop_oneshot_completion(&req.cfg, &req.system, &req.user, &req.opts).await {
            Ok(s) => s,
            Err(e) => {
                log::debug!("[active_ai] nld_predict oneshot failed: {e:#}");
                return None;
            }
        };
        parsing::sanitize_predict(&raw)
    }
}

// ---------------------------------------------------------------------------
// relevant_files
// ---------------------------------------------------------------------------

pub mod relevant_files {
    use super::*;

    #[derive(Debug, Clone, Serialize)]
    pub struct FileEntry {
        pub path: String,
        pub symbols: String,
    }

    pub struct Input {
        pub query: String,
        pub files: Vec<FileEntry>,
    }

    pub struct Prepared {
        pub req: RenderedRequest,
        pub input_paths: Vec<String>,
    }

    pub fn dispatch(
        app: &AppContext,
        terminal_view_id: Option<EntityId>,
        input: Input,
    ) -> Option<Prepared> {
        let cfg = resolve_active_ai_oneshot(app, terminal_view_id)?;
        let input_paths: Vec<String> = input.files.iter().map(|f| f.path.clone()).collect();
        let system = render_system(
            PromptSlot::RelevantFiles,
            &prompt_choice(app, terminal_view_id, PromptSlot::RelevantFiles),
            "relevant_files_system.j2",
            context! {},
        );
        let user = render(
            "relevant_files_user.j2",
            context! {
                query => input.query,
                files => input.files,
            },
        );
        Some(Prepared {
            req: RenderedRequest {
                cfg,
                system,
                user,
                opts: OneshotOptions {
                    kind: CallKind::RelevantFiles,
                    response_format_json: true,
                    max_chars: Some(12000),
                    ..Default::default()
                },
            },
            input_paths,
        })
    }

    pub async fn run(prepared: Prepared) -> Vec<String> {
        let raw = match byop_oneshot_completion(
            &prepared.req.cfg,
            &prepared.req.system,
            &prepared.req.user,
            &prepared.req.opts,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                log::debug!("[active_ai] relevant_files oneshot failed: {e:#}");
                return Vec::new();
            }
        };
        parsing::parse_relevant_files(&raw, &prepared.input_paths)
    }
}

// ---------------------------------------------------------------------------
// workflow_metadata(Workflow Editor 的 Autofill 按钮:命令 → 参数化 metadata)
// ---------------------------------------------------------------------------

pub mod workflow_metadata {
    use super::*;

    pub use parsing::WorkflowMetadataDto;

    pub struct Input {
        pub command: String,
    }

    /// Spawn 前调用:解 BYOP 配置 + 渲染 prompt。`None` ⇒ 调用方提示用户配置 BYOP。
    pub fn dispatch(
        app: &AppContext,
        terminal_view_id: Option<EntityId>,
        input: Input,
    ) -> Option<RenderedRequest> {
        let cfg = resolve_active_ai_oneshot(app, terminal_view_id)?;
        let system = render_system(
            PromptSlot::WorkflowMetadata,
            &prompt_choice(app, terminal_view_id, PromptSlot::WorkflowMetadata),
            "workflow_metadata_system.j2",
            context! {},
        );
        let user = render(
            "workflow_metadata_user.j2",
            context! {
                command => input.command,
            },
        );
        Some(RenderedRequest {
            cfg,
            system,
            user,
            opts: OneshotOptions {
                kind: CallKind::WorkflowMetadata,
                response_format_json: true,
                max_chars: Some(4000),
                ..Default::default()
            },
        })
    }

    /// Spawn 内执行:发请求 + 解析。失败 → `None`(调用方映射为 BadCommand)。
    pub async fn run(req: RenderedRequest) -> Option<WorkflowMetadataDto> {
        let raw = match byop_oneshot_completion(&req.cfg, &req.system, &req.user, &req.opts).await {
            Ok(s) => s,
            Err(e) => {
                log::debug!("[active_ai] workflow_metadata oneshot failed: {e:#}");
                return None;
            }
        };
        log::debug!(
            "[active_ai] workflow_metadata raw response ({} chars): {raw}",
            raw.len()
        );
        parsing::parse_workflow_metadata(&raw)
    }
}

// ---------------------------------------------------------------------------
// next_command(灰色补全 / zero-state 建议)
// ---------------------------------------------------------------------------

pub mod next_command {
    use super::*;

    #[derive(Debug, Serialize)]
    struct UserRuleCtx {
        name: Option<String>,
        content: String,
    }

    pub struct Input {
        pub recent_blocks: Vec<BlockSnippet>,
        /// 已在 client 端从历史 DB 选出的相似命令上下文(可选)。
        pub history_context: String,
        pub system_context: Option<String>,
        /// 用户已输入的前缀(必须用作输出前缀)。
        pub prefix: Option<String>,
        /// 之前已 reject 的建议(避免重复)。
        pub rejected_suggestions: Vec<String>,
        /// 用户在 设置 → Agents → Rules 中配置的全局规则快照。
        pub user_rules: Vec<(Option<String>, String)>,
    }

    /// What [`resolve`] takes from the `AppContext` for [`run_with`].
    #[derive(Clone)]
    pub struct Resolved {
        pub cfg: OneshotConfig,
        system_prompt: PromptChoice,
    }

    /// Pre-spawn:解 BYOP 配置(需要 `&AppContext`)。`None` ⇒ 静默 no-op。
    pub fn resolve(app: &AppContext, terminal_view_id: Option<EntityId>) -> Option<Resolved> {
        let cfg = resolve_next_command_oneshot(app, terminal_view_id)?;
        Some(Resolved {
            cfg,
            system_prompt: prompt_choice(app, terminal_view_id, PromptSlot::NextCommand),
        })
    }

    /// In-spawn:用 cfg + Input 渲染 prompt 并发请求。
    /// 模板渲染不依赖 AppContext,可在 spawn 内同步调用。
    pub async fn run_with(resolved: Resolved, input: Input) -> Option<String> {
        let Resolved { cfg, system_prompt } = resolved;
        let user_rule_ctxs: Vec<UserRuleCtx> = input
            .user_rules
            .into_iter()
            .map(|(name, content)| UserRuleCtx { name, content })
            .collect();
        let system = render_system(
            PromptSlot::NextCommand,
            &system_prompt,
            "next_command_system.j2",
            context! {
                user_rules => user_rule_ctxs,
            },
        );
        let user = render(
            "next_command_user.j2",
            context! {
                recent_blocks => input.recent_blocks,
                history_context => input.history_context,
                system_context => input.system_context,
                prefix => input.prefix,
                rejected_suggestions => input.rejected_suggestions,
            },
        );
        let opts = OneshotOptions {
            kind: CallKind::NextCommand,
            response_format_json: false,
            max_chars: Some(8000),
            ..Default::default()
        };
        let raw = match byop_oneshot_completion(&cfg, &system, &user, &opts).await {
            Ok(s) => s,
            Err(e) => {
                log::debug!("[active_ai] next_command oneshot failed: {e:#}");
                return None;
            }
        };
        log::info!(
            "[active_ai] next_command raw response ({} chars): {raw:?}",
            raw.len()
        );
        let sanitized = parsing::sanitize_predict(&raw);
        if sanitized.is_none() && !raw.trim().is_empty() {
            log::warn!("[active_ai] next_command sanitize REJECTED raw response");
        }
        sanitized
    }
}
