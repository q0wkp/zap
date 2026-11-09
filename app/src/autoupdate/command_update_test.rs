use std::fs;

use super::*;

#[test]
fn test_tab_command_reports_success_to_zap() {
    let pacman = "sudo pacman -U https://github.com/q0wkp/pkgs/releases/download/x86_64/zap-1_mod.1-1-x86_64.pkg.tar.zst";
    assert_eq!(
        tab_command(pacman, ShellType::Bash, "abc1234"),
        format!("{pacman} && warp_finish_update abc1234")
    );
    assert_eq!(
        tab_command("pacstall -I zap-deb", ShellType::Zsh, "abc1234"),
        "pacstall -I zap-deb && warp_finish_update abc1234"
    );
    assert_eq!(
        tab_command(pacman, ShellType::Fish, "abc1234"),
        format!("{pacman}; and warp_finish_update abc1234")
    );
}

#[test]
fn test_file_identity_changes_when_the_file_is_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("zap");
    fs::write(&path, "old version").unwrap();
    let before = FileIdentity::of(&path).unwrap();
    assert_eq!(FileIdentity::of(&path).unwrap(), before);

    // Package managers unpack the new file next to the old one and rename it into place.
    let new_path = dir.path().join("zap.new");
    fs::write(&new_path, "new version").unwrap();
    fs::rename(&new_path, &path).unwrap();
    assert_ne!(FileIdentity::of(&path).unwrap(), before);
}

#[test]
fn test_homebrew_upgrade_command_needs_the_cask_installed() {
    let dir = tempfile::tempdir().unwrap();
    let apple_silicon = dir.path().join("opt/homebrew");
    let intel = dir.path().join("usr/local");
    for prefix in [&apple_silicon, &intel] {
        fs::create_dir_all(prefix.join("bin")).unwrap();
        fs::create_dir_all(prefix.join("Caskroom")).unwrap();
        fs::write(prefix.join("bin/brew"), "").unwrap();
    }
    let prefixes = || [apple_silicon.as_path(), intel.as_path()];

    // Homebrew is installed, but Zap's cask isn't.
    assert_eq!(homebrew_upgrade_command_in(prefixes()), None);

    fs::create_dir_all(intel.join("Caskroom/zap-terminal/2026.07.09.2-mod.2")).unwrap();
    assert_eq!(
        homebrew_upgrade_command_in(prefixes()),
        Some(format!(
            "{} upgrade --cask --greedy zap-terminal",
            intel.join("bin/brew").display()
        ))
    );
}

#[test]
fn test_arch_pkgver_matches_bundle_arch() {
    // What `sed -E 's/^v//; s/[-:/[:space:]]+/_/g'` in script/linux/bundle_arch prints.
    assert_eq!(arch_pkgver("v2026.07.09.2-mod.3"), "2026.07.09.2_mod.3");
    assert_eq!(arch_pkgver("2026.07.09.2"), "2026.07.09.2");
    assert_eq!(arch_pkgver("v1.0--rc: 1/x\ty"), "1.0_rc_1_x_y");
    assert_eq!(arch_pkgver("vv1"), "v1");
}

/// The assets of q0wkp/pkgs' release `x86_64` once it has published Zap's v2026.07.09.2-mod.3.
const PKGS_ASSETS: [&str; 8] = [
    "q0wkp.db",
    "q0wkp.db.sig",
    "q0wkp.files",
    "q0wkp.files.sig",
    "zap-2026.07.09.2_mod.3-1-x86_64.pkg.tar.zst",
    "zap-2026.07.09.2_mod.3-1-x86_64.pkg.tar.zst.sig",
    "zmx-bin-0.8.2.mod.2-1-x86_64.pkg.tar.zst",
    "zmx-bin-0.8.2.mod.2-1-x86_64.pkg.tar.zst.sig",
];

#[test]
fn test_find_pacman_package() {
    let tag = "v2026.07.09.2-mod.3";
    assert_eq!(
        find_pacman_package(PKGS_ASSETS, "zap", tag),
        Some("zap-2026.07.09.2_mod.3-1-x86_64.pkg.tar.zst")
    );
    // Not another version, package or architecture.
    assert_eq!(
        find_pacman_package(PKGS_ASSETS, "zap", "v2026.07.09.2-mod.2"),
        None
    );
    assert_eq!(find_pacman_package(PKGS_ASSETS, "zmx", "v0.8.2"), None);
    assert_eq!(
        find_pacman_package(
            [
                "zap-bin-2026.07.09.2_mod.3-1-x86_64.pkg.tar.zst",
                "zap-bin-2026.07.09.2_mod.3-1-x86_64.pkg.tar.zst.sig",
                "zap-2026.07.09.2_mod.3-1-aarch64.pkg.tar.zst",
                "zap-2026.07.09.2_mod.3-1-aarch64.pkg.tar.zst.sig",
                "zap-2026.07.09.2_mod.3-x-x86_64.pkg.tar.zst",
                "zap-2026.07.09.2_mod.3-x-x86_64.pkg.tar.zst.sig",
            ],
            "zap",
            tag
        ),
        None
    );
    // Not without its signature.
    assert_eq!(
        find_pacman_package(["zap-2026.07.09.2_mod.3-1-x86_64.pkg.tar.zst"], "zap", tag),
        None
    );
    // The highest pkgrel that has its signature.
    assert_eq!(
        find_pacman_package(
            [
                "zap-2026.07.09.2_mod.3-1-x86_64.pkg.tar.zst",
                "zap-2026.07.09.2_mod.3-1-x86_64.pkg.tar.zst.sig",
                "zap-2026.07.09.2_mod.3-10-x86_64.pkg.tar.zst",
                "zap-2026.07.09.2_mod.3-10-x86_64.pkg.tar.zst.sig",
                "zap-2026.07.09.2_mod.3-2.1-x86_64.pkg.tar.zst",
                "zap-2026.07.09.2_mod.3-2.1-x86_64.pkg.tar.zst.sig",
                "zap-2026.07.09.2_mod.3-11-x86_64.pkg.tar.zst",
            ],
            "zap",
            tag
        ),
        Some("zap-2026.07.09.2_mod.3-10-x86_64.pkg.tar.zst")
    );
}

