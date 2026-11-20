//! Grid tooltips for the terminal view

use std::path::PathBuf;

use pathfinder_geometry::vector::vec2f;

use zap_ng_ui::{
    assets::asset_cache::{AssetCache, AssetSource, LocalFileContentVersion},
    elements::{
        Border, CacheOption, ChildAnchor, ConstrainedBox, Container, CornerRadius, Dismiss, Flex,
        Image, MouseStateHandle, OffsetPositioning, ParentElement, PositionedElementAnchor,
        PositionedElementOffsetBounds, Radius, Stack,
    },
    image_cache::ImageType,
    AppContext, Element, EventContext, SingletonEntity, ViewContext,
};

use super::{TerminalAction, TerminalView};
use crate::util::tooltips::{TooltipLink, TooltipRedaction};
use crate::{
    appearance::Appearance,
    terminal::{
        links::directly_open_link_keybinding_string,
        model::{session::SessionId, ObfuscateSecrets, Secret},
        safe_mode_settings::get_secret_obfuscation_mode,
        view::SecretTooltip,
        TerminalModel,
    },
};

cfg_if::cfg_if! {
    if #[cfg(feature = "local_fs")] {
        use crate::terminal::view::{RichContentLink, RichContentLinkTooltipInfo};
        use crate::util::openable_file_type::is_supported_image_file;
        use super::GridHighlightedLink;
    }
}

/// The largest size a link's image preview is drawn at.
pub(super) const IMAGE_PREVIEW_MAX_WIDTH: f32 = 320.;
pub(super) const IMAGE_PREVIEW_MAX_HEIGHT: f32 = 240.;
/// The preview's size when the image's own can't be read from its header (an SVG has none).
const IMAGE_PREVIEW_FALLBACK_WIDTH: f32 = 240.;
const IMAGE_PREVIEW_FALLBACK_HEIGHT: f32 = 160.;

/// A thumbnail shown above the links of a link's tooltip, when the link leads to an image file.
#[derive(Debug, Clone)]
pub(super) struct LinkImagePreview {
    /// The file the open tooltip's link leads to.
    pub(super) path: PathBuf,
    /// The image and the width and height to draw it at. `None` while the bytes of a file on
    /// a remote host are being fetched.
    pub(super) image: Option<(AssetSource, f32, f32)>,
}

/// The size to draw an image of the given dimensions at: its own, scaled down to fit the
/// preview's bounds.
pub(super) fn image_preview_size(dimensions: Option<(u32, u32)>) -> (f32, f32) {
    match dimensions {
        Some((width, height)) if width > 0 && height > 0 => {
            let (width, height) = (width as f32, height as f32);
            let scale = (IMAGE_PREVIEW_MAX_WIDTH / width)
                .min(IMAGE_PREVIEW_MAX_HEIGHT / height)
                .min(1.);
            ((width * scale).max(1.), (height * scale).max(1.))
        }
        Some(_) | None => (IMAGE_PREVIEW_FALLBACK_WIDTH, IMAGE_PREVIEW_FALLBACK_HEIGHT),
    }
}

struct GridTooltipLink {
    text: String,
    action: TerminalAction,
    /// Optional detail text to show after the link.
    detail: Option<String>,
    mouse_state: MouseStateHandle,
}

/// If appropriate, returns a GridTooltipLink for opening the file in zap-ng.
/// Mutates `detail_for_default` leaving None in place if the GridTooltipLink returned is the default
/// action on "Cmd+Click" and thus should use the detail_for_default.
#[cfg(feature = "local_fs")]
fn open_in_zap_ng_tooltip(
    path: std::path::PathBuf,
    line_and_column_num: Option<zap_ng_util::path::LineAndColumnArg>,
    detail_for_default: &mut Option<String>,
    mouse_state: MouseStateHandle,
    app: &AppContext,
) -> Option<GridTooltipLink> {
    use crate::{
        settings::CodeSettings, util::file::external_editor::EditorSettings,
        util::tooltips::should_show_open_in_zap_ng_link,
    };
    use settings::Setting as _;
    use zap_ng_ui::SingletonEntity;

    if !should_show_open_in_zap_ng_link(&path, app) {
        return None;
    }

    let detail = if *CodeSettings::as_ref(app).code_as_default_editor.value() {
        detail_for_default.take()
    } else {
        None
    };
    Some(GridTooltipLink {
        text: crate::t!("terminal-open-in-warp"),
        action: TerminalAction::OpenCodeInZapNG {
            path,
            layout: *EditorSettings::as_ref(app).open_file_layout.value(),
            line_col: line_and_column_num,
        },
        mouse_state,
        detail,
    })
}

