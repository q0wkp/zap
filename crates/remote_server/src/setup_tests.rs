// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 The Zap Authors (https://github.com/zerx-lab/zap)
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: tested the signed SHA-256 check and the failed-install detection of the remote-server install, and moved the tests to the new release tags and names; renamed the Warp and Zap names to ZapNG.

use super::*;

#[test]
fn parse_uname_linux_x86_64() {
    let platform = parse_uname_output("Linux x86_64").unwrap();
    assert_eq!(platform.os, RemoteOs::Linux);
    assert_eq!(platform.arch, RemoteArch::X86_64);
}

#[test]
fn parse_uname_linux_aarch64() {
    let platform = parse_uname_output("Linux aarch64").unwrap();
    assert_eq!(platform.os, RemoteOs::Linux);
    assert_eq!(platform.arch, RemoteArch::Aarch64);
}

#[test]
fn parse_uname_darwin_arm64() {
    let platform = parse_uname_output("Darwin arm64").unwrap();
    assert_eq!(platform.os, RemoteOs::MacOs);
    assert_eq!(platform.arch, RemoteArch::Aarch64);
}

#[test]
fn parse_uname_darwin_x86_64() {
    let platform = parse_uname_output("Darwin x86_64").unwrap();
    assert_eq!(platform.os, RemoteOs::MacOs);
    assert_eq!(platform.arch, RemoteArch::X86_64);
}

#[test]
fn parse_uname_linux_armv8l() {
    let platform = parse_uname_output("Linux armv8l").unwrap();
    assert_eq!(platform.os, RemoteOs::Linux);
    assert_eq!(platform.arch, RemoteArch::Aarch64);
}

#[test]
fn parse_uname_skips_shell_initialization_output() {
    let output = "Last login: Mon Apr  7 10:00:00 2025\nWelcome to Ubuntu\nLinux x86_64";
    let platform = parse_uname_output(output).unwrap();
    assert_eq!(platform.os, RemoteOs::Linux);
    assert_eq!(platform.arch, RemoteArch::X86_64);
}

#[test]
fn parse_uname_trims_whitespace() {
    let platform = parse_uname_output("  Linux x86_64  \n").unwrap();
    assert_eq!(platform.os, RemoteOs::Linux);
    assert_eq!(platform.arch, RemoteArch::X86_64);
}

#[test]
fn parse_uname_unsupported_os() {
    let result = parse_uname_output("Windows x86_64");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unsupported OS"));
}

#[test]
fn parse_uname_unsupported_arch() {
    let result = parse_uname_output("Linux mips");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unsupported arch"));
}

#[test]
fn parse_uname_empty_output() {
    let result = parse_uname_output("");
    assert!(result.is_err());
}

#[test]
fn parse_uname_missing_arch() {
    let result = parse_uname_output("Linux");
    assert!(result.is_err());
}

#[test]
fn state_is_ready() {
    assert!(RemoteServerSetupState::Ready.is_ready());
    assert!(!RemoteServerSetupState::Checking.is_ready());
    assert!(!RemoteServerSetupState::Initializing.is_ready());
}

#[test]
fn state_is_failed() {
    assert!(RemoteServerSetupState::Failed {
        error: "test".into()
    }
    .is_failed());
    assert!(!RemoteServerSetupState::Ready.is_failed());
}

#[test]
fn state_is_terminal() {
    assert!(RemoteServerSetupState::Ready.is_terminal());
    assert!(RemoteServerSetupState::Failed {
        error: "test".into()
    }
    .is_terminal());
    assert!(RemoteServerSetupState::Unsupported {
        reason: UnsupportedReason::NonGlibc {
            name: "musl".into()
        }
    }
    .is_terminal());
    assert!(!RemoteServerSetupState::Checking.is_terminal());
    assert!(!RemoteServerSetupState::Installing {
        progress_percent: None,
    }
    .is_terminal());
    assert!(!RemoteServerSetupState::Updating.is_terminal());
    assert!(!RemoteServerSetupState::Initializing.is_terminal());
}

#[test]
fn parse_preinstall_supported_glibc() {
    let stdout = "required_glibc=2.31\n\
                  libc_family=glibc\n\
                  libc_version=2.35\n\
                  status=supported\n";
    let result = PreinstallCheckResult::parse(stdout);
    assert_eq!(result.status, PreinstallStatus::Supported);
    assert_eq!(result.libc, RemoteLibc::Glibc(GlibcVersion::new(2, 35)));
    assert!(result.is_supported());
}

