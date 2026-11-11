use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use channel_versions::VersionInfo;
use instant::Duration;
use warp_core::channel::{Channel, ChannelState};

use super::command_update::{self, PkgsRelease, PkgsTarget};
use super::release_assets_directory_url;
use super::{github, DownloadProgress, DownloadReady, ProgressCallback, ReadyForRelaunch};

lazy_static::lazy_static! {
    /// Stores the path to the current executable.
    ///
    /// We cache this before running auto-update because the returned path for
    /// a deleted file includes " (deleted)" _in the file name_, which breaks
    /// the relaunch logic.
    static ref CURRENT_EXE: std::io::Result<PathBuf> = std::env::current_exe();
}

pub(super) async fn download_update_and_cleanup(
    version_info: &VersionInfo,
    _update_id: &str,
    client: &http_client::Client,
    on_progress: ProgressCallback,
) -> Result<DownloadReady> {
    match UpdateMethod::detect() {
        UpdateMethod::Unknown => Ok(DownloadReady::NeedsAuthorization),
        UpdateMethod::AppImage(appimage_path) => {
            appimage::download_update_and_cleanup(version_info, &appimage_path, client, on_progress)
                .await
        }
        UpdateMethod::PackageManager(package_manager) => {
            log::info!("Detected that Zap was installed using {package_manager:?}");
            let manual_upgrade = || DownloadReady::ManualUpgrade {
                hint: package_manager.upgrade_hint(),
            };
            let Some(target) = package_manager.pkgs_target() else {
                return Ok(manual_upgrade());
            };
            // Until q0wkp/pkgs has published the release for this package manager, which would
            // otherwise install the running version again, there is no update; the next poll
            // checks again.
            let tag = github::release_tag(&version_info.version);
            let Some(published) = command_update::pkgs_published(client, target, &tag).await?
            else {
                return Ok(DownloadReady::No);
            };
            Ok(match package_manager.upgrade_command(&published) {
                Some(command) => DownloadReady::RunCommand(command),
                None => manual_upgrade(),
            })
        }
    }
}

/// Caches the path of the running executable, which [`relaunch`] starts again. Call it before a
/// package manager can replace the executable, after which the path would end in " (deleted)".
pub(super) fn cache_current_exe() {
    let _ = CURRENT_EXE.as_ref();
}

pub(super) fn apply_update() -> Result<ReadyForRelaunch> {
    // Make sure CURRENT_EXE is initialized before we actually apply the update.
    let _ = CURRENT_EXE.as_ref();

    match UpdateMethod::detect() {
        UpdateMethod::Unknown => bail!("Cannot apply update for unknown update method!"),
        UpdateMethod::AppImage(_) => Ok(ReadyForRelaunch::Yes),
        UpdateMethod::PackageManager(package_manager) => bail!(
            "Zap does not support package-manager autoupdate for {package_manager}; install the new release manually"
        ),
    }
}

pub(super) fn relaunch() -> Result<()> {
    match UpdateMethod::detect() {
        UpdateMethod::Unknown => bail!("Don't know how to relaunch for an unknown update method!"),
        UpdateMethod::AppImage(appimage_path) => appimage::relaunch(&appimage_path),
        UpdateMethod::PackageManager(_) => package_manager::relaunch(),
    }
}

mod appimage {
    use std::path::Path;

    use super::*;

