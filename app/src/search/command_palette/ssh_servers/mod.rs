// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

//! Command palette 数据源:SSH 服务器(ZapNG 独有)。
//!
//! 用户在 Ctrl+Shift+P 中按服务器名 / host 模糊匹配,选中 → emit
//! `WorkspaceAction::OpenSshTerminal` 开新 tab 连接(走 SecretInjector 自动
//! 注入密码,跟从 SSH 管理器右键"连接"完全等价)。

pub mod data_source;
pub mod search_item;

pub use data_source::SshServersDataSource;
pub use search_item::SshServerSearchItem;