#[test]
fn parse_preinstall_legacy_glibc_too_old_now_supported() {
    // remote-server 现在是静态 musl 二进制,旧脚本的 `glibc_too_old` 门禁
    // 已失效。即使老 remote 端缓存着旧脚本并报出该理由,客户端也应把它当
    // 作支持处理,而不是回退到 ControlMaster。
    let stdout = "required_glibc=2.17\n\
                  libc_family=glibc\n\
                  libc_version=2.17\n\
                  status=unsupported\n\
                  reason=glibc_too_old\n";
    let result = PreinstallCheckResult::parse(stdout);
    assert_eq!(result.status, PreinstallStatus::Supported);
    assert!(result.is_supported());
}

#[test]
fn parse_preinstall_legacy_non_glibc_now_supported() {
    // 同理:musl/uclibc 宿主在静态二进制下也能运行。旧脚本的 `non_glibc`
    // 不再触发 fall-back。
    let stdout = "required_glibc=2.17\n\
                  libc_family=musl\n\
                  status=unsupported\n\
                  reason=non_glibc\n";
    let result = PreinstallCheckResult::parse(stdout);
    assert_eq!(result.status, PreinstallStatus::Supported);
    assert_eq!(
        result.libc,
        RemoteLibc::NonGlibc {
            name: "musl".to_string()
        }
    );
    assert!(result.is_supported());
}

#[test]
fn parse_preinstall_musl_host_supported() {
    // 新版脚本在 musl 宿主上直接报 `status=supported`。
    let stdout = "required_glibc=2.17\n\
                  libc_family=musl\n\
                  status=supported\n";
    let result = PreinstallCheckResult::parse(stdout);
    assert_eq!(result.status, PreinstallStatus::Supported);
    assert!(result.is_supported());
}

#[test]
fn parse_preinstall_old_glibc_host_supported() {
    // 新版脚本在旧 glibc(< 2.35)宿主上也直接报 `status=supported`。
    let stdout = "required_glibc=2.17\n\
                  libc_family=glibc\n\
                  libc_version=2.17\n\
                  status=supported\n";
    let result = PreinstallCheckResult::parse(stdout);
    assert_eq!(result.status, PreinstallStatus::Supported);
    assert!(result.is_supported());
}

#[test]
fn parse_preinstall_missing_status_falls_open() {
    // Garbled / partial script output — missing status field. Confirms
    // the fail-open invariant: anything we can't positively classify as
    // unsupported degrades to Unknown and is treated as supported, so a
    // flaky probe doesn't block the install.
    let stdout = "libc_family=glibc\nlibc_version=2.35\n";
    let result = PreinstallCheckResult::parse(stdout);
    assert_eq!(result.status, PreinstallStatus::Unknown);
    assert!(result.is_supported());
}

#[test]
fn oss_remote_server_dir_uses_zap_ng_namespace() {
    assert_eq!(remote_server_dir(), "~/.zap-ng/remote-server");
}

#[test]
fn oss_binary_name_matches_zap_ng_cli() {
    assert_eq!(binary_name(), "zap-ng");
}

/// A release tag, and the version its files carry.
const RELEASE_TAG: &str = "v2026.07.09.2-mod.1.1";
const RELEASE_VERSION: &str = "2026.07.09.2-mod.1.1";

#[test]
fn release_tarball_url_names_the_file_by_version() {
    let url = release_tarball_url(RELEASE_TAG, "linux", "x86_64");

    assert_eq!(
        url,
        "https://github.com/q0wkp/zap-ng/releases/download/v2026.07.09.2-mod.1.1/zap-ng-2026.07.09.2-mod.1.1-linux-x86_64.tar.gz"
    );
    assert!(!url.contains("app.warp.dev"));
    assert!(!url.contains("/download/cli"));
}

#[test]
fn download_tarball_url_names_the_platform() {
    let platform = RemotePlatform {
        os: RemoteOs::MacOs,
        arch: RemoteArch::Aarch64,
    };

    assert_eq!(
        download_tarball_url(RELEASE_TAG, &platform),
        release_tarball_url(RELEASE_TAG, "macos", "aarch64")
    );
}

#[test]
fn release_tag_in_url_reads_the_release_page() {
    assert_eq!(
        release_tag_in_url("https://github.com/q0wkp/zap-ng/releases/tag/v2026.07.09.2-mod.1.1\n"),
        Some(RELEASE_TAG)
    );
    // Where `latest` redirects to in a repository without a release.
    assert_eq!(
        release_tag_in_url("https://github.com/q0wkp/zap-ng/releases"),
        None
    );
    assert_eq!(
        release_tag_in_url("https://github.com/q0wkp/zap-ng/releases/tag/"),
        None
    );
}

