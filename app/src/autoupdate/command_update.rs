//! Updates installed by running the package manager that installed Zap (pacman, Pacstall, Homebrew)
//! in a terminal tab: checking that q0wkp/pkgs has published the update for that package manager,
//! building the tab's command line, and noticing that the package manager has replaced the running
//! executable with a new version.

use std::collections::HashMap;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context as _, Result};

use super::github;
use crate::terminal::shell::ShellType;

/// The command line the update tab runs: the upgrade, then, only if it succeeded, the shell
/// integration's `warp_finish_update`, which tells Zap through a hook that the upgrade is done.
pub(super) fn tab_command(command: &str, shell_type: ShellType, update_id: &str) -> String {
    let and = shell_type.and_combiner();
    format!("{command}{and}warp_finish_update {update_id}")
}

/// What tells a file apart from another one written over it at the same path: package managers
/// install a new executable as a new file (a new inode), and a rewrite in place changes its size or
/// modification time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileIdentity {
    dev: u64,
    ino: u64,
    size: u64,
    mtime: i64,
    mtime_nsec: i64,
}

impl FileIdentity {
    fn of(path: &Path) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        Ok(Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
            size: metadata.size(),
            mtime: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
        })
    }
}

lazy_static::lazy_static! {
    /// The executable this process was started from, and its identity at startup.
    static ref RUNNING_EXECUTABLE: Option<(PathBuf, FileIdentity)> = std::env::current_exe()
        .and_then(|path| FileIdentity::of(&path).map(|identity| (path, identity)))
        .inspect_err(|err| log::warn!("Could not record the running executable: {err}"))
        .ok();
}

/// Records the running executable's identity. Call it at startup, before a package manager can
/// replace the executable.
pub(super) fn record_running_executable() {
    lazy_static::initialize(&RUNNING_EXECUTABLE);
}

/// Returns `true` if the file this process was started from has since been replaced, i.e. a package
/// manager has installed another version of Zap that a restart would run.
pub(super) fn running_executable_replaced() -> bool {
    RUNNING_EXECUTABLE.as_ref().is_some_and(|(path, identity)| {
        FileIdentity::of(path).is_ok_and(|current| current != *identity)
    })
}

/// The cask that q0wkp/homebrew-tap publishes Zap as.
#[cfg(any(target_os = "macos", test))]
const HOMEBREW_CASK: &str = "zap-terminal";

/// Homebrew's default prefixes, on Apple silicon and on Intel. `brew` isn't looked up on `PATH`,
/// since Zap started from Finder or the Dock doesn't have Homebrew's directories there.
#[cfg(target_os = "macos")]
const HOMEBREW_PREFIXES: [&str; 2] = ["/opt/homebrew", "/usr/local"];

/// Returns the command that upgrades Zap if it was installed as a Homebrew cask.
#[cfg(target_os = "macos")]
pub(super) fn homebrew_upgrade_command() -> Option<String> {
    homebrew_upgrade_command_in(HOMEBREW_PREFIXES.iter().map(Path::new))
}

/// Returns the command that upgrades Zap's cask with the `brew` of the first of `prefixes` that has
/// it installed. Homebrew records an installed cask as the directory `<prefix>/Caskroom/<token>`.
#[cfg(any(target_os = "macos", test))]
fn homebrew_upgrade_command_in<'a>(prefixes: impl IntoIterator<Item = &'a Path>) -> Option<String> {
    prefixes
        .into_iter()
        .map(|prefix| (prefix, prefix.join("bin").join("brew")))
        .find(|(prefix, brew)| {
            prefix.join("Caskroom").join(HOMEBREW_CASK).is_dir() && brew.is_file()
        })
        .map(|(_, brew)| {
            let brew = shell_escape::escape(brew.to_string_lossy());
            // `--greedy` includes casks that declare `auto_updates true`, as Zap's does. Homebrew
            // already upgrades a cask named on the command line that way; the flag says so.
            format!("{brew} upgrade --cask --greedy {HOMEBREW_CASK}")
        })
}

