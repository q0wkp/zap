// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

//! Offering, on the first start, to import a Zap install through the plugin `import-zap`.
//!
//! The import replaces the files ZapNG works with, so it doesn't run while they are open: on
//! accepting, the call is left for the next start and ZapNG quits.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use zap_ng_ui::modals::{AlertDialogWithCallbacks, ModalButton};
use zap_ng_ui::platform::TerminationMode;
use zap_ng_ui::ViewContext;

use super::{defer_call, work_dir, DeferredCall, Plugins};
use crate::workspace::Workspace;

const PLUGIN_ID: &str = "import-zap";

/// The keychain service Zap stored its secrets under.
pub(super) const ZAP_KEYCHAIN_SERVICE: &str = "dev.zap.Zap";

/// The file whose presence says the user was asked already.
const ASKED_FILE_NAME: &str = "asked";

/// How long looking for a Zap install may take.
const DETECT_TIMEOUT: Duration = Duration::from_secs(60);

/// Whether a window of this run already looks for a Zap install.
static OFFERED_THIS_RUN: AtomicBool = AtomicBool::new(false);

fn asked_file() -> std::path::PathBuf {
    work_dir().join(PLUGIN_ID).join(ASKED_FILE_NAME)
}

fn mark_asked() {
    let path = asked_file();
    let result = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&path, ""));
    if let Err(error) = result {
        log::warn!("Can't write {}: {error}", path.display());
    }
}

/// The directories of a Zap install on this machine, as the plugin finds them; `None` when the
/// plugin could not be asked.
fn detect() -> Option<Vec<String>> {
    let found = Plugins::load()
        .and_then(|plugins| plugins.call(PLUGIN_ID, "detect", "", DETECT_TIMEOUT))
        .and_then(|answer| Ok(serde_json::from_str::<Vec<String>>(&answer)?));
    match found {
        Ok(found) => Some(found),
        Err(error) => {
            // Without the plugin there is nothing to offer.
            log::info!("Not looking for a Zap install: {error:#}");
            None
        }
    }
}

/// Asks once, when a Zap install is found, whether to import it. Looking for it happens in the
/// background; the question appears in the window that was opened first.
pub fn offer_zap_import(ctx: &mut ViewContext<Workspace>) {
    if OFFERED_THIS_RUN.swap(true, Ordering::Relaxed) || asked_file().exists() {
        return;
    }
    ctx.spawn(async { detect() }, |workspace, found, ctx| {
        let Some(found) = found else { return };
        if found.is_empty() {
            // Nothing to import, and nothing to look for on the starts to come.
            mark_asked();
            return;
        }
        let dialog = AlertDialogWithCallbacks::for_app(
            crate::t!("zap-import-offer-title"),
            crate::t!("zap-import-offer-body"),
            vec![
                ModalButton::for_app(crate::t!("zap-import-offer-accept"), |app| {
                    mark_asked();
                    let payload = serde_json::json!({
                        "settings_routes": crate::settings::zap_ng_file::ZAP_NG_SETTINGS,
                    });
                    match defer_call(DeferredCall {
                        plugin: PLUGIN_ID.to_owned(),
                        hook: "import".to_owned(),
                        payload: payload.to_string(),
                    }) {
                        Ok(()) => app.terminate_app(TerminationMode::ForceTerminate, None),
                        Err(error) => {
                            log::warn!("Can't leave the import for the next start: {error:#}")
                        }
                    }
                }),
                ModalButton::for_app(crate::t!("zap-import-offer-decline"), |_| mark_asked()),
            ],
            |_| {},
        );
        workspace.show_native_modal(dialog, ctx);
    });
}