#[test]
fn install_script_uses_zap_ng_asset_and_staging_placeholder() {
    let script = install_script_for(
        Some(RELEASE_TAG),
        Some("~/.zap-ng/remote-server/zap-ng-upload.tar.gz"),
        None,
    );

    assert!(script
        .contains("staging_tarball_path=\"~/.zap-ng/remote-server/zap-ng-upload.tar.gz\""));
    assert!(script.contains(&format!("release_tag=\"{RELEASE_TAG}\"")));
    assert!(script.contains(
        "url=\"https://github.com/q0wkp/zap-ng/releases/download/v$release_version/zap-ng-$release_version-$os_name-$arch_name.tar.gz\""
    ));
    assert!(!script.contains("app.warp.dev"));
    assert!(!script.contains("/download/cli"));
}

/// The program an install script downloads with.
#[derive(Clone, Copy)]
enum Downloader {
    Curl,
    /// On a host without curl.
    Wget,
}

/// What running an install script did.
struct InstallScriptRun {
    exit_code: Option<i32>,
    stderr: String,
    /// The command lines of the downloader's and of shasum's calls, in order.
    calls: Vec<String>,
    /// The home directory the script ran in.
    home: tempfile::TempDir,
}

/// The remote host an install script runs on in [`run_install_script`].
struct RemoteHost<'a> {
    downloader: Downloader,
    /// Where GitHub redirects the newest release's page to.
    latest_location: &'a str,
    /// The files the downloader can fetch, by the last part of their URL, with their contents.
    served: Vec<(String, Vec<u8>)>,
    /// The programs the host doesn't have, besides curl for [`Downloader::Wget`].
    missing: &'a [&'a str],
}

impl<'a> RemoteHost<'a> {
    fn new(downloader: Downloader, latest_location: &'a str) -> Self {
        Self {
            downloader,
            latest_location,
            served: Vec::new(),
            missing: &[],
        }
    }

    fn serving(mut self, name: impl Into<String>, contents: impl Into<Vec<u8>>) -> Self {
        self.served.push((name.into(), contents.into()));
        self
    }
}

/// Runs `script` with `bash`, as the SSH transport does on the remote host, in an empty home
/// directory and with a downloader that records its calls and fetches only what `host` serves:
/// asked for the newest release, it answers as the real one does when GitHub redirects to
/// `host.latest_location` (curl's and GNU Wget's output for q0wkp/zap-ng, seen 2026-11-11); any
/// other download fails.
fn run_install_script(script: &str, host: RemoteHost) -> InstallScriptRun {
    let home = tempfile::tempdir().unwrap();
    // Outside the home directory, whose contents some tests look at.
    let host_dir = tempfile::tempdir().unwrap();
    let calls_file = host_dir.path().join("calls");
    let served_dir = host_dir.path().join("served");
    std::fs::create_dir(&served_dir).unwrap();
    for (name, contents) in &host.served {
        std::fs::write(served_dir.join(name), contents).unwrap();
    }
    let mut missing = host.missing.to_vec();
    let downloader_stub = match host.downloader {
        Downloader::Curl => {
            r#"curl() {
  printf 'curl %s\n' "$*" >> "$CALLS_FILE"
  case "$*" in
    *"$LATEST_RELEASE_URL"*) printf 'HTTP/2 302 \r\nlocation: %s\r\n\r\n' "$LATEST_LOCATION"; return ;;
  esac
  serve "$@" || return 22
}"#
        }
        Downloader::Wget => {
            missing.push("curl");
            r#"wget() {
  printf 'wget %s\n' "$*" >> "$CALLS_FILE"
  case "$*" in
    *"$LATEST_RELEASE_URL"*)
      printf '  HTTP/1.1 302 Found\n  Location: %s\nLocation: %s [following]\n  HTTP/1.1 200 OK\n' \
        "$LATEST_LOCATION" "$LATEST_LOCATION" >&2
      return ;;
  esac
  serve "$@" || return 8
}"#
        }
    };
    // `serve` copies the served file that the URL among its arguments names to the path after
    // -o or -O. shasum, which many Linux hosts lack, is computed with sha256sum.
    let stubs = format!(
        r#"command() {{
  if [ "$1" = -v ]; then
    case " $MISSING_COMMANDS " in *" $2 "*) return 1 ;; esac
  fi
  builtin command "$@"
}}
serve() {{
  local arg url= out= previous=
  for arg in "$@"; do
    case "$previous" in -o|-O) out=$arg ;; esac
    case "$arg" in https://*) url=$arg ;; esac
    previous=$arg
  done
  [ -n "$url" ] && [ -n "$out" ] && [ -f "$SERVED_DIR/${{url##*/}}" ] || return 1
  cp "$SERVED_DIR/${{url##*/}}" "$out"
}}
shasum() {{
  printf 'shasum %s\n' "$*" >> "$CALLS_FILE"
  [ "$*" = "-a 256" ] || return 1
  builtin command sha256sum 2>/dev/null || builtin command shasum -a 256
}}
{downloader_stub}"#
    );

    let output = command::blocking::Command::new("bash")
        .arg("-c")
        .arg(format!("{stubs}\n{script}"))
        .env("HOME", home.path())
        .env("CALLS_FILE", &calls_file)
        .env("SERVED_DIR", &served_dir)
        .env("MISSING_COMMANDS", missing.join(" "))
        .env("LATEST_RELEASE_URL", LATEST_RELEASE_URL)
        .env("LATEST_LOCATION", host.latest_location)
        .output()
        .unwrap();

    InstallScriptRun {
        exit_code: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        calls: std::fs::read_to_string(&calls_file)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect(),
        home,
    }
}