    pub(super) async fn download_update_and_cleanup(
        version_info: &VersionInfo,
        appimage_path: &Path,
        client: &http_client::Client,
        on_progress: ProgressCallback,
    ) -> Result<DownloadReady> {
        use futures::StreamExt as _;
        use instant::Instant;
        const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

        let channel = ChannelState::channel();
        // The AppImage of the release being installed, as the release workflow names it.
        let asset = github::appimage_name(&version_info.version);
        // openWarp:从 GitHub Release 缓存里取真实下载 URL,绕开空的 releases_base_url。
        // 官方 channel 仍然走 release_assets_directory_url。
        let url = if matches!(channel, warp_core::channel::Channel::Oss) {
            if let Some(release) = crate::autoupdate::github::cached_release() {
                if let Some(found) = release.find_asset(&asset) {
                    found.browser_download_url.clone()
                } else {
                    log::warn!(
                        "openWarp: cached release tag {} 没有名为 {asset} 的资产,回退到 tag URL",
                        release.tag_name
                    );
                    format!(
                        "https://github.com/q0wkp/zap/releases/download/v{}/{asset}",
                        version_info.version
                    )
                }
            } else {
                format!(
                    "https://github.com/q0wkp/zap/releases/download/v{}/{asset}",
                    version_info.version
                )
            }
        } else {
            let Some(appimage_name) = option_env!("APPIMAGE_NAME") else {
                bail!("APPIMAGE_NAME environment variable was not set at compile time!");
            };
            format!(
                "{}/{}",
                release_assets_directory_url(channel, &version_info.version),
                appimage_name
            )
        };

        // Create a temporary file that we'll write the download into.
        let mut new_appimage = tempfile::NamedTempFile::new()?;

        log::info!("Downloading {url} to {}...", new_appimage.path().display());

        let response = client
            .get(&url)
            .timeout(DOWNLOAD_TIMEOUT)
            .send()
            .await?
            .error_for_status()?;

        // 流式读 chunk + 写入,过程中节流上报进度。AppImage 体积大(数十 MB),
        // 一次 `.bytes()` 会卡住整个 UI 直到下载完;改成 stream 让 UI 看到进度。
        let total = response
            .headers()
            .get(http::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());
        on_progress(DownloadProgress {
            downloaded: 0,
            total,
        });
        let mut downloaded: u64 = 0;
        let mut last_reported = 0u64;
        let mut last_reported_at = Instant::now();
        const REPORT_BYTES_THRESHOLD: u64 = 64 * 1024;
        const REPORT_TIME_THRESHOLD: Duration = Duration::from_millis(250);

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            new_appimage.as_file_mut().write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            if downloaded - last_reported >= REPORT_BYTES_THRESHOLD
                || last_reported_at.elapsed() >= REPORT_TIME_THRESHOLD
            {
                on_progress(DownloadProgress {
                    downloaded,
                    total,
                });
                last_reported = downloaded;
                last_reported_at = Instant::now();
            }
        }
        on_progress(DownloadProgress {
            downloaded,
            total,
        });

        // openWarp:在覆盖原 AppImage 之前先对临时文件做 SHA-256 校验,
        // 防御 CDN 中间人 / 网络损坏。其他 channel 跳过(有自家流程)。
        if matches!(channel, warp_core::channel::Channel::Oss) {
            let temp_path = new_appimage.path().to_path_buf();
            if let Err(e) = crate::autoupdate::verify_oss_asset_sha256(&temp_path, &asset) {
                // 临时文件会随 NamedTempFile drop 自动清理,这里只需返回错误。
                return Err(e);
            }
        }

        log::info!(
            "Copying downloaded AppImage from {} to {}",
            new_appimage.path().display(),
            appimage_path.display()
        );

        // Copy permissions to new app before moving it to ensure we don't leave it
        // in a bad state if the move succeeds but we are unable to update the
        // permissions afterwards.
        new_appimage
            .as_file_mut()
            .set_permissions(appimage_path.metadata()?.permissions())?;

        // Move new AppImage over the one that launched the current Zap instance.
        let new_appimage_path = new_appimage.into_temp_path();
        let mv_status = command::r#async::Command::new("mv")
            .arg(new_appimage_path.as_os_str())
            .arg(appimage_path)
            .output()
            .await?
            .status;
        if !mv_status.success() {
            bail!("Failed to move new AppImage over the old one: {mv_status}");
        }

        // Ensure we don't accidentally drop `new_appimage_path` before we finish
        // moving it to its final location.
        let _ = new_appimage_path;

        Ok(DownloadReady::Yes)
    }

    pub(super) fn relaunch(appimage_path: &Path) -> Result<()> {
        let mut command = command::blocking::Command::new(appimage_path);
        // Pass a flag to the app to let it know it was restarted as part of the
        // autoupdate process.
        command.arg(warp_cli::finish_update_flag());
        // 测试本地通道版本 JSON 时，让新启动的二进制继续引用同一个文件，
        // 以便验证自动更新后的 changelog 展示。
        if let Ok(path) = std::env::var("WARP_CHANNEL_VERSIONS_PATH") {
            command.env("WARP_CHANNEL_VERSIONS_PATH", path);
        }

        log::info!("Relaunching warp for update...");
        command.spawn()?;
        Ok(())
    }
}

mod package_manager {
    use super::*;

