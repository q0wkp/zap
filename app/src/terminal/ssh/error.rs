use crate::appearance::Appearance;
use crate::terminal::model::ansi::ZapNGificationUnavailableReason;
use crate::terminal::zapngify;
use crate::terminal::zapngify::render::apply_spacing_styles;
use crate::terminal::zapngify::render::build_description_row;
use crate::terminal::zapngify::settings::ZapNGifySettings;
use crate::ui_components::icons::Icon as UiIcon;
use markdown_parser::FormattedText;
use markdown_parser::FormattedTextFragment;
use markdown_parser::FormattedTextLine;
use zap_ng_core::channel::ChannelState;
use zap_ng_core::ui::theme::ZapNGTheme;
use zap_ng_ui::elements::HighlightedHyperlink;
use zap_ng_ui::elements::Hoverable;
use zap_ng_ui::elements::Icon;
use zap_ng_ui::elements::MainAxisAlignment;
use zap_ng_ui::elements::MainAxisSize;
use zap_ng_ui::elements::MouseStateHandle;
use zap_ng_ui::keymap::FixedBinding;
use zap_ng_ui::platform::Cursor;
use zap_ng_ui::ui_components::button::ButtonVariant;
use zap_ng_ui::ui_components::components::UiComponent;
use zap_ng_ui::ui_components::components::UiComponentStyles;
use zap_ng_ui::AppContext;
use zap_ng_ui::BlurContext;
use zap_ng_ui::FocusContext;
use zap_ng_ui::{
    elements::{Border, Container, CrossAxisAlignment, Flex, ParentElement},
    Element, Entity, SingletonEntity, TypedActionView, View, ViewContext,
};

const TMUX_NOT_INSTALLED_ERROR: &str =
    "tmux is not installed on the remote machine. Please install tmux and try again.";
const UNSUPPORTED_TMUX_VERSION_ERROR: &str =
    "The tmux version available on the remote machine is below 3.0. Please install tmux 3.0 or greater using a different method and try again.";
const TMUX_FAILED_ERROR: &str =
    "tmux failed to execute on the remote machine. Please re-install tmux and try again.";
const WARPIFY_TIMEOUT_ERROR: &str = "Warpifying the session hit a timeout.";
const UNSUPPORTED_SHELL_ERROR: &str =
    "Unsupported shell. Please set bash, zsh, or fish as your default shell and try again.";
const TMUX_INSTALL_FAILED_ERROR: &str =
    "The tmux install hit an unexpected error. Please install tmux manually and try again.";

const SSH_GITHUB_ISSUE_URL: &str = "https://github.com/zerx-lab/warp/issues/new?assignees=&labels=Bugs,SSH-tmux&projects=&template=03_ssh_tmux.yml";

fn get_ssh_github_issue_url(title: &str) -> String {
    let url = if let Some(version) = ChannelState::app_version() {
        format!("{SSH_GITHUB_ISSUE_URL}&warp-version={version}")
    } else {
        SSH_GITHUB_ISSUE_URL.to_string()
    };
    // prepend the title with "SSH tmux bug report: " and uri encode it
    let title = format!("SSH tmux bug report: {title:?}");
    let title = urlencoding::encode(&title);
    format!("{url}&title={title}")
}