/// The URL of the tarball that the release [`RELEASE_TAG`] has for the system the tests run on,
/// which the install script takes for the remote host.
fn release_tarball_url_of_this_system() -> String {
    // The names the script and the release workflow give this system.
    let os = match std::env::consts::OS {
        "macos" => "macos",
        "linux" => "linux",
        other => panic!("the install script doesn't support {other}"),
    };
    let arch = std::env::consts::ARCH;
    let url = release_tarball_url(RELEASE_TAG, os, arch);
    assert!(url.ends_with(&format!("/zap-ng-{RELEASE_VERSION}-{os}-{arch}.tar.gz")));
    url
}

/// Where GitHub redirects `latest` to while [`RELEASE_TAG`] is the newest release.
const LATEST_LOCATION: &str = "https://github.com/q0wkp/zap-ng/releases/tag/v2026.07.09.2-mod.1.1";

/// The last part of `url`, the name of the file it downloads.
fn file_name_in_url(url: &str) -> &str {
    url.rsplit_once('/').unwrap().1
}

/// The SHA-256 of `contents` in lower-case hex.
fn sha256_hex(contents: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(contents))
}

/// A release's SHA-256 file for the file `name` with `contents`, as `sha256sum` writes it.
fn checksum_file(name: &str, contents: &[u8]) -> String {
    format!("{}  {name}\n", sha256_hex(contents))
}

/// A CLI tarball as the release workflow packs it, with the binary at its top.
fn cli_tarball() -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(binary_name()), "#!/bin/sh\necho zap-ng\n").unwrap();
    std::fs::create_dir(dir.path().join("resources")).unwrap();
    let tarball = dir.path().join("cli.tar.gz");
    let status = command::blocking::Command::new("tar")
        .arg("czf")
        .arg(&tarball)
        .arg("-C")
        .arg(dir.path())
        .arg(binary_name())
        .arg("resources")
        .status()
        .unwrap();
    assert!(status.success());
    std::fs::read(tarball).unwrap()
}

/// Where the install script that ran in `home` installs the binary.
fn installed_binary(home: &std::path::Path) -> std::path::PathBuf {
    std::path::PathBuf::from(remote_server_binary().replacen('~', home.to_str().unwrap(), 1))
}

/// What the install directory in `home` holds besides the installed binary, such as a staging
/// directory left behind.
fn install_dir_leftovers(home: &std::path::Path) -> Vec<String> {
    let install_dir =
        std::path::PathBuf::from(remote_server_dir().replacen('~', home.to_str().unwrap(), 1));
    let binary = installed_binary(home);
    std::fs::read_dir(install_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| *path != binary)
        .map(|path| path.display().to_string())
        .collect()
}

#[test]
fn install_script_downloads_the_tarball_of_its_release_tag() {
    let script = install_script_for(Some(RELEASE_TAG), None, None);
    let url = release_tarball_url_of_this_system();
    let name = file_name_in_url(&url);
    // The newest release is another one, which a build with a tag doesn't ask for.
    let host = RemoteHost::new(
        Downloader::Curl,
        "https://github.com/q0wkp/zap-ng/releases/tag/v2026.07.09.2-mod.5",
    )
    .serving(format!("{name}.sha256"), checksum_file(name, b"tarball"));
    let run = run_install_script(&script, host);

    // The SHA-256 first, then the tarball.
    assert_eq!(run.calls.len(), 2, "{:?}", run.calls);
    let checksum_url = checksum_url(&url);
    assert!(
        run.calls[0].split(' ').any(|arg| arg == checksum_url),
        "{:?}",
        run.calls
    );
    assert!(
        run.calls[1].split(' ').any(|arg| arg == url),
        "{:?}",
        run.calls
    );
    // The exit code of the curl that failed to download.
    assert_eq!(run.exit_code, Some(22), "{}", run.stderr);
}

