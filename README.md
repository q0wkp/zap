<p align="center">
  <img src="./assets/zap-ng-logo.svg" alt="ZapNG logo" width="96">
</p>

# ZapNG

<p align="right">
    <strong>English</strong> | <a href="./README/README_CN.md">中文</a>
</p>

[![License](https://img.shields.io/badge/License-AGPL%20v3%20AND%20MIT%20AND%20Apache%202.0%20AND%20BSD--2--Clause%20AND%20OFL--1.1%20AND%20Bitstream--Vera-blue.svg)](#license)
[![Release](https://img.shields.io/github/v/release/q0wkp/zap-ng)](https://github.com/q0wkp/zap-ng/releases/latest)

ZapNG is a local-first agentic terminal for Linux and macOS, forked from [Warp](https://github.com/warpdotdev/warp).

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Configuration](#configuration)
- [Plugins](#plugins)
- [Importing from Zap](#importing-from-zap)
- [Compatibility](#compatibility)
- [Changes from the Original Zap](#changes-from-the-original-zap)
- [Disclaimer](#disclaimer)
- [License](#license)
- [Author](#author)

## Overview

ZapNG began as my own build of [zerx-lab/zap](https://github.com/zerx-lab/zap), the original Zap, a fork of Warp that works without Warp's cloud services. Its author archived that repository on 2026-11-17, and since then this project is developed independently; it was named Zap until 2026-11-19.

ZapNG aims to be a superset of Warp, apart from the Warp features Zap removed on purpose: what it adds is kept apart from what Warp has, in settings and in code, so that Warp's fixes can still be taken.

## Features

- **Your own model providers**: agents talk to the providers you configure, with a built-in catalog of models, support for Ollama, an inspector for the requests sent, prompt overrides per profile, and redaction of secrets in what is sent.
- **Other vendors' CLI agents**: CLI agents run in the terminal with ZapNG's agent interface, also inside zmx sessions.
- **SSH manager**: saved hosts with their credentials in the system keychain, a file browser over SFTP, and the zmx sessions of a host to attach to.
- **Image previews**: clicking a path to an image shows a thumbnail in the link's tooltip, also for a path on an SSH host.
- **Plugins**: WebAssembly components that run in a sandbox. See [Plugins](#plugins).
- **Import from Zap**: see [Importing from Zap](#importing-from-zap).
- **Interface languages**: English, Chinese, Japanese and Russian.
- **Fixes from Warp**: fixes Warp made after Zap forked from it are ported; [CHANGES.csv](CHANGES.csv) lists them.

<!-- ## Installation: to be written once the packages carry the new name. -->

## Configuration

ZapNG keeps its settings in two files in the same directory, `~/.config/zap-ng` on Linux and `~/.zap-ng` on macOS:

| File | Holds |
|------|-------|
| `settings.toml` | The settings Warp has, under the paths Warp gives them, so that the file stays interchangeable with Warp's |
| `zap-ng.toml` | The settings only ZapNG has (model providers, BYOP, CLI agents, the interface language and fonts, the proxy, the SSH manager, sync, updates), and what concerns plugins |

Both files are watched: a change takes effect without a restart.

## Plugins

A plugin is a directory holding a manifest, `plugin.toml`, and a WebAssembly component built against the interface in [`crates/zap_ng_plugin/wit/plugin.wit`](crates/zap_ng_plugin/wit/plugin.wit). ZapNG loads the plugins shipped with it and the ones in `~/.zap-ng/plugins`, and calls each for the hooks its manifest names.

```toml
id = "example"
name = "Example"
version = "0.1.0"
api = "0.1"
wasm = "example.wasm"
hooks = ["<hook>"]
```

- A plugin runs in a sandbox: it sees a working directory of its own and the functions of the host interface.
- Directories outside its working directory, and the system keychain, are given only to the plugins shipped with ZapNG.
- A plugin can provide a name that others require (`provides = ["runtime:<name>"]`, `requires = ["runtime:<name>"]`), such as a language runtime, and is loaded before them.

`zap-ng.toml` turns a plugin on or off and configures it:

```toml
[plugins.<id>]
enabled = false

[plugins.<id>.config]
<key> = "<value>"
```

The plugins in [`plugins/`](plugins) are built with `script/build_plugins <DEST>`, which needs the Rust target `wasm32-wasip2`.

## Importing from Zap

On its first start ZapNG looks for a Zap install and, when it finds one, asks whether to import it. The import copies Zap's settings, files, history and data to ZapNG's directories, moves the settings that belong in `zap-ng.toml` there, and copies the secrets saved in the system keychain. Zap's own files are not changed.

The import replaces what ZapNG already has, so it is meant for a ZapNG that has not been used yet. It runs when ZapNG is started again after the question is accepted. On macOS, the settings Zap kept in the system preferences are not imported.

## Compatibility

Releases are built for:

- Linux: x86_64
- macOS: aarch64 and x86_64

Windows is intentionally not supported, since I don't use ZapNG on Windows. If you want Windows support back, you
are welcome to do it in a fork of your own: [WINDOWS_REMOVAL.csv](WINDOWS_REMOVAL.csv) lists the commits and pull
requests that removed the Windows code and, for each file, where the commit changed it, which you may find helpful.
The places are the hunks of the commit's diff (`-<line>,<count> +<line>,<count>`, as `git show <commit> -- <file>`
prints them); the Note column says when they refer to another commit.

## Changes from the Original Zap

[CHANGES.csv](CHANGES.csv) lists what this project changed since [zerx-lab/zap](https://github.com/zerx-lab/zap) as
it was archived, one feature per row, with the commits and pull requests that carry it. Changes taken from other
projects (Warp's fixes, most of them as [grizmin/zap](https://github.com/grizmin/zap) adapted them to Zap) name their
authors, where they come from and their license, with thanks.

## Disclaimer

This project is not affiliated with, endorsed by, or sponsored by Denver Technologies, Inc. "Warp" is a trademark of Denver Technologies, Inc. and is referenced here solely for descriptive purposes.

This project is not affiliated with, endorsed by, or sponsored by Zap or its maintainers. "Zap" refers to the software developed by zerx-lab (https://github.com/zerx-lab/zap) and is referenced here solely for descriptive purposes.

The ZapNG logo is a modified version of the Zap logo, which the authors of Zap (https://github.com/zerx-lab/zap) distributed under the GNU Affero General Public License, version 3 (AGPL-3.0-only); its colours were changed.

## License

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
| [Warp](https://github.com/warpdotdev/warp) | Forked from<br>Cherry-picked commits | [AGPL-3.0-only](LICENSES/Warp-AGPL-3.0-only.txt) | Copyright (C) 2020-2026 Denver Technologies, Inc. | Yes |
| [Warp](https://github.com/warpdotdev/warp) UI framework | Forked from, in `crates/zap_ng_ui` and `crates/zap_ng_ui_core` | [MIT](LICENSES/Warp-MIT.txt) | Copyright (C) 2020-2026 Denver Technologies, Inc. | Yes |
| [Zap](https://github.com/zerx-lab/zap) | Forked from | [AGPL-3.0-only](LICENSES/Warp-AGPL-3.0-only.txt) | Copyright (C) 2020-2026 Denver Technologies, Inc. | Yes |
| [grizmin/zap](https://github.com/grizmin/zap) | Cherry-picked commits | [AGPL-3.0-only](LICENSES/Warp-AGPL-3.0-only.txt) | Copyright (C) 2020-2026 Denver Technologies, Inc. | Yes |
| [bash-preexec](https://github.com/rcaloras/bash-preexec) | Copied into `app/assets/bundled/bootstrap` | [MIT](app/assets/bundled/bootstrap/bash-preexec-LICENSE.md) | Copyright (c) 2017 Ryan Caloras and contributors (see https://github.com/rcaloras/bash-preexec) | Yes |
| [GitHub Desktop](https://github.com/desktop/desktop) | Copied into `app/src/code_review/diff_state.rs` | [MIT](app/src/code_review/GITHUB-DESKTOP-LICENSE) | Copyright (c) GitHub, Inc. | Yes |
| [Reachability](https://github.com/tonymillion/Reachability) | Copied into `crates/zap_ng_ui/src/platform/mac/objc` | [BSD-2-Clause](LICENSES/Reachability-BSD-2-Clause.txt) | Copyright (c) 2011, Tony Million. | Yes |
| [tree-sitter-ruby](https://github.com/tree-sitter/tree-sitter-ruby) | Copied into `crates/languages/grammars/ruby/locals.scm` | [MIT](LICENSES/tree-sitter-ruby-MIT.txt) | Copyright (c) 2016 Rob Rix | No |
| [Windows Terminal](https://github.com/microsoft/terminal) | Copied into `crates/zap_ng_ui/src/rendering/wgpu/shaders/glyph_shader.wgsl` | [MIT](LICENSES/WindowsTerminal-MIT.txt) | Copyright (c) Microsoft Corporation. | Yes |
| [Claude API skill](https://github.com/anthropics/skills) | Vendored in `resources/bundled/skills/claude-api` | [Apache-2.0](resources/bundled/skills/claude-api/LICENSE.txt) | Copyright 2026 Anthropic, PBC. | No |
| [Hack](https://github.com/source-foundry/Hack) | Vendored in `app/assets/bundled/fonts/hack` | [MIT AND Bitstream-Vera](app/assets/bundled/fonts/hack/LICENSE.md) | Copyright (c) 2018 Source Foundry Authors<br>Copyright (c) 2003 Bitstream Inc. | No |
| [models.dev](https://github.com/anomalyco/models.dev) | Vendored in `app/resources/models_dev` | [MIT](app/resources/models_dev/LICENSE) | Copyright (c) 2025 models.dev | No |
| [Roboto](https://github.com/googlefonts/roboto-3-classic) | Vendored in `app/assets/bundled/fonts/roboto` | [OFL-1.1](app/assets/bundled/fonts/roboto/LICENSE.txt) | Copyright 2011 The Roboto Project Authors (https://github.com/googlefonts/roboto-classic) | No |
| [rust-genai](https://github.com/jeremychone/rust-genai) | Vendored in `lib/rust-genai` | [MIT OR Apache-2.0](lib/rust-genai/LICENSE-MIT) | Copyright (c) 2025 Jeremy Chone | Yes |
| [skill-creator skill](https://github.com/anthropics/skills) | Vendored in `resources/bundled/skills/create-skill` | [Apache-2.0](resources/bundled/skills/create-skill/LICENSE.txt) | Copyright 2026 Anthropic, PBC. | No |

All other code in this repository, including changes made to the code 
above, is licensed under the GNU Affero General Public License, Version 3. The commit history of 
this repository, together with snippets and file headers marked in the 
source files, identifies which code belongs to which project. Files that 
contain code from more than one project are subject to all applicable 
licenses. Contributions to this repository are accepted only under the 
GNU Affero General Public License, Version 3 ("inbound = outbound" principle).

## Author

Tianyu (Sky) Lu
