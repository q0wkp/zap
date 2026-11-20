// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use super::*;

#[test]
fn yaml_file_type_accepts_both_yaml_and_yml() {
    assert_eq!(FileType::Yaml.extensions(), &["yaml", "yml"]);
}

#[test]
fn markdown_file_type_accepts_md_and_markdown() {
    assert_eq!(FileType::Markdown.extensions(), &["md", "markdown"]);
}