#[test]
fn install_script_without_release_tag_downloads_the_newest_release_with_curl() {
    let script = install_script_for(None, None, None);
    let url = release_tarball_url_of_this_system();
    let name = file_name_in_url(&url);
    let host = RemoteHost::new(Downloader::Curl, LATEST_LOCATION)
        .serving(format!("{name}.sha256"), checksum_file(name, b"tarball"));
    let run = run_install_script(&script, host);

    assert_eq!(run.calls.len(), 3, "{:?}", run.calls);
    assert!(run.calls[0].starts_with("curl "), "{:?}", run.calls);
    assert!(run.calls[0].contains(LATEST_RELEASE_URL), "{:?}", run.calls);
    assert!(
        run.calls[2].split(' ').any(|arg| arg == url),
        "{:?}",
        run.calls
    );
    assert_eq!(run.exit_code, Some(22), "{}", run.stderr);
}

#[test]
fn install_script_without_release_tag_downloads_the_newest_release_with_wget() {
    let script = install_script_for(None, None, None);
    let url = release_tarball_url_of_this_system();
    let name = file_name_in_url(&url);
    let host = RemoteHost::new(Downloader::Wget, LATEST_LOCATION)
        .serving(format!("{name}.sha256"), checksum_file(name, b"tarball"));
    let run = run_install_script(&script, host);

    assert_eq!(run.calls.len(), 3, "{:?}", run.calls);
    assert!(run.calls[0].starts_with("wget "), "{:?}", run.calls);
    assert!(run.calls[0].contains(LATEST_RELEASE_URL), "{:?}", run.calls);
    assert!(
        run.calls[2].split(' ').any(|arg| arg == url),
        "{:?}",
        run.calls
    );
    // The exit code of the wget that failed to download.
    assert_eq!(run.exit_code, Some(8), "{}", run.stderr);
}

#[test]
fn install_script_without_release_tag_fails_when_no_release_is_found() {
    let script = install_script_for(None, None, None);
    // Where `latest` redirects to in a repository without a release.
    let no_release = "https://github.com/q0wkp/zap-ng/releases";

    for downloader in [Downloader::Curl, Downloader::Wget] {
        let run = run_install_script(&script, RemoteHost::new(downloader, no_release));

        // Only the lookup, and no download.
        assert_eq!(run.calls.len(), 1, "{:?}", run.calls);
        // Not 2, after which the client wouldn't try to download and upload the tarball itself.
        assert_eq!(run.exit_code, Some(1), "{}", run.stderr);
        assert!(
            run.stderr.contains("could not find the newest release"),
            "{}",
            run.stderr
        );
    }
}

#[test]
fn install_script_installs_a_tarball_whose_sha256_matches() {
    let script = install_script_for(Some(RELEASE_TAG), None, None);
    let url = release_tarball_url_of_this_system();
    let name = file_name_in_url(&url);
    let tarball = cli_tarball();

    for downloader in [Downloader::Curl, Downloader::Wget] {
        let host = RemoteHost::new(downloader, LATEST_LOCATION)
            .serving(format!("{name}.sha256"), checksum_file(name, &tarball))
            .serving(name, tarball.clone());
        let run = run_install_script(&script, host);

        assert_eq!(run.exit_code, Some(0), "{}", run.stderr);
        assert!(installed_binary(run.home.path()).is_file());
        assert!(
            install_dir_leftovers(run.home.path()).is_empty(),
            "{:?}",
            install_dir_leftovers(run.home.path())
        );
    }
}

#[test]
fn install_script_verifies_with_shasum_without_sha256sum() {
    let script = install_script_for(Some(RELEASE_TAG), None, None);
    let url = release_tarball_url_of_this_system();
    let name = file_name_in_url(&url);
    let tarball = cli_tarball();

    // As on macOS, which has shasum only.
    let mut host = RemoteHost::new(Downloader::Curl, LATEST_LOCATION)
        .serving(format!("{name}.sha256"), checksum_file(name, &tarball))
        .serving(name, tarball);
    host.missing = &["sha256sum"];
    let run = run_install_script(&script, host);

    assert_eq!(run.exit_code, Some(0), "{}", run.stderr);
    assert!(
        run.calls.iter().any(|call| call == "shasum -a 256"),
        "{:?}",
        run.calls
    );
    assert!(installed_binary(run.home.path()).is_file());

    // A mismatch is found with shasum as well.
    let mut host = RemoteHost::new(Downloader::Curl, LATEST_LOCATION)
        .serving(
            format!("{name}.sha256"),
            checksum_file(name, b"another tarball"),
        )
        .serving(name, cli_tarball());
    host.missing = &["sha256sum"];
    let run = run_install_script(&script, host);
    assert_eq!(run.exit_code, Some(6), "{}", run.stderr);
}