impl ZapNGificationUnavailableReason {
    fn error_message(&self) -> &'static str {
        match self {
            ZapNGificationUnavailableReason::TmuxNotInstalled { .. } => TMUX_NOT_INSTALLED_ERROR,
            ZapNGificationUnavailableReason::UnsupportedTmuxVersion { .. } => {
                UNSUPPORTED_TMUX_VERSION_ERROR
            }
            ZapNGificationUnavailableReason::TmuxFailed => TMUX_FAILED_ERROR,
            ZapNGificationUnavailableReason::Timeout { .. } => WARPIFY_TIMEOUT_ERROR,
            ZapNGificationUnavailableReason::UnsupportedShell { .. } => UNSUPPORTED_SHELL_ERROR,
            ZapNGificationUnavailableReason::TmuxInstallFailed { .. } => TMUX_INSTALL_FAILED_ERROR,
        }
    }

    fn error_title(&self) -> &'static str {
        match self {
            ZapNGificationUnavailableReason::TmuxNotInstalled { .. } => "tmux Not Installed",
            ZapNGificationUnavailableReason::UnsupportedTmuxVersion { .. } => {
                "Unsupported Tmux Version"
            }
            ZapNGificationUnavailableReason::TmuxFailed => "tmux Failed",
            ZapNGificationUnavailableReason::Timeout {
                is_tmux_install, ..
            } => {
                if *is_tmux_install {
                    "tmux Install Timeout"
                } else {
                    "SSH Warpify Timeout"
                }
            }
            ZapNGificationUnavailableReason::UnsupportedShell { .. } => "Unsupported Shell",
            ZapNGificationUnavailableReason::TmuxInstallFailed { .. } => "tmux Install Failed",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SshErrorBlockEvent {
    ContinueWithoutZapNGification,
    ZapNGifyWithoutTmux,
}

#[derive(Debug, Clone)]
pub enum SshErrorBlockAction {
    ContinueWithoutZapNGification,
    ZapNGifyWithoutTmux,
    OpenUrl(String),
    AddSshHostToDenylist(String),
    Focus,
}

pub struct SshErrorBlock {
    error_reason: ZapNGificationUnavailableReason,
    ssh_host: Option<String>,
    zapngify_without_tmux_button_mouse_state: MouseStateHandle,
    continue_button_mouse_state: MouseStateHandle,
    report_link_highlight_index: HighlightedHyperlink,
    never_zapngify_mouse_state_handle: MouseStateHandle,
    block_mouse_state: MouseStateHandle,
    is_focused: bool,
}

pub fn init(app: &mut AppContext) {
    use zap_ng_ui::keymap::macros::*;

    app.register_fixed_bindings([
        FixedBinding::new(
            "enter",
            SshErrorBlockAction::ZapNGifyWithoutTmux,
            id!(SshErrorBlock::ui_name()),
        ),
        FixedBinding::new(
            "escape",
            SshErrorBlockAction::ContinueWithoutZapNGification,
            id!(SshErrorBlock::ui_name()),
        ),
        FixedBinding::new(
            "ctrl-c",
            SshErrorBlockAction::ContinueWithoutZapNGification,
            id!(SshErrorBlock::ui_name()),
        ),
    ]);
}

impl SshErrorBlock {
    #[allow(clippy::new_without_default)]
    pub fn new(error_reason: ZapNGificationUnavailableReason, ssh_host: Option<String>) -> Self {
        Self {
            error_reason,
            ssh_host,
            zapngify_without_tmux_button_mouse_state: Default::default(),
            continue_button_mouse_state: Default::default(),
            report_link_highlight_index: Default::default(),
            never_zapngify_mouse_state_handle: Default::default(),
            block_mouse_state: Default::default(),
            is_focused: false,
        }
    }

    pub fn focus(&self, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
        ctx.notify();
    }

    fn should_show_report_to_zap_ng_button(&self) -> bool {
        matches!(
            self.error_reason,
            ZapNGificationUnavailableReason::Timeout { .. }
                | ZapNGificationUnavailableReason::TmuxInstallFailed { .. }
        )
    }

    fn render_title_ui(
        &self,
        app: &AppContext,
        theme: &ZapNGTheme,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let header_contents = zapngify::render::build_header_row(
            "Error Warpifying session",
            Icon::new(UiIcon::AlertTriangle.into(), theme.ui_error_color()),
            theme,
            appearance,
        )
        .with_margin_right(8.)
        .finish();

        let right_hand_size = zapngify::render::render_never_zapngify_ssh_link(
            &self.ssh_host,
            app,
            appearance,
            self.never_zapngify_mouse_state_handle.clone(),
            move |ctx, ssh_host| {
                ctx.dispatch_typed_action(SshErrorBlockAction::AddSshHostToDenylist(
                    ssh_host.to_owned(),
                ));
            },
        );

        let mut row = Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::End)
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(header_contents);

        if let Some(right_hand_size) = right_hand_size {
            row.add_child(right_hand_size);
        }

        zapngify::render::apply_spacing_styles(Container::new(row.finish())).finish()
    }
}

impl Entity for SshErrorBlock {
    type Event = SshErrorBlockEvent;
}

pub const SSH_ERROR_BLOCK_VISIBLE_KEY: &str = "SshErrorBlockVisible";

impl View for SshErrorBlock {
    fn ui_name() -> &'static str {
        "SshErrorBlock"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();