    pub(super) fn relaunch() -> Result<()> {
        let Ok(program) = CURRENT_EXE.as_ref() else {
            bail!(
                "Failed to get path to current executable to relaunch after completing auto-update"
            );
        };
        log::info!("Relaunching using path: {program:?}");
        let mut command = command::blocking::Command::new(program);
        // Add any arguments that were passed to warp, skipping the first
        // argument (the name of the executable) and dropping the flag for
        // finishing an update.
        let finish_update_flag = warp_cli::finish_update_flag();
        command.args(
            std::env::args()
                .skip(1)
                .filter(|arg| arg != &finish_update_flag),
        );
        // Pass a flag to the app to let it know it was restarted as part of the
        // autoupdate process.
        command.arg(finish_update_flag);
        // 测试本地通道版本 JSON 时，让新启动的二进制继续引用同一个文件，
        // 以便验证自动更新后的 changelog 展示。
        if let Ok(path) = std::env::var("WARP_CHANNEL_VERSIONS_PATH") {
            command.env("WARP_CHANNEL_VERSIONS_PATH", path);
        }

        log::info!("Relaunching warp for update...");
        command.spawn()?;
        Ok(())
    }
}

/// Returns which method should be used to update Zap.
#[derive(Debug)]
pub(crate) enum UpdateMethod {
    /// We don't know how to update Zap.
    Unknown,
    /// Zap is running as an AppImage and should be updated in-place.
    AppImage(PathBuf),
    /// Zap can be updated using the given package manager.
    PackageManager(PackageManager),
}

impl UpdateMethod {
    pub(crate) fn detect() -> Self {
        if let Some(appimage_path) = std::env::var_os("APPIMAGE").map(PathBuf::from) {
            return Self::AppImage(appimage_path);
        }
        if let Ok(package_manager) = PackageManager::detect() {
            // 记录用户应当跑的升级命令,方便从日志查问题。UI 仍然兜底跳 GitHub
            // release 页(用户可以下 .deb/.rpm 自行 apt install / dnf install)。
            package_manager.log_upgrade_hint();
            return Self::PackageManager(package_manager);
        }
        Self::Unknown
    }
}

/// Package managers that we understand and can assist with auto-update for. A pacman package is
/// `PacmanQ0wkp` when q0wkp/pkgs' repository `[q0wkp]` has it, which Zap upgrades alone with
/// `pacman -U`; `PacmanRepo` when another sync repository has it, upgraded with `pacman -Syu`; and
/// `PacmanAur` when no sync repository has it, as for the AUR or a package built by hand, upgraded
/// with an AUR helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageManager {
    Apt {
        package_name: String,
    },
    Yum {
        package_name: String,
    },
    Dnf {
        package_name: String,
    },
    Zypper {
        package_name: String,
    },
    /// The deb was installed by Pacstall (from q0wkp/pkgs, as `zap-deb`), which upgrades it.
    Pacstall {
        /// The Pacstall package, whose pacscript `gives` the dpkg package `package_name`.
        pacstall_name: String,
        package_name: String,
    },
    /// A pacman package that q0wkp/pkgs' repository `[q0wkp]` has (`pacman -Si q0wkp/<pkg>`).
    PacmanQ0wkp {
        package_name: String,
    },
    /// A pacman package that another sync repository has (`pacman -Si <pkg>`).
    PacmanRepo {
        package_name: String,
    },
    /// AUR / 本地手工安装(`pacman -Qi <pkg>` 命中但 `pacman -Si <pkg>` 不命中)。
    PacmanAur {
        package_name: String,
    },
}