#[test]
fn test_pacman_package_url() {
    assert_eq!(
        pacman_package_url("zap-2026.07.09.2_mod.3-1-x86_64.pkg.tar.zst"),
        "https://github.com/q0wkp/pkgs/releases/download/x86_64/zap-2026.07.09.2_mod.3-1-x86_64.pkg.tar.zst"
    );
}

#[cfg(target_os = "linux")]
mod pkgs_published {
    use mockito::{Server, ServerGuard};

    use super::*;

    async fn published(server: &ServerGuard, target: PkgsTarget, tag: &str) -> Option<PkgsRelease> {
        let url = server.url();
        let source = PkgsSource {
            raw: &url,
            api: &url,
        };
        let client = http_client::Client::new_for_test();
        pkgs_published_from(&source, &client, target, tag)
            .await
            .unwrap()
    }

    async fn serve_published_tag(
        server: &mut ServerGuard,
        target: &str,
        tag: &str,
    ) -> mockito::Mock {
        server
            .mock(
                "GET",
                format!("/q0wkp/pkgs/main/versions/zap/{target}").as_str(),
            )
            .with_body(format!("{tag}\n"))
            .create_async()
            .await
    }

    async fn serve_pacman_assets(server: &mut ServerGuard, assets: &[&str]) -> mockito::Mock {
        let assets: Vec<_> = assets
            .iter()
            .map(|name| {
                serde_json::json!({
                    "name": name,
                    "browser_download_url": format!("https://github.com/q0wkp/pkgs/releases/download/x86_64/{name}"),
                    "size": 1,
                })
            })
            .collect();
        server
            .mock("GET", "/repos/q0wkp/pkgs/releases/tags/x86_64")
            .with_body(
                serde_json::json!({
                    "tag_name": "x86_64",
                    "html_url": "https://github.com/q0wkp/pkgs/releases/tag/x86_64",
                    "assets": assets,
                })
                .to_string(),
            )
            .create_async()
            .await
    }

    #[tokio::test]
    async fn test_not_until_published() {
        // q0wkp/pkgs has never published for the target.
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/q0wkp/pkgs/main/versions/zap/pacstall")
            .with_status(404)
            .create_async()
            .await;
        assert_eq!(
            published(&server, PkgsTarget::Pacstall, "v1.0.0-mod.1").await,
            None
        );

        // It has published an older release.
        let mut server = Server::new_async().await;
        serve_published_tag(&mut server, "pacstall", "v1.0.0-mod.1").await;
        assert_eq!(
            published(&server, PkgsTarget::Pacstall, "v1.0.0-mod.2").await,
            None
        );
    }

    #[tokio::test]
    async fn test_pacstall() {
        let mut server = Server::new_async().await;
        let mock = serve_published_tag(&mut server, "pacstall", "v1.0.1-mod.1")
            .await
            .expect(1);
        let release = PkgsRelease {
            pacman_package_url: None,
        };
        assert_eq!(
            published(&server, PkgsTarget::Pacstall, "v1.0.1-mod.1").await,
            Some(release.clone())
        );
        // Later polls don't ask again.
        assert_eq!(
            published(&server, PkgsTarget::Pacstall, "v1.0.1-mod.1").await,
            Some(release)
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_pacman_needs_the_signed_package() {
        let tag = "v1.0.2-mod.1";
        let package = "zap-1.0.2_mod.1-1-x86_64.pkg.tar.zst";
        let signature = format!("{package}.sig");

        // The version file names the release, but its package isn't signed yet.
        let mut server = Server::new_async().await;
        serve_published_tag(&mut server, "pacman", tag).await;
        serve_pacman_assets(&mut server, &["q0wkp.db", package]).await;
        assert_eq!(published(&server, PkgsTarget::Pacman, tag).await, None);

        let mut server = Server::new_async().await;
        serve_published_tag(&mut server, "pacman", tag).await;
        serve_pacman_assets(&mut server, &["q0wkp.db", package, &signature]).await;
        assert_eq!(
            published(&server, PkgsTarget::Pacman, tag).await,
            Some(PkgsRelease {
                pacman_package_url: Some(format!(
                    "https://github.com/q0wkp/pkgs/releases/download/x86_64/{package}"
                )),
            })
        );
    }
}