        let mut content = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);

        content.add_child(self.render_title_ui(app, theme, appearance));

        content.add_child(zapngify::render::description_row(
            self.error_reason.error_message(),
            theme,
            appearance,
        ));

        let ui_builder = appearance.ui_builder();

        if self.should_show_report_to_zap_ng_button() {
            let report_issue_text = build_description_row(
                FormattedText::new([FormattedTextLine::Line(vec![
                    FormattedTextFragment::plain_text(crate::t!(
                        "terminal-ssh-report-issue-prefix"
                    )),
                    FormattedTextFragment::hyperlink(
                        crate::t!("terminal-ssh-report-issue-link"),
                        get_ssh_github_issue_url(self.error_reason.error_title()),
                    ),
                    FormattedTextFragment::plain_text(crate::t!(
                        "terminal-ssh-report-issue-suffix"
                    )),
                ])]),
                theme,
                appearance,
                self.report_link_highlight_index.clone(),
            )
            .with_hyperlink_font_color(theme.accent().into())
            .register_default_click_handlers(|link, ctx, _| {
                ctx.dispatch_typed_action(SshErrorBlockAction::OpenUrl(link.url));
            })
            .finish();
            content.add_child(apply_spacing_styles(Container::new(report_issue_text)).finish());
        }

        let buttons = Flex::row()
            .with_main_axis_size(MainAxisSize::Min)
            .with_child(
                Container::new(
                    ui_builder
                        .button(
                            ButtonVariant::Accent,
                            self.zapngify_without_tmux_button_mouse_state.clone(),
                        )
                        .with_centered_text_label(crate::t!("terminal-warpify-without-tmux"))
                        .with_style(UiComponentStyles {
                            font_size: Some(appearance.monospace_font_size()),
                            ..Default::default()
                        })
                        .build()
                        .with_cursor(Cursor::PointingHand)
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(SshErrorBlockAction::ZapNGifyWithoutTmux)
                        })
                        .finish(),
                )
                .with_margin_right(8.)
                .finish(),
            )
            .with_child(
                ui_builder
                    .button(
                        ButtonVariant::Secondary,
                        self.continue_button_mouse_state.clone(),
                    )
                    .with_centered_text_label(crate::t!("terminal-continue-without-warpification"))
                    .with_style(UiComponentStyles {
                        font_size: Some(appearance.monospace_font_size()),
                        ..Default::default()
                    })
                    .build()
                    .with_cursor(Cursor::PointingHand)
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(SshErrorBlockAction::ContinueWithoutZapNGification)
                    })
                    .finish(),
            );

        content.add_child(
            Container::new(buttons.finish())
                .with_uniform_margin(20.)
                .finish(),
        );

        Hoverable::new(self.block_mouse_state.clone(), |_| {
            Container::new(content.finish())
                .with_padding_top(10.)
                .with_background(theme.foreground().with_opacity(10))
                .with_border(Border::top(1.).with_border_fill(theme.outline()))
                .finish()
        })
        .on_click(|ctx, _, _| {
            ctx.dispatch_typed_action(SshErrorBlockAction::Focus);
        })
        .finish()
    }

    fn on_focus(&mut self, focus_ctx: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus_ctx.is_self_focused() {
            self.is_focused = true;
            ctx.notify();
        }
    }

    fn on_blur(&mut self, blur_ctx: &BlurContext, ctx: &mut ViewContext<Self>) {
        if blur_ctx.is_self_blurred() {
            self.is_focused = false;
            ctx.notify();
        }
    }
}

impl TypedActionView for SshErrorBlock {
    type Action = SshErrorBlockAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            SshErrorBlockAction::ZapNGifyWithoutTmux => {
                ctx.emit(SshErrorBlockEvent::ZapNGifyWithoutTmux)
            }
            SshErrorBlockAction::ContinueWithoutZapNGification => {
                ctx.emit(SshErrorBlockEvent::ContinueWithoutZapNGification)
            }
            SshErrorBlockAction::OpenUrl(url) => {
                ctx.open_url(url);
            }
            SshErrorBlockAction::AddSshHostToDenylist(ssh_host) => {
                let settings = ZapNGifySettings::handle(ctx);
                settings.update(ctx, |zapngify, ctx| {
                    zapngify.denylist_ssh_host(ssh_host, ctx);
                });
                ctx.emit(SshErrorBlockEvent::ContinueWithoutZapNGification);
                ctx.notify()
            }
            SshErrorBlockAction::Focus => {
                self.focus(ctx);
            }
        }
    }
}
