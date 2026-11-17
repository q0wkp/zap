<p align="center">
  <img src="./assets/zap-logo.svg" alt="Zap logo" width="96">
</p>

# Zap

[![License](https://img.shields.io/badge/License-AGPL--3.0--only%20AND%20MIT-blue.svg)](#license)
[![Release](https://img.shields.io/github/v/release/q0wkp/zap)](https://github.com/q0wkp/zap/releases/latest)

A terminal forked from [Warp](https://github.com/warpdotdev/warp). It began as my own build of
[zerx-lab/zap](https://github.com/zerx-lab/zap), the original Zap. Its author archived that repository on 2026-11-17,
and since then this project is developed independently. It is not affiliated with or supported by the maintainers of
the original Zap or of Warp.

## Changes from the Original Zap

"Upstream" below is zerx-lab/zap as it was archived.


- **zmx sessions**: CLI agents running inside a [zmx](https://github.com/neurosnap/zmx) session are detected, from
  the variables zmx sets, the foreground process it reports on attach, or the attach command line. SSH hosts can name a
  zmx session to attach to after connecting ([#1](https://github.com/q0wkp/zap/pull/1)), and the host's editor
  lists the sessions on the server, with a preview of each, to pick one from
  ([#5](https://github.com/q0wkp/zap/pull/5)).
- **Tab key setting**: the settings dropdown no longer shows "User defined" for the Tab key behavior after it is
  refreshed. ([#2](https://github.com/q0wkp/zap/pull/2); also proposed upstream as PR 345 in zerx-lab/zap, still
  open when it was archived)
- **Releases and auto-update**: releases are published from this repository, and auto-update, the about page and the
  remote server download point here, so a build of this fork never replaces itself with an upstream one. Tags are the
  next upstream version plus `-mod.X.Y` (upstream's `v2026.07.09.1` becomes `v2026.07.09.2-mod.1.1`): Y counts the
  releases, and X goes up, with Y back at 1, when a release breaks compatibility with the one before it. The releases up
  to `v2026.07.09.2-mod.3`, numbered with one part, were withdrawn when this numbering began. A release is created only
  once every platform has built, for macOS (Apple silicon and Intel) and Linux x86_64; Windows is no longer built. The
  DMGs, the AppImage and the CLI archives carry the release's version, as `zap-<version>-<system>` (for example
  `zap-2026.07.09.2-mod.1.1-linux-x86_64.AppImage`). A Zap of the withdrawn releases (up to `v2026.07.09.2-mod.3`) can't
  update itself to these, whose files it looks for under the earlier names and whose number its package managers read as
  older: install the new one once, with the package manager or from the
  [releases](https://github.com/q0wkp/zap/releases/latest). A build made without a release tag installs the remote
  server of the newest release, which may differ from its own source; a debug build, or any build started with
  `WARP_REMOTE_SERVER_FROM_LOCAL=1`, builds and uploads its own instead.
- **Linux packages**: the release builds the Arch package, which a wrong makepkg option had kept from ever being built,
  and both it and the `.deb` declare the libraries the binary links against (ALSA, liblzma), so they start on systems
  without them, and the oldest glibc the binary runs on, read from it when it is packaged, so that a system too old
  for it refuses the package.
- **Package managers**: `zap` for pacman, `zap-deb` for Pacstall and the cask `zap-terminal` for Homebrew, from
  [q0wkp/pkgs](https://github.com/q0wkp/pkgs), which says how to add them.
- **Updates through package managers**: when Zap was installed with pacman from q0wkp/pkgs' `[q0wkp]`, with Pacstall
  or with the Homebrew cask, updating opens a tab that upgrades Zap alone, with `sudo pacman -U` of the release's
  package from q0wkp/pkgs, `pacstall -I zap-deb` or `brew upgrade --cask --greedy zap-terminal`, and Zap offers to
  restart once the new version is installed. Zap offers an update there only once q0wkp/pkgs has published it for
  that package manager, which can take up to a day after the release. A `.deb` or `.rpm` installed by hand, an AUR
  package or one from another pacman repository shows the command that installs the new one; the AppImage and the DMG
  update themselves.
- **Workflows**: the release workflow is this fork's own, and asks q0wkp/pkgs to package each release once it is out;
  upstream's other workflows, its issue and pull request templates and its Dependabot settings are gone.

## License

The UI framework (the `warpui_core` and `warpui` crates) is licensed under the [MIT license](LICENSE-MIT).

The rest of the code in this repository is licensed under the [AGPL v3](LICENSE-AGPL).