#[test]
fn install_script_refuses_a_tarball_whose_sha256_differs() {
    let script = install_script_for(Some(RELEASE_TAG), None, None);
    let url = release_tarball_url_of_this_system();
    let name = file_name_in_url(&url);
    let host = RemoteHost::new(Downloader::Curl, LATEST_LOCATION)
        .serving(
            format!("{name}.sha256"),
            checksum_file(name, b"another tarball"),
        )
        .serving(name, cli_tarball());
    let run = run_install_script(&script, host);

    assert_eq!(run.exit_code, Some(6), "{}", run.stderr);
    assert!(run.stderr.contains("SHA-256 mismatch"), "{}", run.stderr);
    assert!(run.stderr.contains(&url), "{}", run.stderr);
    // Neither installed nor left behind extracted.
    assert!(!installed_binary(run.home.path()).exists());
    assert!(
        install_dir_leftovers(run.home.path()).is_empty(),
        "{:?}",
        install_dir_leftovers(run.home.path())
    );
}

#[test]
fn install_script_refuses_a_release_without_sha256() {
    let script = install_script_for(Some(RELEASE_TAG), None, None);
    let url = release_tarball_url_of_this_system();
    let name = file_name_in_url(&url);

    for downloader in [Downloader::Curl, Downloader::Wget] {
        // As a release made before checksums were published.
        let host = RemoteHost::new(downloader, LATEST_LOCATION).serving(name, cli_tarball());
        let run = run_install_script(&script, host);

        assert_eq!(run.exit_code, Some(5), "{}", run.stderr);
        assert!(run.stderr.contains(&checksum_url(&url)), "{}", run.stderr);
        // The tarball isn't downloaded.
        assert_eq!(run.calls.len(), 1, "{:?}", run.calls);
        assert!(!installed_binary(run.home.path()).exists());
    }
}

#[test]
fn install_script_refuses_a_sha256_file_without_sha256() {
    let script = install_script_for(Some(RELEASE_TAG), None, None);
    let url = release_tarball_url_of_this_system();
    let name = file_name_in_url(&url);

    for contents in ["", "Not Found\n", "0123abcd  short\n"] {
        let host = RemoteHost::new(Downloader::Curl, LATEST_LOCATION)
            .serving(format!("{name}.sha256"), contents)
            .serving(name, cli_tarball());
        let run = run_install_script(&script, host);

        assert_eq!(run.exit_code, Some(5), "{contents:?}: {}", run.stderr);
        assert!(run.stderr.contains("holds no SHA-256"), "{}", run.stderr);
        assert!(!installed_binary(run.home.path()).exists());
    }
}

#[test]
fn install_script_without_sha256_program_downloads_nothing() {
    let script = install_script_for(Some(RELEASE_TAG), None, None);
    let mut host = RemoteHost::new(Downloader::Curl, LATEST_LOCATION);
    host.missing = &["sha256sum", "shasum"];
    let run = run_install_script(&script, host);

    // Not 2, so that the client downloads, verifies and uploads the tarball itself.
    assert_eq!(run.exit_code, Some(4), "{}", run.stderr);
    assert!(
        run.stderr.contains("neither sha256sum nor shasum"),
        "{}",
        run.stderr
    );
    assert!(run.calls.is_empty(), "{:?}", run.calls);
}

#[test]
fn install_script_extracts_an_uploaded_tarball_without_sha256_program() {
    // The client verified the tarball it uploaded.
    let fake_home = tempfile::tempdir().unwrap();
    let uploaded = fake_home.path().join(SCP_UPLOAD_TARBALL_NAME);
    std::fs::write(&uploaded, cli_tarball()).unwrap();
    let script = install_script_for(Some(RELEASE_TAG), Some(uploaded.to_str().unwrap()), None);

    let output = command::blocking::Command::new("bash")
        .arg("-c")
        .arg(format!(
            "command() {{ case \"$1 $2\" in '-v sha256sum'|'-v shasum') return 1 ;; esac; builtin command \"$@\"; }}\n{script}"
        ))
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(installed_binary(fake_home.path()).is_file());
}

#[test]
fn checksum_url_appends_sha256_to_the_tarball() {
    let url = release_tarball_url(RELEASE_TAG, "linux", "x86_64");
    assert_eq!(checksum_url(&url), format!("{url}.sha256"));
    assert!(install_script_for(Some(RELEASE_TAG), None, None).contains(
        "checksum_url=\"https://github.com/q0wkp/zap-ng/releases/download/v$release_version/zap-ng-$release_version-$os_name-$arch_name.tar.gz.sha256\""
    ));
}