/// Returns a GridTooltipLink for revealing the file in the platform's file explorer
/// (Finder on macOS, file manager on Linux/Windows).
#[cfg(feature = "local_fs")]
fn show_in_file_explorer_tooltip(
    path: std::path::PathBuf,
    mouse_state: MouseStateHandle,
) -> GridTooltipLink {
    let text = if cfg!(target_os = "macos") {
        "Show in Finder"
    } else {
        "Show containing folder"
    }
    .to_string();
    GridTooltipLink {
        text,
        action: TerminalAction::ShowInFileExplorer(path),
        mouse_state,
        detail: None,
    }
}

impl TerminalView {
    /// Renders the link and/or secrets tooltips on top of the grid
    /// Expects at least one of the two tooltips to be visible.
    // Unused variables allowed when no local filesystem as the `app` argument
    // is unused.
    #[cfg_attr(not(feature = "local_fs"), allow(unused_variables))]
    pub(super) fn render_grid_tooltip(
        &self,
        stack: &mut Stack,
        model: &TerminalModel,
        appearance: &Appearance,
        app: &AppContext,
    ) {
        let mut element_id = "terminal_view:first_cell_in_link".to_string();
        let mut links = vec![];
        let mut is_agent_conversation = false;

        if let Some(open_secret_tooltip) = &self.open_secret_tool_tip {
            match open_secret_tooltip {
                SecretTooltip::Grid {
                    tooltip,
                    is_agent_mode,
                } => {
                    let handle = *tooltip;
                    is_agent_conversation = *is_agent_mode;

                    // Position the tooltip above the first cell in the secret.
                    element_id = format!(
                        "terminal_view:first_cell_in_secret_{}",
                        handle.get_inner().id()
                    );

                    if matches!(get_secret_obfuscation_mode(app), ObfuscateSecrets::Yes) {
                        let is_redacted = model
                            .secret_from_handle(tooltip)
                            .is_some_and(Secret::is_obfuscated);

                        if is_redacted {
                            links.push(GridTooltipLink {
                                text: crate::t!("terminal-reveal-secret"),
                                action: TerminalAction::ToggleGridSecret {
                                    handle,
                                    show_secret: true,
                                },
                                mouse_state: self.mouse_states.toggle_secrets_tooltip.clone(),
                                detail: None,
                            });
                        } else {
                            links.push(GridTooltipLink {
                                text: crate::t!("terminal-hide-secret"),
                                action: TerminalAction::ToggleGridSecret {
                                    handle,
                                    show_secret: false,
                                },
                                mouse_state: self.mouse_states.toggle_secrets_tooltip.clone(),
                                detail: None,
                            })
                        }
                    }

                    links.push(GridTooltipLink {
                        text: crate::t!("terminal-copy-secret"),
                        action: TerminalAction::CopyGridSecret(handle),
                        mouse_state: self.mouse_states.copy_secrets_tooltip.clone(),
                        detail: None,
                    });
                }
                SecretTooltip::RichContent {
                    tooltip,
                    is_agent_mode,
                } => {
                    let tooltip_info = tooltip;
                    // Position the tooltip above the first cell in the secret.
                    element_id = tooltip_info.position_id.to_owned();
                    is_agent_conversation = *is_agent_mode;

                    if matches!(get_secret_obfuscation_mode(app), ObfuscateSecrets::Yes) {
                        let is_obfuscated = tooltip_info.is_obfuscated;

                        if is_obfuscated {
                            links.push(GridTooltipLink {
                                text: crate::t!("terminal-reveal-secret"),
                                action: TerminalAction::ToggleRichContentSecret {
                                    rich_content_tooltip_info: tooltip_info.clone(),
                                    show_secret: true,
                                },
                                mouse_state: self.mouse_states.toggle_secrets_tooltip.clone(),
                                detail: None,
                            });
                        } else {
                            links.push(GridTooltipLink {
                                text: crate::t!("terminal-hide-secret"),
                                action: TerminalAction::ToggleRichContentSecret {
                                    rich_content_tooltip_info: tooltip_info.clone(),
                                    show_secret: false,
                                },
                                mouse_state: self.mouse_states.toggle_secrets_tooltip.clone(),
                                detail: None,
                            })
                        }
                    }

                    links.push(GridTooltipLink {
                        text: crate::t!("terminal-copy-secret"),
                        action: TerminalAction::CopyRichContentSecret(tooltip_info.clone()),
                        mouse_state: self.mouse_states.copy_secrets_tooltip.clone(),
                        detail: None,
                    });
                }
            }
        }

        #[cfg_attr(not(feature = "local_fs"), allow(unused_mut))]
        if let Some(link) = &self.open_grid_link_tool_tip {
            let mut open_in_zap_ng = None;
            let mut show_in_file_explorer = None;
            let modifier = directly_open_link_keybinding_string();
            let mut detail = Some(format!("[{modifier} Click]"));
            #[cfg(feature = "local_fs")]
            {
                if let GridHighlightedLink::File(file_link) = link {
                    if let Some(path) = file_link.get_inner().absolute_path() {
                        open_in_zap_ng = open_in_zap_ng_tooltip(
                            path.clone(),
                            file_link.get_inner().line_and_column_num,
                            &mut detail,
                            self.mouse_states.open_in_zap_ng_tooltip.clone(),
                            app,
                        );
                        show_in_file_explorer = Some(show_in_file_explorer_tooltip(
                            path,
                            self.mouse_states.show_in_file_explorer_tooltip.clone(),
                        ));
                    }
                }
            }

            links.push(GridTooltipLink {
                text: link.tooltip_text().to_owned(),
                action: TerminalAction::OpenGridLink(link.clone()),
                mouse_state: self.mouse_states.grid_link_tooltip.clone(),
                detail,
            });

            links.extend(open_in_zap_ng);
            links.extend(show_in_file_explorer);
        }

        #[cfg_attr(not(feature = "local_fs"), allow(unused_mut))]
        if let Some(tooltip_info) = &self.open_rich_content_link_tool_tip {
            element_id = tooltip_info.position_id.to_owned();
            let mut open_in_zap_ng = None;
            let mut show_in_file_explorer = None;
            let modifier_string = directly_open_link_keybinding_string();
            let mut detail = Some(format!("[{modifier_string} Click]"));

            #[cfg(feature = "local_fs")]
            {
                if let RichContentLink::FilePath {
                    absolute_path,
                    line_and_column_num,
                    ..
                } = &tooltip_info.link
                {
                    open_in_zap_ng = open_in_zap_ng_tooltip(
                        absolute_path.clone(),
                        *line_and_column_num,
                        &mut detail,
                        self.mouse_states.open_in_zap_ng_tooltip.clone(),
                        app,
                    );
                    show_in_file_explorer = Some(show_in_file_explorer_tooltip(
                        absolute_path.clone(),
                        self.mouse_states.show_in_file_explorer_tooltip.clone(),
                    ));
                }
            }

            links.push(GridTooltipLink {
                text: tooltip_info.link.tooltip_text().to_owned(),
                action: TerminalAction::OpenRichContentLink(tooltip_info.link.clone()),
                mouse_state: self.mouse_states.rich_content_link_tooltip.clone(),
                detail,
            });

            links.extend(open_in_zap_ng);
            links.extend(show_in_file_explorer);
        }

        let secret_redaction = get_secret_obfuscation_mode(app);

        // Get the secret level from the current tooltip
        let secret_level = self.open_secret_tool_tip.as_ref().and_then(|tooltip| {
            match tooltip {
                SecretTooltip::Grid { tooltip, .. } => {
                    // For grid secrets, get the secret level from the secret itself
                    model
                        .secret_from_handle(tooltip)
                        .map(|secret| secret.secret_level())
                }
                SecretTooltip::RichContent { tooltip, .. } => Some(tooltip.secret_level),
            }
        });

        let redaction = match (
            self.open_secret_tool_tip.is_some(),
            secret_redaction.should_redact_secret(),
            is_agent_conversation,
        ) {
            (true, true, true) => TooltipRedaction::SecretNotSentToLLMMessaging { secret_level },
            (true, true, false) => {
                TooltipRedaction::SecretWillNotBeSentToLLMMessaging { secret_level }
            }
            (_, _, _) => TooltipRedaction::NoRedaction,
        };
        stack.add_positioned_overlay_child(
            render_tooltip(
                links,
                self.link_image_preview.as_ref(),
                redaction,
                appearance,
                app,
            ),
            OffsetPositioning::offset_from_save_position_element(
                element_id,
                // Add a small buffer between the tooltip and the top of the cell.
                vec2f(0., -2.),
                PositionedElementOffsetBounds::ParentByPosition,
                PositionedElementAnchor::TopLeft,
                ChildAnchor::BottomLeft,
            ),
        );
    }
}

