# AGENTS.md

A map of this repository for coding agents: how the code is layered, where common changes start, and the rules to
follow when changing it.

## Overview

zap is a Rust agentic terminal forked from Warp. It is built on Warp's own UI framework, WarpUI, and combines
terminal emulation, AI agents, cloud sync (Drive), code review, completions, notebooks, settings and IPC.

| Path | Contents |
| --- | --- |
| `app/` | The main binary crate (`warp`): wires up every subsystem, the UI, platform glue and persistence |
| `crates/` | The library crates, one concern each; most describe themselves in a `//!` comment at the top of `src/lib.rs` |
| `command-signatures-v2/` | A separate project, excluded from workspace test runs |
| `script/` | Bootstrap, build and presubmit scripts |
| `resources/` | Fonts, icons, shell integration scripts, shaders and other runtime assets |
| `specs/` | Product and technical specs |
| `.agents/skills/` | Agent skills inherited from Warp, such as adding a feature flag or writing unit tests |

`crates/warpui` and `crates/warpui_core` are MIT-licensed; everything else is AGPL-3.0-only.

Package names that differ from their directory: `crates/editor` is `warp_editor`, `crates/isolation_platform` is
`warp_isolation_platform`, `crates/managed_secrets` is `warp_managed_secrets`, and `crates/virtual_fs` is `virtual-fs`.

Crates zap added on top of Warp: `warp_ssh_manager` (the SSH manager's saved servers, keychain credentials and command
building; its UI is in `app/src/ssh_manager/`), `zap_sftp` (SFTP sessions over `ssh2`) and `zap_sync` (shared cloud
sync infrastructure).

## Architecture

The code falls into four layers. Find the layer a change belongs to first, and never make a lower layer depend on a
higher one.

```
app/  (main binary: wiring, entry points, platform glue, persistence, root views)
  ↑
product crates: ai / computer_use / vim / onboarding /
                warp_completer / lsp / languages / ...
  ↑
framework crates: warpui / warpui_core / warpui_extras / editor /
                  ui_components / sum_tree / syntax_tree
  ↑
infrastructure crates: warp_core / warp_util / http_client /
                       websocket / ipc / jsonrpc / persistence /
                       managed_secrets / virtual_fs / watcher / asset_cache / ...
```

Key patterns:

1. **Entity-Handle system**: the global `App` owns every view and model entity. Views refer to each other through
   `ViewHandle<T>` instead of owning them.
2. **Elements and actions**: the UI is a declarative element tree (Flutter-style) plus an action system for events.
3. **Cross-platform**: native macOS, Windows and Linux plus a WASM target; platform code is isolated with `#[cfg(...)]`.
4. **AI**: Agent Mode and context indexing live in `app/src/ai/` and `crates/ai/`.
5. **Cloud sync**: Drive syncs objects across devices; see `app/src/drive/` and `crates/warp_files/`.

## Where to start

| Change | Start in |
| --- | --- |
| Terminal grid or shell integration | `crates/warp_terminal/src/`, together with `app/src/terminal/` |
| Agent UI or conversations | `app/src/ai/`, the largest subtree: grep for the subtopic (`agent_*`, `conversation_*`, `cloud_agent_*`, `mcp`, `tool_*`) |
| AI models or the tool-call protocol | `crates/ai/` |
| Command completion | `crates/warp_completer/` (some of it is behind `--features v2`) |
| A new setting | `crates/settings`, `crates/settings_value*`; the UI is in `app/src/settings_view/` |
| A feature flag | `crates/warp_features/src/lib.rs` and where it is used |
| Cloud-synced objects | `crates/warp_files/`, `app/src/drive/`, `app/src/cloud_object/` |
| Persisted data | `crates/persistence/` (see Database below) |
| A new binary | `app/src/bin/` |
| Platform-specific code | `#[cfg(target_os = "...")]`; UI platform glue is in `app/src/platform/` |
| Vim mode | `crates/vim/`, `app/src/vim_registers.rs` |
| Notebooks and workflows | `app/src/notebooks/`, `app/src/workflows/`, `crates/warp_files/` |
| Spawning processes | `crates/command/` |
| File search and watching | `crates/repo_metadata/`, `crates/watcher/`, `crates/warp_ripgrep/` |

## Rules

### General

- Keep changes precise: every changed line should trace back to the request. Don't "improve" unrelated code,
  comments or formatting along the way, and don't delete or rewrite existing comments in an unrelated change.
- `cargo check` must pass before opening a PR or pushing commits; it is the only required check.
- Put worktrees in `.worktrees/<name>/`, which is gitignored.

### Rust style

- Leave out type annotations that aren't needed, especially on closure parameters.
- Import at the top of the file instead of writing long qualified paths. Inside `#[cfg]` branches, a local import or a
  one-off full path is fine.
- A context parameter (`AppContext`, `ViewContext`, `ModelContext`) is named `ctx` and goes last, unless the function
  also takes a closure, which then goes last.
- Remove unused parameters entirely instead of prefixing them with `_`, and update every call site.
- Use inline format arguments (`"{x}"`, not `"{}", x`) in `println!`, `format!` and similar macros, for Clippy's
  `uninlined_format_args`.
- Avoid the `_` wildcard in `match` wherever possible, so that a new enum variant has to be handled explicitly.

### Terminal model locking

- `TerminalModel::lock()` deadlocks easily, which freezes the UI (a beach ball on macOS).
- Before adding a `model.lock()`, make sure nothing up the call stack already holds the lock. Prefer passing the
  locked model down the stack over locking it again.
- Keep the lock scope short, and don't call anything that might lock the model again while holding it.

### Feature flags

- Add a variant to the `FeatureFlag` enum in `crates/warp_features/src/lib.rs` (re-exported as
  `warp_core::features`), and list it in `DOGFOOD_FLAGS`, `PREVIEW_FLAGS` or `RELEASE_FLAGS` as needed.
- Prefer the runtime check `FeatureFlag::Xxx.is_enabled()` over `#[cfg(...)]`. Use `cfg` only where the code can't
  compile without it (platform code, optional dependencies).
- Gate a whole product feature, not each call site, and gate its UI entry points with the same flag. Once the feature
  is stable, remove the flag and the dead branches.

### Database

- Diesel with SQLite.
- Change the schema only through a migration: a new directory with `up.sql` and `down.sql` under
  `crates/persistence/migrations/`. `crates/persistence/src/schema.rs` is generated by `diesel print-schema` (with
  `crates/persistence/schema.patch` applied, see `diesel.toml`), so don't edit it by hand.

### Tests

- Run them with `cargo nextest run --no-fail-fast --workspace --exclude command-signatures-v2`.
- Unit tests go in a separate file next to the module, usually `<file>_tests.rs` (some modules use `_test.rs`;
  follow the module's existing file), included at the end of the module with:

  ```rust
  #[cfg(test)]
  #[path = "filename_tests.rs"]
  mod tests;
  ```

- Integration tests use the framework in `crates/integration/`; examples are in `app/src/integration_testing/`.

### Spawning processes

- Never call `std::process::Command::new(...)` directly; on Windows it opens a console window. Use `crates/command`.