/// The repository that publishes Zap's releases for pacman, Pacstall and Homebrew.
const PKGS_REPO: &str = "q0wkp/pkgs";

/// Where q0wkp/pkgs' files are read from: GitHub, or in tests a server standing in for it.
struct PkgsSource<'a> {
    /// Serves the files on the repository's branches, as raw.githubusercontent.com does.
    raw: &'a str,
    /// Serves GitHub's REST API, as api.github.com does.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    api: &'a str,
}

const GITHUB: PkgsSource<'static> = PkgsSource {
    raw: "https://raw.githubusercontent.com",
    api: "https://api.github.com",
};

/// Zap's pacman package in q0wkp/pkgs.
#[cfg(any(target_os = "linux", test))]
const PKGS_PACMAN_PACKAGE: &str = "zap";

/// The release of q0wkp/pkgs whose assets are its pacman repository, packages and signatures.
#[cfg(any(target_os = "linux", test))]
const PKGS_PACMAN_RELEASE: &str = "x86_64";

/// The package managers that q0wkp/pkgs publishes Zap for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum PkgsTarget {
    #[cfg(target_os = "linux")]
    Pacman,
    #[cfg(target_os = "linux")]
    Pacstall,
    #[cfg(target_os = "macos")]
    Homebrew,
}

impl PkgsTarget {
    /// The target's name in q0wkp/pkgs, whose `versions/zap/<name>` holds the tag of the release it
    /// last published for the target.
    fn name(self) -> &'static str {
        match self {
            #[cfg(target_os = "linux")]
            Self::Pacman => "pacman",
            #[cfg(target_os = "linux")]
            Self::Pacstall => "pacstall",
            #[cfg(target_os = "macos")]
            Self::Homebrew => "homebrew",
        }
    }
}

/// What q0wkp/pkgs has published of a release for one target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PkgsRelease {
    /// For pacman, the URL of the package file, which has its signature next to it. Pacstall and
    /// Homebrew find their package by name.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub(super) pacman_package_url: Option<String>,
}

lazy_static::lazy_static! {
    /// The releases that q0wkp/pkgs was seen to have published, by target and tag, so that the polls
    /// after it don't ask again.
    static ref PKGS_PUBLISHED: Mutex<HashMap<(PkgsTarget, String), PkgsRelease>> =
        Mutex::new(HashMap::new());
}

/// Returns what q0wkp/pkgs has published of Zap's release `tag` for `target`, or `None` while it
/// hasn't yet. q0wkp/pkgs publishes a release minutes to a day after it is out, and until then the
/// package manager would install the version that is already running.
pub(super) async fn pkgs_published(
    client: &http_client::Client,
    target: PkgsTarget,
    tag: &str,
) -> Result<Option<PkgsRelease>> {
    pkgs_published_from(&GITHUB, client, target, tag).await
}

async fn pkgs_published_from(
    source: &PkgsSource<'_>,
    client: &http_client::Client,
    target: PkgsTarget,
    tag: &str,
) -> Result<Option<PkgsRelease>> {
    let key = (target, tag.to_owned());
    if let Some(release) = PKGS_PUBLISHED
        .lock()
        .ok()
        .and_then(|published| published.get(&key).cloned())
    {
        return Ok(Some(release));
    }

    let published_tag = pkgs_published_tag(source, client, target).await?;
    if published_tag.as_deref() != Some(tag) {
        log::info!(
            "{PKGS_REPO} has published {published_tag:?} for {}, not {tag} yet",
            target.name()
        );
        return Ok(None);
    }

    let pacman_package_url = match target {
        #[cfg(target_os = "linux")]
        PkgsTarget::Pacman => {
            let url = format!(
                "{}/repos/{PKGS_REPO}/releases/tags/{PKGS_PACMAN_RELEASE}",
                source.api
            );
            let release = github::fetch_release(client, &url).await?;
            let asset_names = release.assets.iter().map(|asset| asset.name.as_str());
            let Some(file) = find_pacman_package(asset_names, PKGS_PACMAN_PACKAGE, tag) else {
                log::info!("{PKGS_REPO} has no signed pacman package of {tag} yet");
                return Ok(None);
            };
            Some(pacman_package_url(file))
        }
        #[cfg(target_os = "linux")]
        PkgsTarget::Pacstall => None,
        #[cfg(target_os = "macos")]
        PkgsTarget::Homebrew => None,
    };
    let release = PkgsRelease { pacman_package_url };
    if let Ok(mut published) = PKGS_PUBLISHED.lock() {
        published.insert(key, release.clone());
    }
    Ok(Some(release))
}