impl TerminalView {
    /// The file the open link tooltip leads to and the session its path belongs to, when it
    /// is an image.
    fn link_tooltip_image_path(&self) -> Option<(PathBuf, Option<SessionId>)> {
        #[cfg(feature = "local_fs")]
        {
            let (path, session_id) =
                if let Some(GridHighlightedLink::File(file_link)) = &self.open_grid_link_tool_tip {
                    (
                        file_link.get_inner().absolute_path()?,
                        self.session_id_for_grid_link(file_link),
                    )
                } else if let Some(RichContentLinkTooltipInfo {
                    link: RichContentLink::FilePath { absolute_path, .. },
                    ..
                }) = &self.open_rich_content_link_tool_tip
                {
                    // Rich content has no block of its own: its paths belong to the active session.
                    (absolute_path.clone(), self.active_block_session_id())
                } else {
                    return None;
                };
            if is_supported_image_file(&path) {
                return Some((path, session_id));
            }
        }
        None
    }

    /// Brings the image preview in line with the link tooltip that is open. Call it whenever
    /// a link tooltip opens or closes: it reads the file's metadata and header, which
    /// rendering must not do.
    pub(super) fn refresh_link_image_preview(&mut self, ctx: &mut ViewContext<Self>) {
        let Some((path, session_id)) = self.link_tooltip_image_path() else {
            self.set_link_image_preview(None, ctx);
            return;
        };
        if self
            .link_image_preview
            .as_ref()
            .is_some_and(|preview| preview.path == path)
        {
            return;
        }

        // A path printed in a remote session names a file on that host.
        #[cfg(all(
            feature = "local_tty",
            feature = "local_fs",
            not(target_family = "wasm")
        ))]
        if let Some(host_id) = self.remote_host_id_for_session(session_id, ctx) {
            self.show_remote_link_image_preview(host_id, path, ctx);
            return;
        }
        #[cfg(not(all(
            feature = "local_tty",
            feature = "local_fs",
            not(target_family = "wasm")
        )))]
        let _ = session_id;

        // The version makes a screenshot rewritten at the same path load anew; a file that
        // is gone has none, and gets no preview.
        let preview = LocalFileContentVersion::for_path(&path).map(|version| {
            let (width, height) = image_preview_size(image::image_dimensions(&path).ok());
            let source = AssetSource::LocalFile {
                path: path.to_string_lossy().into_owned(),
                content_version: Some(version),
            };
            LinkImagePreview {
                path,
                image: Some((source, width, height)),
            }
        });
        self.set_link_image_preview(preview, ctx);
    }

    /// Replaces the image preview, dropping the image of the one it replaces when that was
    /// put into the asset cache for it.
    pub(super) fn set_link_image_preview(
        &mut self,
        preview: Option<LinkImagePreview>,
        ctx: &mut ViewContext<Self>,
    ) {
        let previous = std::mem::replace(&mut self.link_image_preview, preview);
        if let Some(LinkImagePreview {
            image: Some((AssetSource::Raw { id }, _, _)),
            ..
        }) = previous
        {
            AssetCache::handle(ctx).update(ctx, |cache, ctx| {
                cache.remove_raw_asset::<ImageType>(&id, ctx);
            });
        }
    }
}