#[test]
fn parse_checksum_file_reads_sha256sum_output() {
    let sha256 = sha256_hex(b"tarball");
    assert_eq!(
        parse_checksum_file(&format!("{sha256}  zap-ng.tar.gz\n")),
        Some(sha256.clone())
    );
    // shasum's binary mode marker, and no name or newline at all.
    assert_eq!(
        parse_checksum_file(&format!("{sha256} *zap-ng.tar.gz")),
        Some(sha256.clone())
    );
    assert_eq!(
        parse_checksum_file(&sha256.to_uppercase()),
        Some(sha256.clone())
    );
    assert_eq!(parse_checksum_file(""), None);
    assert_eq!(parse_checksum_file("Not Found"), None);
    assert_eq!(parse_checksum_file(&sha256[1..]), None);
    assert_eq!(parse_checksum_file(&format!("{sha256}0")), None);
}

#[test]
fn verify_sha256_accepts_only_the_published_sha256() {
    let dir = tempfile::tempdir().unwrap();
    let tarball = dir.path().join("zap-ng.tar.gz");
    std::fs::write(&tarball, b"tarball").unwrap();

    verify_sha256(
        &tarball,
        &checksum_file("zap-ng.tar.gz", b"tarball"),
        "zap-ng.tar.gz",
    )
    .unwrap();

    let error = verify_sha256(
        &tarball,
        &checksum_file("zap-ng.tar.gz", b"other"),
        "zap-ng.tar.gz",
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("SHA-256 mismatch for zap-ng.tar.gz"), "{error}");
    assert!(error.contains(&sha256_hex(b"tarball")), "{error}");

    let error = verify_sha256(&tarball, "Not Found", "zap-ng.tar.gz")
        .unwrap_err()
        .to_string();
    assert!(error.contains("holds no SHA-256"), "{error}");
}

