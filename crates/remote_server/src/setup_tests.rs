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
fn oss_remote_server_dir_uses_zap_namespace() {
    assert_eq!(remote_server_dir(), "~/.zap/remote-server");
}

#[test]
fn oss_binary_name_matches_zap_cli() {
    assert_eq!(binary_name(), "zap-oss");
}

/// A release tag, and the version its files carry.
const RELEASE_TAG: &str = "v2026.07.09.2-mod.4";
const RELEASE_VERSION: &str = "2026.07.09.2-mod.4";

#[test]
fn release_tarball_url_names_the_file_by_version() {
    let url = release_tarball_url(RELEASE_TAG, "linux", "x86_64");

    assert_eq!(
        url,
        "https://github.com/q0wkp/zap/releases/download/v2026.07.09.2-mod.4/zap-2026.07.09.2-mod.4-linux-x86_64.tar.gz"
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
        release_tag_in_url("https://github.com/q0wkp/zap/releases/tag/v2026.07.09.2-mod.4\n"),
        Some(RELEASE_TAG)
    );
    // Where `latest` redirects to in a repository without a release.
    assert_eq!(
        release_tag_in_url("https://github.com/q0wkp/zap/releases"),
        None
    );
    assert_eq!(
        release_tag_in_url("https://github.com/q0wkp/zap/releases/tag/"),
        None
    );
}

#[test]
fn install_script_uses_zap_asset_and_staging_placeholder() {
    let script = install_script_for(
        Some(RELEASE_TAG),
        Some("~/.zap/remote-server/zap-upload.tar.gz"),
    );

    assert!(script
        .contains("staging_tarball_path=\"~/.zap/remote-server/zap-upload.tar.gz\""));
    assert!(script.contains(&format!("release_tag=\"{RELEASE_TAG}\"")));
    assert!(script.contains(
        "url=\"https://github.com/q0wkp/zap/releases/download/v$release_version/zap-$release_version-$os_name-$arch_name.tar.gz\""
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
    /// The command lines of the downloader's calls, in order.
    calls: Vec<String>,
}

/// Runs `script` with `bash`, as the SSH transport does on the remote host, in an empty home
/// directory and with a downloader that records its calls and fetches nothing: asked for the
/// newest release, it answers as the real one does when GitHub redirects to `latest_location`
/// (curl's and GNU Wget's output for q0wkp/zap, seen 2026-11-11); any other download fails.
fn run_install_script(
    script: &str,
    downloader: Downloader,
    latest_location: &str,
) -> InstallScriptRun {
    let dir = tempfile::tempdir().unwrap();
    let calls_file = dir.path().join("calls");
    let stubs = match downloader {
        Downloader::Curl => {
            r#"curl() {
  printf 'curl %s\n' "$*" >> "$CALLS_FILE"
  case "$*" in
    *"$LATEST_RELEASE_URL"*) printf 'HTTP/2 302 \r\nlocation: %s\r\n\r\n' "$LATEST_LOCATION" ;;
    *) return 22 ;;
  esac
}"#
        }
        Downloader::Wget => {
            r#"command() {
  if [ "$1" = -v ] && [ "$2" = curl ]; then return 1; fi
  builtin command "$@"
}
wget() {
  printf 'wget %s\n' "$*" >> "$CALLS_FILE"
  case "$*" in
    *"$LATEST_RELEASE_URL"*)
      printf '  HTTP/1.1 302 Found\n  Location: %s\nLocation: %s [following]\n  HTTP/1.1 200 OK\n' \
        "$LATEST_LOCATION" "$LATEST_LOCATION" >&2 ;;
    *) return 8 ;;
  esac
}"#
        }
    };

    let output = command::blocking::Command::new("bash")
        .arg("-c")
        .arg(format!("{stubs}\n{script}"))
        .env("HOME", dir.path())
        .env("CALLS_FILE", &calls_file)
        .env("LATEST_RELEASE_URL", LATEST_RELEASE_URL)
        .env("LATEST_LOCATION", latest_location)
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
    assert!(url.ends_with(&format!("/zap-{RELEASE_VERSION}-{os}-{arch}.tar.gz")));
    url
}

/// Where GitHub redirects `latest` to while [`RELEASE_TAG`] is the newest release.
const LATEST_LOCATION: &str = "https://github.com/q0wkp/zap/releases/tag/v2026.07.09.2-mod.4";

#[test]
fn install_script_downloads_the_tarball_of_its_release_tag() {
    let script = install_script_for(Some(RELEASE_TAG), None);
    // The newest release is another one, which a build with a tag doesn't ask for.
    let run = run_install_script(
        &script,
        Downloader::Curl,
        "https://github.com/q0wkp/zap/releases/tag/v2026.07.09.2-mod.5",
    );

    let url = release_tarball_url_of_this_system();
    assert_eq!(run.calls.len(), 1, "{:?}", run.calls);
    assert!(
        run.calls[0].split(' ').any(|arg| arg == url),
        "{:?}",
        run.calls
    );
    // The exit code of the curl that failed to download.
    assert_eq!(run.exit_code, Some(22), "{}", run.stderr);
}

#[test]
fn install_script_without_release_tag_downloads_the_newest_release_with_curl() {
    let script = install_script_for(None, None);
    let run = run_install_script(&script, Downloader::Curl, LATEST_LOCATION);

    let url = release_tarball_url_of_this_system();
    assert_eq!(run.calls.len(), 2, "{:?}", run.calls);
    assert!(run.calls[0].starts_with("curl "), "{:?}", run.calls);
    assert!(run.calls[0].contains(LATEST_RELEASE_URL), "{:?}", run.calls);
    assert!(
        run.calls[1].split(' ').any(|arg| arg == url),
        "{:?}",
        run.calls
    );
    assert_eq!(run.exit_code, Some(22), "{}", run.stderr);
}

#[test]
fn install_script_without_release_tag_downloads_the_newest_release_with_wget() {
    let script = install_script_for(None, None);
    let run = run_install_script(&script, Downloader::Wget, LATEST_LOCATION);

    let url = release_tarball_url_of_this_system();
    assert_eq!(run.calls.len(), 2, "{:?}", run.calls);
    assert!(run.calls[0].starts_with("wget "), "{:?}", run.calls);
    assert!(run.calls[0].contains(LATEST_RELEASE_URL), "{:?}", run.calls);
    assert!(
        run.calls[1].split(' ').any(|arg| arg == url),
        "{:?}",
        run.calls
    );
    // The exit code of the wget that failed to download.
    assert_eq!(run.exit_code, Some(8), "{}", run.stderr);
}

#[test]
fn install_script_without_release_tag_fails_when_no_release_is_found() {
    let script = install_script_for(None, None);
    // Where `latest` redirects to in a repository without a release.
    let no_release = "https://github.com/q0wkp/zap/releases";

    for downloader in [Downloader::Curl, Downloader::Wget] {
        let run = run_install_script(&script, downloader, no_release);

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
