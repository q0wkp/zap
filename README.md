<p align="center">
  <img src="./assets/zap-logo.svg" alt="Zap logo" width="96">
</p>

# Zap

[![License](https://img.shields.io/badge/License-AGPL--3.0--only%20AND%20MIT-blue.svg)](#license)
[![Release](https://img.shields.io/github/v/release/q0wkp/zap)](https://github.com/q0wkp/zap/releases/latest)

My own build of [Zap](https://github.com/zerx-lab/zap), the terminal forked from
[Warp](https://github.com/warpdotdev/warp), kept for my personal use. It is not affiliated with or supported by the
maintainers of Zap or Warp.

## Changes from Upstream

- **zmx sessions**: CLI agents running inside a [zmx](https://github.com/lutianyu2001/zmx) session are detected, from
  the variables zmx sets, the foreground process it reports on attach, or the attach command line. SSH hosts can name a
  zmx session to attach to after connecting. ([#1](https://github.com/q0wkp/zap/pull/1))
- **Tab key setting**: the settings dropdown no longer shows "User defined" for the Tab key behavior after it is
  refreshed. ([#2](https://github.com/q0wkp/zap/pull/2); also proposed upstream as PR 345 in zerx-lab/zap)
- **Releases and auto-update**: releases are published from this repository, and auto-update, the about page and the
  remote server download point here, so a build of this fork never replaces itself with an upstream one. Tags are the
  next upstream version plus `-mod.N` (upstream's `v2026.07.09.1` becomes `v2026.07.09.2-mod.1`). A release is created
  only once every platform has built, for macOS (Apple silicon and Intel) and Linux x86_64; Windows is no longer built.
- **Linux packages**: the release builds the Arch package, which a wrong makepkg option had kept from ever being built,
  and both it and the `.deb` declare the libraries the binary links against (ALSA, liblzma), so they start on systems
  without them.
- **Package managers**: `zap` for pacman, `zap-deb` for Pacstall and the cask `zap-terminal` for Homebrew, from
  [q0wkp/pkgs](https://github.com/q0wkp/pkgs), which says how to add them.
- **Workflows**: the release workflow is this fork's own, and asks q0wkp/pkgs to package each release once it is out;
  upstream's other workflows, its issue and pull request templates and its Dependabot settings are gone.

## License

The UI framework (the `warpui_core` and `warpui` crates) is licensed under the [MIT license](LICENSE-MIT).

The rest of the code in this repository is licensed under the [AGPL v3](LICENSE-AGPL).
