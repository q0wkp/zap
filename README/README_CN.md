<p align="center">
  <img src="../assets/zap-ng-logo.svg" alt="ZapNG logo" width="96">
</p>

# ZapNG

<p align="right">
    <a href="../README.md">English</a> | <strong>中文</strong>
</p>

[![License](https://img.shields.io/badge/License-AGPL%20v3%20AND%20MIT%20AND%20Apache%202.0%20AND%20BSD--2--Clause%20AND%20OFL--1.1%20AND%20Bitstream--Vera-blue.svg)](#许可证)
[![Release](https://img.shields.io/github/v/release/q0wkp/zap-ng)](https://github.com/q0wkp/zap-ng/releases/latest)

ZapNG 是一个本地优先的智能体终端，支持 Linux 和 macOS，由 [Warp](https://github.com/warpdotdev/warp) 分支而来。

## 目录

- [概述](#概述)
- [功能特性](#功能特性)
- [配置](#配置)
- [插件](#插件)
- [从 Zap 导入](#从-zap-导入)
- [兼容性](#兼容性)
- [相对原版 Zap 的改动](#相对原版-zap-的改动)
- [免责声明](#免责声明)
- [许可证](#许可证)
- [作者](#作者)

## 概述

ZapNG 最初是我自己构建的 [zerx-lab/zap](https://github.com/zerx-lab/zap)，也就是原版 Zap：一个不依赖 Warp 云服务的 Warp 分支。原作者在 2026-11-17 归档了那个仓库，此后本项目独立开发；2026-11-19 之前它叫 Zap。

ZapNG 的目标是成为 Warp 的超集（Zap 有意移除的 Warp 功能除外）：它新增的东西在设置和代码上都与 Warp 原有的部分分开，以便继续吸收 Warp 的修复。

## 功能特性

- **自己的模型服务商**：智能体使用你配置的服务商，内置模型目录，支持 Ollama，可以查看发出的请求、按 profile 覆盖提示词，并对发出的内容做密钥脱敏。
- **其他厂商的 CLI agent**：CLI agent 在终端里运行，并使用 ZapNG 的智能体界面，在 zmx 会话里也一样。
- **SSH 管理器**：保存主机，凭据存在系统钥匙串里；通过 SFTP 浏览文件；列出主机上的 zmx 会话并接入。
- **图片预览**：点击指向图片的路径，链接提示框里会显示缩略图，SSH 主机上的路径也可以。
- **插件**：在沙箱里运行的 WebAssembly 组件，见[插件](#插件)。
- **从 Zap 导入**：见[从 Zap 导入](#从-zap-导入)。
- **界面语言**：英文、中文、日文和俄文。
- **来自 Warp 的修复**：Zap 分支之后 Warp 做的修复会被移植过来，列在 [CHANGES.csv](../CHANGES.csv) 里。

<!-- ## 安装：等安装包改用新名字后再写。 -->

## 配置

ZapNG 的设置保存在同一目录下的两个文件里，Linux 上是 `~/.config/zap-ng`，macOS 上是 `~/.zap-ng`：

| 文件 | 内容 |
|------|------|
| `settings.toml` | Warp 也有的设置，路径与 Warp 的一致，因此这个文件可以和 Warp 的互换 |
| `zap-ng.toml` | 只有 ZapNG 才有的设置（模型服务商、BYOP、CLI agent、界面语言和字体、代理、SSH 管理器、同步、更新），以及与插件有关的内容 |

两个文件都被监视：改动无需重启即可生效。

## 插件

插件是一个目录，里面有清单 `plugin.toml` 和一个按 [`crates/zap_ng_plugin/wit/plugin.wit`](../crates/zap_ng_plugin/wit/plugin.wit) 中的接口构建的 WebAssembly 组件。ZapNG 加载随它附带的插件和 `~/.zap-ng/plugins` 里的插件，并在清单列出的钩子上调用它们。

```toml
id = "example"
name = "Example"
version = "0.1.0"
api = "0.1"
wasm = "example.wasm"
hooks = ["<hook>"]
```

- 插件在沙箱里运行：它只能看到自己的工作目录和宿主接口提供的函数。
- 工作目录之外的目录和系统钥匙串，只开放给随 ZapNG 附带的插件。
- 插件可以提供一个供其他插件依赖的名字（`provides = ["runtime:<name>"]`、`requires = ["runtime:<name>"]`），例如某种语言的运行时，并先于依赖它的插件加载。

`zap-ng.toml` 可以启用或停用插件，并对它进行配置：

```toml
[plugins.<id>]
enabled = false

[plugins.<id>.config]
<key> = "<value>"
```

[`plugins/`](../plugins) 里的插件用 `script/build_plugins <DEST>` 构建，需要 Rust 的 `wasm32-wasip2` 目标。

## 从 Zap 导入

首次启动时，ZapNG 会查找 Zap 的安装，找到后询问是否导入。导入会把 Zap 的设置、文件、历史记录和数据复制到 ZapNG 的目录，把属于 `zap-ng.toml` 的设置挪过去，并复制保存在系统钥匙串里的密钥。Zap 自己的文件不会被改动。

导入会替换 ZapNG 已有的内容，因此适用于还没用过的 ZapNG。接受询问后，导入在下次启动 ZapNG 时执行。在 macOS 上，Zap 保存在系统偏好设置里的那部分设置不会被导入。

## 兼容性

发布版本面向：

- Linux：x86_64
- macOS：aarch64 和 x86_64

不支持 Windows。

## 相对原版 Zap 的改动

[CHANGES.csv](../CHANGES.csv) 列出了本项目自 [zerx-lab/zap](https://github.com/zerx-lab/zap) 归档以来的改动，每行一项功能，附带对应的提交和 pull request。取自其他项目的改动（Warp 的修复，其中大部分是 [grizmin/zap](https://github.com/grizmin/zap) 适配到 Zap 上的版本）注明了作者、来源和协议，在此致谢。

## 免责声明

This project is not affiliated with, endorsed by, or sponsored by Denver Technologies, Inc. "Warp" is a trademark of Denver Technologies, Inc. and is referenced here solely for descriptive purposes.

This project is not affiliated with, endorsed by, or sponsored by Zap or its maintainers. "Zap" refers to the software developed by zerx-lab (https://github.com/zerx-lab/zap) and is referenced here solely for descriptive purposes.

## 许可证

Copyright 2026 Tianyu (Sky) Lu

Except for the third-party code listed below, this repository is 
licensed under the GNU Affero General Public License, Version 3 (the 
"License"); you can redistribute and/or modify these files under the 
terms of version 3 of the License. You may obtain a copy of the License at

```plaintext
https://www.gnu.org/licenses/agpl-3.0.html
```

This program is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the License
for more details.

### Third-Party Code

This repository includes the following third-party code, which remains under its respective original licenses:

| Code | Included As | License | Copyright | Modified |
|------|-------------|---------|-----------|----------|
| [Warp](https://github.com/warpdotdev/warp) | Forked from<br>Cherry-picked commits | [AGPL-3.0-only](../LICENSES/Warp-AGPL-3.0-only.txt) | Copyright (C) 2020-2026 Denver Technologies, Inc. | Yes |
| [Warp](https://github.com/warpdotdev/warp) UI framework | Forked from, in `crates/zap_ng_ui` and `crates/zap_ng_ui_core` | [MIT](../LICENSES/Warp-MIT.txt) | Copyright (C) 2020-2026 Denver Technologies, Inc. | Yes |
| [Zap](https://github.com/zerx-lab/zap) | Forked from | [AGPL-3.0-only](../LICENSES/Warp-AGPL-3.0-only.txt) | Copyright (C) 2020-2026 Denver Technologies, Inc. | Yes |
| [grizmin/zap](https://github.com/grizmin/zap) | Cherry-picked commits | [AGPL-3.0-only](../LICENSES/Warp-AGPL-3.0-only.txt) | Copyright (C) 2020-2026 Denver Technologies, Inc. | Yes |
| [bash-preexec](https://github.com/rcaloras/bash-preexec) | Copied into `app/assets/bundled/bootstrap` | [MIT](../app/assets/bundled/bootstrap/bash-preexec-LICENSE.md) | Copyright (c) 2017 Ryan Caloras and contributors (see https://github.com/rcaloras/bash-preexec) | Yes |
| [GitHub Desktop](https://github.com/desktop/desktop) | Copied into `app/src/code_review/diff_state.rs` | [MIT](../app/src/code_review/GITHUB-DESKTOP-LICENSE) | Copyright (c) GitHub, Inc. | Yes |
| [Reachability](https://github.com/tonymillion/Reachability) | Copied into `crates/zap_ng_ui/src/platform/mac/objc` | [BSD-2-Clause](../LICENSES/Reachability-BSD-2-Clause.txt) | Copyright (c) 2011, Tony Million. | Yes |
| [tree-sitter-ruby](https://github.com/tree-sitter/tree-sitter-ruby) | Copied into `crates/languages/grammars/ruby/locals.scm` | [MIT](../LICENSES/tree-sitter-ruby-MIT.txt) | Copyright (c) 2016 Rob Rix | No |
| [Windows Terminal](https://github.com/microsoft/terminal) | Copied into `crates/zap_ng_ui/src/rendering/wgpu/shaders/glyph_shader.wgsl` | [MIT](../LICENSES/WindowsTerminal-MIT.txt) | Copyright (c) Microsoft Corporation. | Yes |
| [Claude API skill](https://github.com/anthropics/skills) | Vendored in `resources/bundled/skills/claude-api` | [Apache-2.0](../resources/bundled/skills/claude-api/LICENSE.txt) | Copyright 2026 Anthropic, PBC. | No |
| [Hack](https://github.com/source-foundry/Hack) | Vendored in `app/assets/bundled/fonts/hack` | [MIT AND Bitstream-Vera](../app/assets/bundled/fonts/hack/LICENSE.md) | Copyright (c) 2018 Source Foundry Authors<br>Copyright (c) 2003 Bitstream Inc. | No |
| [models.dev](https://github.com/anomalyco/models.dev) | Vendored in `app/resources/models_dev` | [MIT](../app/resources/models_dev/LICENSE) | Copyright (c) 2025 models.dev | No |
| [Roboto](https://github.com/googlefonts/roboto-3-classic) | Vendored in `app/assets/bundled/fonts/roboto` | [OFL-1.1](../app/assets/bundled/fonts/roboto/LICENSE.txt) | Copyright 2011 The Roboto Project Authors (https://github.com/googlefonts/roboto-classic) | No |
| [rust-genai](https://github.com/jeremychone/rust-genai) | Vendored in `lib/rust-genai` | [MIT OR Apache-2.0](../lib/rust-genai/LICENSE-MIT) | Copyright (c) 2025 Jeremy Chone | Yes |
| [skill-creator skill](https://github.com/anthropics/skills) | Vendored in `resources/bundled/skills/create-skill` | [Apache-2.0](../resources/bundled/skills/create-skill/LICENSE.txt) | Copyright 2026 Anthropic, PBC. | No |

All other code in this repository, including changes made to the code 
above, is licensed under the GNU Affero General Public License, Version 3. The commit history of 
this repository, together with snippets and file headers marked in the 
source files, identifies which code belongs to which project. Files that 
contain code from more than one project are subject to all applicable 
licenses. Contributions to this repository are accepted only under the 
GNU Affero General Public License, Version 3 ("inbound = outbound" principle).

## 作者

Tianyu (Sky) Lu