impl PackageManager {
    /// 当前 channel 下要在系统包管理器里查询的候选包名,按可能性从高到低排序。
    /// OSS 在 deb/rpm/arch bundle 脚本里包名都是 `zap`(见 script/linux/bundle_*),
    /// 但 AUR 上常见命名是 `zap-bin` / `zap-git`,所以多试几个。
    fn candidate_names(channel: Channel) -> &'static [&'static str] {
        match channel {
            Channel::Stable => &["warp-terminal"],
            Channel::Preview => &["warp-terminal-preview"],
            Channel::Dev => &["warp-terminal-dev"],
            Channel::Integration => &["warp-terminal-integration"],
            Channel::Local => &["warp-terminal-local"],
            // OSS:bundle_deb/rpm/arch 全部用 `zap` 作 package name,但 AUR
            // 维护者可能选 `zap-bin` / `zap-git`,所以也试一下。
            Channel::Oss => &["zap", "zap-bin", "zap-git"],
        }
    }

    fn detect() -> Result<Self> {
        let channel = ChannelState::channel();
        let candidates = Self::candidate_names(channel);

        // 依次试每个候选包名;第一个被任意 PM 识别为已安装的就返回。
        // pacman 命中后再用 `pacman -Si` 区分官方仓库 / AUR。
        for &name in candidates {
            if let Some(pm) = Self::probe_one(name)? {
                return Ok(pm);
            }
        }
        bail!(
            "Could not determine which package manager was used to install \
             this build (tried candidate names: {candidates:?})"
        );
    }

    /// 对一个具体的包名跑探测脚本;命中则返回对应的 PackageManager,未命中返回 None。
    /// pacman 命中后额外查 `pacman -Si` 来区分官方仓库和 AUR。
    fn probe_one(package_name: &str) -> Result<Option<Self>> {
        // shell 脚本里 `$PACKAGE_NAME` 由 env 传入,内容不会被 shell 转义注入
        // (传到 command 而非 sh -c 字符串拼接)。
        let detect_script = r#"
            command -p pacman -Qi "$PACKAGE_NAME" >/dev/null 2>/dev/null
            if [ $? -eq 0 ]; then
              # 区分官方仓库 vs AUR/手工。-Si 查 sync database,AUR/手工
              # 安装的包不会被 sync 出来。
              if command -p pacman -Si "q0wkp/$PACKAGE_NAME" >/dev/null 2>/dev/null; then
                echo "pacman-q0wkp"
              elif command -p pacman -Si "$PACKAGE_NAME" >/dev/null 2>/dev/null; then
                echo "pacman-official"
              else
                echo "pacman-aur"
              fi
              exit
            fi

            command -p zypper search --match-exact --installed-only "$PACKAGE_NAME" >/dev/null 2>/dev/null
            if [ $? -eq 0 ]; then
              echo "zypper"
              exit
            fi

            command -p dnf list --installed "$PACKAGE_NAME" >/dev/null 2>/dev/null
            if [ $? -eq 0 ]; then
              echo "dnf"
              exit
            fi

            command -p yum list installed "$PACKAGE_NAME" >/dev/null 2>/dev/null
            if [ $? -eq 0 ]; then
              echo "yum"
              exit
            fi

            if [ "$(command -p dpkg-query --show --showformat='${db:Status-Status}' "$PACKAGE_NAME" 2>/dev/null)" = "installed" ]; then
              echo "apt"
              exit
            fi

            exit 1
        "#;

        let output = command::blocking::Command::new("sh")
            .args(["-c", detect_script])
            .env("PACKAGE_NAME", package_name)
            .output();
        let output = match output {
            Ok(o) => o,
            Err(err) => {
                return Err(err).context("Failed to run package manager detection script")
            }
        };

        // exit 1 = 这个候选名没被任何 PM 识别;不是错,继续下一个候选。
        if !output.status.success() {
            return Ok(None);
        }
        let stdout = std::str::from_utf8(&output.stdout)
            .map_err(|_| anyhow::anyhow!("non-UTF-8 detect script output"))?;
        let name = package_name.to_string();
        let pm = match stdout.trim() {
            "pacman-q0wkp" => Self::PacmanQ0wkp { package_name: name },
            "pacman-official" => Self::PacmanRepo { package_name: name },
            "pacman-aur" => Self::PacmanAur { package_name: name },
            "zypper" => Self::Zypper { package_name: name },
            "dnf" => Self::Dnf { package_name: name },
            "yum" => Self::Yum { package_name: name },
            "apt" => match pacstall_package_for(&name, Path::new(PACSTALL_METADATA_DIR)) {
                Some(pacstall_name) => Self::Pacstall {
                    pacstall_name,
                    package_name: name,
                },
                None => Self::Apt { package_name: name },
            },
            other => bail!("Unexpected detection output: {other}"),
        };
        Ok(Some(pm))
    }

    /// The target that q0wkp/pkgs publishes Zap's releases to for this package manager, when Zap
    /// can run the package manager to upgrade itself.
    pub(super) fn pkgs_target(&self) -> Option<PkgsTarget> {
        match self {
            Self::PacmanQ0wkp { .. } => Some(PkgsTarget::Pacman),
            Self::Pacstall { .. } => Some(PkgsTarget::Pacstall),
            Self::Apt { .. }
            | Self::Yum { .. }
            | Self::Dnf { .. }
            | Self::Zypper { .. }
            | Self::PacmanRepo { .. }
            | Self::PacmanAur { .. } => None,
        }
    }

    /// The command that upgrades Zap alone to the release that q0wkp/pkgs has `published`, which
    /// Zap runs for the user in a terminal tab. The others need a package downloaded by hand, a
    /// full upgrade or an AUR helper, see [`Self::upgrade_hint`].
    pub(super) fn upgrade_command(&self, published: &PkgsRelease) -> Option<String> {
        match self {
            // The package file of the release, which pacman checks against its signature next to
            // it, with the keys trusted for [q0wkp]. Unlike `pacman -Sy zap`, a partial upgrade
            // that Arch doesn't support, it leaves the sync databases alone; the package's
            // `glibc>=` dependency keeps it off a system too old to run it.
            Self::PacmanQ0wkp { .. } => published
                .pacman_package_url
                .as_deref()
                .map(|url| format!("sudo pacman -U {}", shell_escape::escape(url.into()))),
            // `pacstall -Up` upgrades every Pacstall package and takes no package names; `-I` of an
            // installed package installs its pacscript from the repository over it, whatever the
            // version, which is why the update waits for q0wkp/pkgs.
            Self::Pacstall { pacstall_name, .. } => Some(format!(
                "pacstall -I {}",
                shell_escape::escape(pacstall_name.into())
            )),
            Self::Apt { .. }
            | Self::Yum { .. }
            | Self::Dnf { .. }
            | Self::Zypper { .. }
            | Self::PacmanRepo { .. }
            | Self::PacmanAur { .. } => None,
        }
    }

    /// What the user runs to upgrade Zap, shown when Zap can't run it itself.
    pub(crate) fn upgrade_hint(&self) -> String {
        match self {
            Self::PacmanQ0wkp { .. } | Self::PacmanRepo { .. } => "sudo pacman -Syu".to_owned(),
            Self::Pacstall { pacstall_name, .. } => {
                format!("pacstall -I {}", shell_escape::escape(pacstall_name.into()))
            }
            Self::Apt { package_name } => format!("sudo apt install ./{package_name}_*.deb"),
            Self::Yum { package_name } => format!("sudo yum install ./{package_name}-*.rpm"),
            Self::Dnf { package_name } => format!("sudo dnf install ./{package_name}-*.rpm"),
            Self::Zypper { package_name } => {
                format!("sudo zypper install ./{package_name}-*.rpm")
            }
            Self::PacmanAur { package_name } => {
                format!("paru -Syu {package_name} / yay -Syu {package_name}")
            }
        }
    }

    /// 把"用户应该跑的升级命令"写到日志里,方便从日志查问题。
    fn log_upgrade_hint(&self) {
        let hint = self.upgrade_hint();
        log::info!("openWarp upgrade hint ({self}): {hint}");
    }
}

