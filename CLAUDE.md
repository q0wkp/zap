# CLAUDE.md

@AGENTS.md

## This fork

This is q0wkp/zap, Tianyu Lu's own build of Zap, a terminal forked from Warp. The notes below are for maintaining it
and the projects around it. This clone sits at `/home/suroot/0WSB/polyrepo_zap/zap`, next to the others:

| Path | What it is | Remotes |
| --- | --- | --- |
| `.` | Zap, the build the user runs | `q0wkp` = q0wkp/zap (ours), `upstream` = zerx-lab/zap |
| `../warp` | Warp, where Warp's own bugs and features are fixed first | `origin` = lutianyu2001/ZZZ_FORKED.warp, `upstream` = warpdotdev/warp |
| `../pkgs` | q0wkp/pkgs, the pacman and Pacstall repository | `origin` = q0wkp/pkgs |
| `/home/suroot/0WSB/ZZZ_FORKED.zmx` | zmx, the session manager | `origin` = lutianyu2001/ZZZ_FORKED.zmx, `upstream` = neurosnap/zmx |

`../CLAUDE.md` and zmx's `CLAUDE.md` are local files, not in any repository, that import this one so sessions started
there get these notes too. Drafts of issues and PRs for other people's projects go to `/home/suroot/0WSB/0TMP`.

### Remotes and accounts

- zap's `origin`, The-Lu-Group/zap, returns 404, so `git fetch origin` fails there. Fetch `q0wkp` and `upstream`.
- The active gh account (`lutianyu2001`) can't push to q0wkp/zap. Push with
  `GH_TOKEN=$(gh auth token -u q0wkp) git push q0wkp ...` instead of switching accounts.
- zmx and q0wkp/zap use the gh-fork-layout branches (`upstream`, own `main`, feature branches kept after merging).
  ZZZ_FORKED.warp keeps the default layout with Actions off. In the user's words: "我自己不编译warp用, 我直接用zap".

### How the user runs this

The user connects with Zap over SSH into this machine and runs `zmx attach claudeN`. Each zmx session holds an
un-warpified bash running `claude --resume`. Never disturb a live zmx session: test zmx only with a throwaway
`ZMX_DIR`, and only replace the installed zmx once those sessions can end, since its IPC can change between versions.

### The user's decisions

- zap's upstream is unmaintained, and the user maintains q0wkp/zap alone (2026-11-03): "zap官方已经躺尸好久了,
  我也不打算去指望他, 我自己维护着自己的仓库就完了." Don't prepare PRs for zerx-lab/zap. zap changes go into
  q0wkp/zap's main through PRs.
- Changes to a feature Warp provides belong in warp: "你肯定要在warp上改啊, 如果是warp提供的功能". Bugs too:
  "warp 导致的bug优先在 warp 修". Fix them in warp first, then port to zap. A bug zap introduced itself is fixed in zap.
  Decide which project owns a feature or caused a bug by comparing with Warp's current code, not only zap's.
- Where Warp behaves that way by design, zap follows Warp instead of getting its own fix. On Stop handing over control
  before killing the command (2026-11-03): "这个修复可以直接移除, 也不用合并到zap里, 只要zap和warp行为是一致的就行."
- Order for zmx support (2026-11-01): "把 zmx 改了, warp 该写的写了, 再把 warp 的这个改动合并进 zap,
  因为最后我们用的是 zap".
- On structure: only logic that is truly independent goes in new files, "如果不是就不能这么做,
  大不了我们手动再在zap里实现一次". Don't distort Warp's code to make the port to zap easier.
- PRs are written in English, also into the user's own forks: "PR 全都用英语!". "PR 的模板要按 /gen-tlu-pr 来".
  Keep talking to the user in Chinese.

### Contributing to Warp

- In the user's words: "warp要提交的话必须和issue挂钩, 并且issue必须有ready-for-implement, 要求其实是很严的."
  The label's real name is `ready-to-implement`, and only maintainers apply it. On PR #16529 the Oz bot dismissed
  its review until the issue had the label, so file the issue, wait for the label, then open the PR.
- CONTRIBUTING.md asks contributors not to have agents talk to the team. Drafts go to `/home/suroot/0WSB/0TMP` and the
  user posts them. Before/after recordings need a real Warp with an agent, which this headless machine can't run.
- One issue and one PR per bug, branch named `lutianyu2001/<topic>`. A regression test must fail when the fix is
  removed: check it by reverting the fix and running the test.

### Porting from Warp to zap

zap forked from Warp on 2026-04-28 (merge-base c325d146a), so a difference between them is often a later Warp commit.
Find it with `git log -S<symbol> c325d146a..upstream/master` in `../warp`, then fetch Warp into zap
(`git fetch ../warp upstream/master`) and cherry-pick it. Keep the original author, and replace Warp's long PR
body with a short message ending in `(cherry picked from commit <sha>)`; it carries a `Co-Authored-By` line otherwise.
Warp's tests often need small changes in zap: its tests live in `view_test.rs` (Warp: `view_tests.rs`), and some test
APIs and test setup helpers differ.

### Building and testing zap

- zap isn't rustfmt-clean: `cargo fmt -p warp` rewrites over a hundred unrelated files. Format only the files a change
  touches, and revert the rest if it happens.
- `cargo test -p warp --lib` has 79 failures on zap `upstream/main` 8954864e3 (seen 2026-11-03), including the two BYOP
  `smoke_*` tests in `agent_providers` (fixed by branch `fix/byop-smoke-test-setup`). Judge a branch by diffing its
  failing tests against that baseline, not by the exit code.
- Share the target dir between worktrees (`CARGO_TARGET_DIR` set to this clone's `target`, and
  `../warp/target` for Warp), so only the workspace crates rebuild.
- The machine is headless, with no xvfb, nextest, wgslfmt, shellcheck or clang-format.

### zmx

zmx's bats suite fails with `NameTooLong` when `TMPDIR` is a long path (socket path limit), so run it with a short
`TMPDIR`. Build with Zig 0.17.0 and `-Dtarget=x86_64-linux-musl`.

### Packages

zap and zmx each publish to q0wkp/pkgs through `packaging/pkgs/sync.sh`, run from a checkout of their main branch.
`../pkgs/README.md` says how publishing works. Packages are signed with the user's personal GPG key, the one that signs
their commits. The old dedicated key in `~/.config/q0wkp-pkgs/gnupg` was deleted and isn't used.
