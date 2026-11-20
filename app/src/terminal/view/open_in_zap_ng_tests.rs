// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use std::path::Path;

use zap_ng_completer::completer::TopLevelCommandCaseSensitivity;
use zap_ng_util::path::EscapeChar;

use super::{check_openable_in_zap_ng, OpenablePath};
use crate::util::openable_file_type::OpenableFileType;

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// The root of the Cargo workspace.
fn workspace_root() -> &'static Path {
    Path::new(MANIFEST_DIR)
        .parent()
        .expect("Tests run from the app crate root")
}

#[test]
fn test_single_markdown_file() {
    let result = async_io::block_on(check_openable_in_zap_ng(
        "glow README.md".to_string(),
        Some(workspace_root().to_string_lossy().to_string()),
        TopLevelCommandCaseSensitivity::CaseInsensitive,
        EscapeChar::Backslash,
    ));
    assert_eq!(
        result,
        Some(OpenablePath {
            path: workspace_root().join("README.md"),
            file_type: OpenableFileType::Markdown,
        })
    );
}

#[test]
fn test_any_text_file_type_supported() {
    let result = async_io::block_on(check_openable_in_zap_ng(
        "cat app/assets/bundled/svg/file.svg".to_string(),
        Some(workspace_root().to_string_lossy().to_string()),
        TopLevelCommandCaseSensitivity::CaseInsensitive,
        EscapeChar::Backslash,
    ));
    // .svg is a non-markdown file, and we allow opening it in the ZapNG Editor (as a "code" file).
    assert_eq!(
        result,
        Some(OpenablePath {
            path: workspace_root().join("app/assets/bundled/svg/file.svg"),
            file_type: OpenableFileType::Text,
        })
    );
}

#[test]
fn test_unsupported_binary_file_type() {
    let result = async_io::block_on(check_openable_in_zap_ng(
        "cat app/assets/bundled/png/blue.png".to_string(),
        Some(workspace_root().to_string_lossy().to_string()),
        TopLevelCommandCaseSensitivity::CaseInsensitive,
        EscapeChar::Backslash,
    ));
    // .png is not a supported file type (binary file), so this should return None
    assert_eq!(result, None);
}

#[test]
fn test_nonexistent_markdown_file() {
    let result = async_io::block_on(check_openable_in_zap_ng(
        "cat nonexistent.md".to_string(),
        Some(workspace_root().to_string_lossy().to_string()),
        TopLevelCommandCaseSensitivity::CaseInsensitive,
        EscapeChar::Backslash,
    ));
    assert_eq!(result, None);
}

#[test]
fn test_compound_command() {
    let result = async_io::block_on(check_openable_in_zap_ng(
        "cd somedir && less -R README.md && cd ..".to_string(),
        Some(workspace_root().to_string_lossy().to_string()),
        TopLevelCommandCaseSensitivity::CaseInsensitive,
        EscapeChar::Backslash,
    ));
    assert_eq!(
        result,
        Some(OpenablePath {
            path: workspace_root().join("README.md"),
            file_type: OpenableFileType::Markdown,
        })
    );
}

#[test]
fn test_single_code_file() {
    let result = async_io::block_on(check_openable_in_zap_ng(
        "cat app/src/bin/oss.rs".to_string(),
        Some(workspace_root().to_string_lossy().to_string()),
        TopLevelCommandCaseSensitivity::CaseInsensitive,
        EscapeChar::Backslash,
    ));
    assert_eq!(
        result,
        Some(OpenablePath {
            path: workspace_root().join("app/src/bin/oss.rs"),
            file_type: OpenableFileType::Code,
        })
    );
}

#[test]
fn test_nonexistent_code_file() {
    let result = async_io::block_on(check_openable_in_zap_ng(
        "cat nonexistent.rs".to_string(),
        Some(workspace_root().to_string_lossy().to_string()),
        TopLevelCommandCaseSensitivity::CaseInsensitive,
        EscapeChar::Backslash,
    ));
    assert_eq!(result, None);
}