/// Where Pacstall keeps a file of metadata for each package it installed (`METADIR` in
/// `/usr/bin/pacstall`).
const PACSTALL_METADATA_DIR: &str = "/var/lib/pacstall/metadata";

/// Returns the Pacstall package that installed the dpkg package `package_name`, if Pacstall did.
fn pacstall_package_for(package_name: &str, metadata_dir: &Path) -> Option<String> {
    std::fs::read_dir(metadata_dir)
        .ok()?
        .flatten()
        .find_map(|entry| {
            let contents = std::fs::read_to_string(entry.path()).ok()?;
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let (pacstall_name, dpkg_name) = parse_pacstall_metadata(&contents, file_name);
            (dpkg_name == package_name).then_some(pacstall_name)
        })
}

/// Reads a Pacstall metadata file, whose lines are shell assignments such as `_name="zap-deb"` and
/// `_gives="zap"`, and returns the Pacstall package's name and the dpkg package it installed. Like
/// Pacstall itself, takes the dpkg package to be `${_gives:-$_name}`; `file_name`, which Pacstall
/// names after the package, stands in for a missing `_name`.
fn parse_pacstall_metadata(contents: &str, file_name: String) -> (String, String) {
    let value = |key: &str| {
        contents.lines().find_map(|line| {
            let value = line.trim().strip_prefix(key)?.strip_prefix('=')?;
            let value = value.trim_matches(|c| c == '"' || c == '\'');
            (!value.is_empty()).then(|| value.to_owned())
        })
    };
    let name = value("_name").unwrap_or(file_name);
    let gives = value("_gives").unwrap_or_else(|| name.clone());
    (name, gives)
}

impl std::fmt::Display for PackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageManager::Apt { .. } => write!(f, "apt"),
            PackageManager::Yum { .. } => write!(f, "yum"),
            PackageManager::Dnf { .. } => write!(f, "dnf"),
            PackageManager::Zypper { .. } => write!(f, "zypper"),
            PackageManager::Pacstall { .. } => write!(f, "Pacstall"),
            PackageManager::PacmanQ0wkp { .. } => write!(f, "pacman ([q0wkp])"),
            PackageManager::PacmanRepo { .. } => write!(f, "pacman"),
            PackageManager::PacmanAur { .. } => write!(f, "pacman (AUR)"),
        }
    }
}

#[cfg(test)]
#[path = "linux_test.rs"]
mod tests;
