// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

//! Imports a Zap install into ZapNG: its directories are copied to ZapNG's, the settings ZapNG
//! keeps in `zap-ng.toml` are moved there, and the secrets saved in the keychain are copied to
//! ZapNG's entry.
//!
//! Nothing of the Zap install is changed. Files ZapNG already has are replaced, so the import
//! is meant for a ZapNG that has not been used yet.

mod import;

pub use import::{detect, import_files, split_settings, Report, DIRECTORIES};

#[cfg(target_arch = "wasm32")]
mod guest {
    use std::path::Path;

    use serde::Deserialize;

    wit_bindgen::generate!({
        path: "../../crates/zap_ng_plugin/wit",
        world: "plugin",
    });

    use zap_ng::plugin::host;

    /// The keychain service Zap stored its secrets under.
    const ZAP_KEYCHAIN_SERVICE: &str = "dev.zap.Zap";
    /// The secrets Zap stored there.
    const SECRET_KEYS: &[&str] = &[
        "AgentProviderSecrets",
        "AiApiKeys",
        "CloudSyncTokens",
        "ProxyPassword",
    ];

    /// What the application sends with the `import` hook.
    #[derive(Deserialize)]
    struct ImportRequest {
        /// The settings kept in `zap-ng.toml`: the path each had in Zap's `settings.toml`, and
        /// its path in `zap-ng.toml`.
        settings_routes: Vec<(String, String)>,
    }

    struct ImportZap;

    impl Guest for ImportZap {
        fn handle(hook: String, payload: String) -> Result<String, String> {
            let home = Path::new("/home");
            match hook.as_str() {
                "detect" => {
                    let found = crate::detect(home);
                    serde_json::to_string(&found).map_err(|error| error.to_string())
                }
                "import" => {
                    let request: ImportRequest =
                        serde_json::from_str(&payload).map_err(|error| error.to_string())?;
                    let mut report = crate::import_files(home, &request.settings_routes);
                    for key in SECRET_KEYS {
                        match host::read_secret(ZAP_KEYCHAIN_SERVICE, key) {
                            Ok(Some(value)) => match host::write_secret(key, &value) {
                                Ok(()) => report.secrets += 1,
                                Err(error) => report.errors.push(format!("secret {key}: {error}")),
                            },
                            Ok(None) => {}
                            Err(error) => report.errors.push(format!("secret {key}: {error}")),
                        }
                    }
                    host::log(
                        host::LogLevel::Info,
                        &format!(
                            "imported {} files, {} settings moved to zap-ng.toml, {} secrets, {} errors",
                            report.files,
                            report.settings_moved,
                            report.secrets,
                            report.errors.len()
                        ),
                    );
                    for error in &report.errors {
                        host::log(host::LogLevel::Warn, error);
                    }
                    serde_json::to_string(&report).map_err(|error| error.to_string())
                }
                other => Err(format!("unknown hook {other}")),
            }
        }
    }

    export!(ImportZap);
}