/// Runs [`previous_install_check_command`] against `fake_home` and returns
/// whether it found a prior install.
fn run_previous_install_check(fake_home: &std::path::Path) -> bool {
    let output = command::blocking::Command::new("bash")
        .arg("-c")
        .arg(previous_install_check_command())
        .env("HOME", fake_home)
        .output()
        .expect("failed to run previous install check");
    match output.status.code() {
        Some(0) => true,
        Some(1) => false,
        code => panic!(
            "previous install check exited with {code:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
    }
}

#[test]
fn previous_install_check_ignores_what_a_failed_install_leaves() {
    let fake_home = tempfile::tempdir().unwrap();
    let resolve_remote_path =
        |path: String| path.replacen('~', fake_home.path().to_str().unwrap(), 1);
    let install_dir = std::path::PathBuf::from(resolve_remote_path(remote_server_dir()));

    assert!(!run_previous_install_check(fake_home.path()));

    // An install whose tarball never arrived fails after creating the
    // install directory.
    let missing_tarball = fake_home.path().join("missing.tar.gz");
    let install_output = command::blocking::Command::new("bash")
        .arg("-c")
        .arg(install_script(
            Some(missing_tarball.to_str().unwrap()),
            None,
        ))
        .env("HOME", fake_home.path())
        .output()
        .expect("failed to run install script");
    assert!(!install_output.status.success());
    assert!(install_dir.is_dir());
    assert!(!run_previous_install_check(fake_home.path()));

    // Interrupted installs can also leave a staging directory and the
    // uploaded tarball behind.
    std::fs::create_dir(install_dir.join(".install.AbC123")).unwrap();
    std::fs::write(install_dir.join(SCP_UPLOAD_TARBALL_NAME), "").unwrap();
    assert!(!run_previous_install_check(fake_home.path()));

    // The daemon's directories outlast the removal of a stale binary.
    std::fs::create_dir(install_dir.join(remote_server_identity_dir_name("identity"))).unwrap();
    assert!(run_previous_install_check(fake_home.path()));
}

#[test]
fn previous_install_check_finds_an_installed_binary() {
    let fake_home = tempfile::tempdir().unwrap();
    let binary = std::path::PathBuf::from(remote_server_binary().replacen(
        '~',
        fake_home.path().to_str().unwrap(),
        1,
    ));
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    std::fs::write(&binary, "#!/bin/sh\n").unwrap();
    assert!(run_previous_install_check(fake_home.path()));
}

#[test]
fn install_script_takes_the_sha256_from_the_client_without_downloading_it() {
    let url = release_tarball_url_of_this_system();
    let name = file_name_in_url(&url);
    let tarball = cli_tarball();

    // The release's own SHA-256 file is not served: asking for it would fail the install.
    let script = install_script_for(Some(RELEASE_TAG), None, Some(&sha256_hex(&tarball)));
    let host = RemoteHost::new(Downloader::Curl, LATEST_LOCATION).serving(name, tarball.clone());
    let run = run_install_script(&script, host);
    assert_eq!(run.exit_code, Some(0), "{}", run.stderr);
    assert!(installed_binary(run.home.path()).is_file());
    assert!(
        !run.calls.iter().any(|call| call.contains(".sha256")),
        "{:?}",
        run.calls
    );

    // A tarball with another SHA-256 is refused, also when the release's file agrees with it.
    let script = install_script_for(
        Some(RELEASE_TAG),
        None,
        Some(&sha256_hex(b"the signed one")),
    );
    let host = RemoteHost::new(Downloader::Curl, LATEST_LOCATION)
        .serving(format!("{name}.sha256"), checksum_file(name, &tarball))
        .serving(name, tarball);
    let run = run_install_script(&script, host);
    assert_eq!(run.exit_code, Some(6), "{}", run.stderr);
    assert!(!installed_binary(run.home.path()).exists());
}

/// An OpenPGP key made for these tests, and what `gpg --armor --detach-sign`, the release
/// workflow's command, signed [`SIGNED_CHECKSUM_FILE`] with (GnuPG 2.4.4).
const TEST_PUBLIC_KEY: &str = "GKVkn7V0ie380dQ4aG7GuY4/JmgFdM8GTdNV97Ahp1k=";
const SIGNED_CHECKSUM_FILE: &str =
    "db4b4d0d1cb480bf9aeea253771c00febe627f236765fa37d6a5614f079a3aa0  zap-ng-2026.07.09.2-mod.1.1-linux-x86_64.tar.gz\n";
const TEST_SIGNATURE: &str = "-----BEGIN PGP SIGNATURE-----

iHUEABYKAB0WIQTyAlrrPOkpyR+KM/adV1wUAJJWawUCav/SwgAKCRCdV1wUAJJW
a5b1AQDHH8f6U/3mpPSf8b42x453YrVJPJnF9TGkE3yBMDrDnQD+JDeZNz0R+0Xw
PFzqYAnkt1kVDrUqJ4bvr+Up4Z/xQw8=
=AWSe
-----END PGP SIGNATURE-----
";

#[test]
fn signed_sha256_file_gives_its_sha256() {
    let url = release_tarball_url(RELEASE_TAG, "linux", "x86_64");
    assert_eq!(
        sha256_signed_with(TEST_PUBLIC_KEY, SIGNED_CHECKSUM_FILE, TEST_SIGNATURE, &url).unwrap(),
        "db4b4d0d1cb480bf9aeea253771c00febe627f236765fa37d6a5614f079a3aa0"
    );
}

#[test]
fn sha256_file_changed_after_signing_is_refused() {
    let url = release_tarball_url(RELEASE_TAG, "linux", "x86_64");
    let changed = SIGNED_CHECKSUM_FILE.replacen("db4b", "0000", 1);
    let error = sha256_signed_with(TEST_PUBLIC_KEY, &changed, TEST_SIGNATURE, &url).unwrap_err();
    assert!(error.to_string().contains("signature"), "{error}");

    // Signed with another key than the client's.
    let error = sha256_signed_with(
        RELEASE_SIGNING_PUBLIC_KEY,
        SIGNED_CHECKSUM_FILE,
        TEST_SIGNATURE,
        &url,
    )
    .unwrap_err();
    assert!(error.to_string().contains("signature"), "{error}");

    // Not a signature at all, one cut short, and one with a byte of its hashed fields changed.
    let truncated = TEST_SIGNATURE.replacen("PFzqYAnkt1kVDrUqJ4bvr+Up4Z/xQw8=\n", "", 1);
    let altered = TEST_SIGNATURE.replacen("iHUEABYKAB0W", "iHUEABYKAB0X", 1);
    for (signature, expected) in [
        ("not a signature", "can't be read"),
        (truncated.as_str(), "can't be read"),
        (altered.as_str(), "does not carry"),
    ] {
        let error = sha256_signed_with(TEST_PUBLIC_KEY, SIGNED_CHECKSUM_FILE, signature, &url)
            .unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn signed_sha256_file_of_another_tarball_is_refused() {
    // The same release's file for another system, and another release's for this one.
    for url in [
        release_tarball_url(RELEASE_TAG, "macos", "aarch64"),
        release_tarball_url("v2026.01.01.1", "linux", "x86_64"),
    ] {
        let error = sha256_signed_with(TEST_PUBLIC_KEY, SIGNED_CHECKSUM_FILE, TEST_SIGNATURE, &url)
            .unwrap_err();
        assert!(error.to_string().contains("signed, but for"), "{error}");
    }
}

#[test]
fn release_signing_key_is_an_ed25519_public_key() {
    use base64::prelude::{Engine as _, BASE64_STANDARD};
    let key = BASE64_STANDARD
        .decode(RELEASE_SIGNING_PUBLIC_KEY.trim())
        .unwrap();
    assert_eq!(key.len(), 32);
}