/// Fetches the tag of the release that q0wkp/pkgs last published for `target`, or `None` if it has
/// never published one.
async fn pkgs_published_tag(
    source: &PkgsSource<'_>,
    client: &http_client::Client,
    target: PkgsTarget,
) -> Result<Option<String>> {
    let url = format!(
        "{}/{PKGS_REPO}/main/versions/zap/{}",
        source.raw,
        target.name()
    );
    let response = client
        .get(&url)
        .header("User-Agent", github::USER_AGENT)
        .timeout(github::FETCH_TIMEOUT)
        .send()
        .await
        .with_context(|| format!("Failed to fetch {url}"))?;
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    let tag = response
        .error_for_status()
        .with_context(|| format!("Failed to fetch {url}"))?
        .text()
        .await
        .with_context(|| format!("Failed to read {url}"))?;
    Ok(Some(tag.trim().to_owned()))
}

/// The pkgver that script/linux/bundle_arch makes of the release tag `tag`, as its
/// `sed -E 's/^v//; s/[-:/[:space:]]+/_/g'` does: without a leading `v`, and with each run of the
/// characters pkgver can't have replaced by `_`.
#[cfg(any(target_os = "linux", test))]
fn arch_pkgver(tag: &str) -> String {
    let is_replaced = |c: char| {
        matches!(
            c,
            '-' | ':' | '/' | ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r'
        )
    };
    let mut pkgver = String::new();
    let mut in_run = false;
    for c in tag.strip_prefix('v').unwrap_or(tag).chars() {
        if is_replaced(c) {
            if !in_run {
                pkgver.push('_');
            }
            in_run = true;
        } else {
            pkgver.push(c);
            in_run = false;
        }
    }
    pkgver
}

/// Returns the file of the x86_64 pacman package `package` of release `tag` among `asset_names`,
/// `<package>-<pkgver>-<pkgrel>-x86_64.pkg.tar.zst`, if its signature `<file>.sig` is there too. If
/// there are several pkgrels, returns the highest.
#[cfg(any(target_os = "linux", test))]
fn find_pacman_package<'a>(
    asset_names: impl IntoIterator<Item = &'a str>,
    package: &str,
    tag: &str,
) -> Option<&'a str> {
    let asset_names: Vec<&str> = asset_names.into_iter().collect();
    let prefix = format!("{package}-{}-", arch_pkgver(tag));
    let pkgrel = |name: &str| -> Option<Vec<u64>> {
        let pkgrel = name
            .strip_prefix(&prefix)?
            .strip_suffix(&format!("-{PKGS_PACMAN_RELEASE}.pkg.tar.zst"))?;
        pkgrel.split('.').map(|part| part.parse().ok()).collect()
    };
    asset_names
        .iter()
        .copied()
        .filter(|name| asset_names.contains(&format!("{name}.sig").as_str()))
        .filter_map(|name| Some((pkgrel(name)?, name)))
        .max()
        .map(|(_, name)| name)
}

/// The URL of the file `file` among q0wkp/pkgs' pacman packages.
#[cfg(any(target_os = "linux", test))]
fn pacman_package_url(file: &str) -> String {
    format!("https://github.com/{PKGS_REPO}/releases/download/{PKGS_PACMAN_RELEASE}/{file}")
}

#[cfg(test)]
#[path = "command_update_test.rs"]
mod tests;