fn render_tooltip(
    tooltip_links: impl IntoIterator<Item = GridTooltipLink>,
    image_preview: Option<&LinkImagePreview>,
    redaction: TooltipRedaction,
    appearance: &Appearance,
    app: &AppContext,
) -> Box<dyn Element> {
    // Convert GridTooltipLink to shared TooltipLink
    let shared_links = tooltip_links.into_iter().map(|link| {
        let action = link.action;
        TooltipLink {
            text: link.text,
            on_click: move |ctx: &mut EventContext| {
                ctx.dispatch_typed_action(action.clone());
            },
            detail: link.detail,
            mouse_state: link.mouse_state,
        }
    });

    let tooltip_content =
        crate::util::tooltips::render_tooltip(shared_links, redaction, appearance, app);

    let tooltip_content = match image_preview.and_then(|preview| preview.image.as_ref()) {
        Some((source, width, height)) => {
            let image = ConstrainedBox::new(
                Image::new(source.clone(), CacheOption::BySize)
                    .contain()
                    .finish(),
            )
            .with_width(*width)
            .with_height(*height)
            .finish();
            Flex::column()
                .with_child(
                    Container::new(image)
                        .with_uniform_padding(4.)
                        .with_margin_bottom(2.)
                        .with_background(appearance.theme().tooltip_background())
                        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
                        .with_border(Border::all(1.).with_border_fill(appearance.theme().outline()))
                        .finish(),
                )
                .with_child(tooltip_content)
                .finish()
        }
        None => tooltip_content,
    };

    Dismiss::new(tooltip_content)
        .on_dismiss(|ctx, _app| {
            ctx.dispatch_typed_action(TerminalAction::MaybeDismissToolTip {
                from_keybinding: false,
            })
        })
        .finish()
}
