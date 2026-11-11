# CLAUDE.md

@AGENTS.md

## This fork

This is q0wkp/zap, Tianyu Lu's own build of Zap, a terminal forked from Warp. The notes below are for maintaining it.

### Remotes and branches

- `origin`, The-Lu-Group/zap, returns 404, so `git fetch origin` fails. Fetch `q0wkp` (ours) and `upstream`
  (zerx-lab/zap).
- The active gh account (`lutianyu2001`) can't push to q0wkp/zap. Push with
  `GH_TOKEN=$(gh auth token -u q0wkp) git push q0wkp ...` instead of switching accounts.
- Branches follow gh-fork-layout (`upstream` mirrors zerx-lab/zap, `main` is ours, feature branches are kept after
  merging), except that fork-only changes go straight to main (see below).

### The user's decisions

- zap's upstream is unmaintained, and the user maintains q0wkp/zap alone (2026-11-03): "zap官方已经躺尸好久了,
  我也不打算去指望他, 我自己维护着自己的仓库就完了." But in case it revives: "我还是希望某一天上游复活了还能接着向上游
  贡献的, 所以还是能准备的接着准备." So:
  - A change upstream could take: branch from `upstream` and PR it into our main as before ("能往上游同步的还是照常开
    pr"), keeping the branch so it can still be PRed to zerx-lab/zap.
  - A change only for this fork: commit it straight to main, no branch or PR ("不往上游同步的我们自己的修改都可以直接
    往main开"). This file is one of those.
- Changes to a feature Warp provides belong in Warp: "你肯定要在warp上改啊, 如果是warp提供的功能". Bugs too:
  "warp 导致的bug优先在 warp 修". Fix them in Warp first (the clone next to this one, `../warp`), then port them here. A
  bug zap introduced itself is fixed in zap. Decide which one owns a feature or caused a bug by comparing with Warp's
  current code, not only zap's.
- Where Warp behaves that way by design, zap follows Warp instead of getting its own fix. On Stop handing over control
  before killing the command (2026-11-03): "这个修复可以直接移除, 也不用合并到zap里, 只要zap和warp行为是一致的就行."
- Order for zmx support (2026-11-01): "把 zmx 改了, warp 该写的写了, 再把 warp 的这个改动合并进 zap,
  因为最后我们用的是 zap".
- On structure: only logic that is truly independent goes in new files, "如果不是就不能这么做,
  大不了我们手动再在zap里实现一次". Don't distort Warp's code to make the port to zap easier.
- This fork supports only Linux and macOS (decided 2026-11-08); all Windows code, assets and scripts were removed.
  When porting from Warp, leave out its Windows-only parts (`cfg(windows)` code, ConPTY, WSL/MSYS2 shells).
- PRs are written in English, also into the user's own forks: "PR 全都用英语!". "PR 的模板要按 /gen-tlu-pr 来".
  Keep talking to the user in Chinese.
- Updating through a package manager upgrades Zap alone (decided 2026-11-09): pacman, for a Zap from `[q0wkp]`, with
  `sudo pacman -U https://github.com/q0wkp/pkgs/releases/download/x86_64/<file>` (not `-Syu`, and never `-Sy zap`, a
  partial upgrade Arch doesn't support), Pacstall with `pacstall -I zap-deb` (not `-Up`). Zap offers an update to a
  pacman, Pacstall or Homebrew install only once q0wkp/pkgs has published it there: its `versions/zap/<target>` on
  main holds the release's tag, and for pacman the release `x86_64` has the package and its `.sig`.
- The Arch and deb packages declare the oldest glibc they run on (decided 2026-11-09): `glibc>=X.Y` and
  `libc6 (>= X.Y)`, where X.Y is the highest `GLIBC_x.y` the packaged ELF files need, computed at bundle time by
  `script/linux/min_glibc_version`, never hard-coded. A system too old for the build then refuses the package instead
  of installing a Zap that can't start. The rpm needs nothing: rpmbuild adds versioned glibc requirements itself.
- The files a release builds itself follow the user's naming rule with no exception (decided 2026-11-10):
  `zap-<version>-<system>`, the version without the tag's `v`, for the DMGs, the AppImage and the CLI archives
  (`zap-2026.07.09.2-mod.4-macos-aarch64.dmg` for the tag `v2026.07.09.2-mod.4`). The `.deb`, `.rpm` and
  `.pkg.tar.zst` keep the names their package managers give them. The earlier names (`Zap-arm64.dmg`,
  `Zap-x86_64.AppImage`, `zap-linux-x86_64.tar.gz`) are not published beside the new ones.

### Porting from Warp

zap forked from Warp on 2026-04-28 (merge-base c325d146a), so a difference between them is often a later Warp commit.
Find it with `git log -S<symbol> c325d146a..upstream/master` in `../warp`, then `git fetch ../warp upstream/master`
here and cherry-pick it. Keep the original author, and replace Warp's long PR body with a short message ending in
`(cherry picked from commit <sha>)`; it carries a `Co-Authored-By` line otherwise. Warp's tests often need small
changes here: zap's tests live in `view_test.rs` (Warp: `view_tests.rs`), and some test APIs and setup helpers differ.

### Building and testing

- zap isn't rustfmt-clean: `cargo fmt -p warp` rewrites over a hundred unrelated files. Format only the files a change
  touches, and revert the rest if it happens.
- `rustfmt <file>` also formats every module the file declares (`rustfmt app/src/workspace/view.rs` rewrites
  `workspace/view/*`), and the repository toolchain's rustfmt (1.8.0, Rust 1.92.0) has no `skip_children`. Format
  one file alone by piping it: `rustfmt --config-path .rustfmt.toml --emit stdout < <file>`, then take only the
  hunks in your lines.
- `cargo test -p warp --lib` has 79 failures on `upstream/main` 8954864e3 (seen 2026-11-03), including the two BYOP
  `smoke_*` tests in `agent_providers` (fixed by branch `fix/byop-smoke-test-setup`). Judge a branch by diffing its
  failing tests against that baseline, not by the exit code.
- The `warp` crate's unit tests can't be built in 8 GB of memory (rustc 1.92.0, seen 2026-11-10): under
  `systemd-run -p MemoryMax=8G`, rustc passed 7.8 GB after 100 seconds of `cargo test -p warp --lib` and was killed,
  with or without `CARGO_INCREMENTAL=0`. Within that limit, `CARGO_INCREMENTAL=0 cargo check -p warp --lib --profile
  test` type-checks the library with its tests (5 GB at its peak); `cargo check -p warp --lib --tests` was killed
  as well.
- Share one `CARGO_TARGET_DIR` (this clone's `target`) between worktrees, so only the workspace crates rebuild.
  But cargo 1.92.0 names workspace-crate artifacts by their path relative to the workspace, so checkouts sharing it
  overwrite each other's, and a checkout whose files are older reuses the other's as fresh: after building a branch
  that removed items from `warp_util`, `cargo test` in the main clone linked that branch's `warp_util` and failed in
  `app/build.rs`. Build a baseline of such a branch in a separate target dir.
- The build machine is headless, with no xvfb, nextest, wgslfmt or clang-format, so UI changes can't be checked
  there. shellcheck 0.9.0 is installed (seen 2026-11-09).

### Packages

zap's recipe lives in q0wkp/pkgs (`recipes/zap/`), whose Sync workflow publishes each release, asked by the release
workflow's `pkgs` job (Sync has no schedule; a release that job didn't reach waits for a run by hand): `zap` for
pacman, `zap-deb` for Pacstall, and the cask `zap-terminal` in q0wkp/homebrew-tap. Each goes out once the release has
its own files (the `.pkg.tar.zst`, the `.deb`, the two DMGs), so a release missing one still reaches the others. The
pkgs README says how publishing works.

A release's files are looked up by name in three places besides the release workflow that names them: Zap's updater
(the DMGs, the AppImage), its remote-server installer (the CLI archives) and the recipe's `ASSETS` in
`recipes/zap/package.conf` (the `.deb`, the `.pkg.tar.zst`, the DMGs). After a name changes in the workflow alone,
the updater and the installer fail to download the file, and pkgs skips the release without an error.

linuxdeploy (1-alpha-20240109-1) names the AppImage it writes itself, `Zap-x86_64.AppImage`, after the desktop
file's `Name` and the architecture; `APPIMAGE_NAME` in `script/linux/bundle` only repeats that name, so the release
workflow renames the file when it collects the packages.

Pacstall 6.4.2 (read in its source): `pacstall -Up` takes no package names ("Invalid argument") and upgrades every
Pacstall package; `pacstall -I <pkg>` of an installed package downloads its pacscript from the repository again and
installs it over the old one, without comparing versions, so it reinstalls the same version when the repository has
nothing newer.
