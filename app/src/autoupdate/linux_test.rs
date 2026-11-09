use std::fs;

use super::*;

/// The metadata Pacstall 6.3.8 wrote for q0wkp/pkgs' `zmx-bin`, with its names changed to Zap's.
const ZAP_DEB_METADATA: &str = r#"_name="zap-deb"
_version="2026.07.09.2.mod.2-pacstall1"
_install_size="180 MB"
_date="Sat Nov  7 07:32:44 PM EST 2026"
declare -a _maintainer=([0]="q0wkp <q0wkp@users.noreply.github.com>")
_homepage="https://github.com/q0wkp/zap"
_gives="zap"
_remoterepo="https://github.com/q0wkp/pkgs"
_remotebranch="main"
"#;

#[test]
fn test_parse_pacstall_metadata() {
    assert_eq!(
        parse_pacstall_metadata(ZAP_DEB_METADATA, "zap-deb".to_owned()),
        ("zap-deb".to_owned(), "zap".to_owned())
    );
    // Without `_gives`, the dpkg package has the Pacstall package's name.
    assert_eq!(
        parse_pacstall_metadata(
            "_name=\"lazygit\"\n_version=\"0.44.1\"\n",
            "lazygit".to_owned()
        ),
        ("lazygit".to_owned(), "lazygit".to_owned())
    );
    assert_eq!(
        parse_pacstall_metadata("_version=\"1\"\n", "neovim".to_owned()),
        ("neovim".to_owned(), "neovim".to_owned())
    );
}

#[test]
fn test_pacstall_package_for() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("lazygit"),
        "_name=\"lazygit\"\n_gives=\"lazygit\"\n",
    )
    .unwrap();
    assert_eq!(pacstall_package_for("zap", dir.path()), None);

    fs::write(dir.path().join("zap-deb"), ZAP_DEB_METADATA).unwrap();
    assert_eq!(
        pacstall_package_for("zap", dir.path()),
        Some("zap-deb".to_owned())
    );
    assert_eq!(pacstall_package_for("zap-deb", dir.path()), None);
    assert_eq!(
        pacstall_package_for("zap", &dir.path().join("missing")),
        None
    );
}

#[test]
fn test_upgrade_commands() {
    let package_name = "zap".to_owned();
    let url = "https://github.com/q0wkp/pkgs/releases/download/x86_64/zap-2026.07.09.2_mod.3-1-x86_64.pkg.tar.zst";
    let pacman_release = PkgsRelease {
        pacman_package_url: Some(url.to_owned()),
    };
    let pacstall_release = PkgsRelease {
        pacman_package_url: None,
    };

    // From q0wkp/pkgs' [q0wkp]: the release's package file alone, quoted for the shell (`:`).
    let q0wkp = PackageManager::PacmanQ0wkp {
        package_name: package_name.clone(),
    };
    assert_eq!(q0wkp.pkgs_target(), Some(PkgsTarget::Pacman));
    assert_eq!(
        q0wkp.upgrade_command(&pacman_release),
        Some(format!("sudo pacman -U '{url}'"))
    );

    // Pacstall: only Zap's own package.
    let pacstall = PackageManager::Pacstall {
        pacstall_name: "zap-deb".to_owned(),
        package_name: package_name.clone(),
    };
    assert_eq!(pacstall.pkgs_target(), Some(PkgsTarget::Pacstall));
    assert_eq!(
        pacstall.upgrade_command(&pacstall_release).as_deref(),
        Some("pacstall -I zap-deb")
    );
    assert_eq!(pacstall.upgrade_hint(), "pacstall -I zap-deb");

    // Another sync repository, packages installed by hand or from the AUR only get a hint.
    let repo = PackageManager::PacmanRepo {
        package_name: package_name.clone(),
    };
    assert_eq!(repo.pkgs_target(), None);
    assert_eq!(repo.upgrade_command(&pacman_release), None);
    assert_eq!(repo.upgrade_hint(), "sudo pacman -Syu");
    let apt = PackageManager::Apt {
        package_name: package_name.clone(),
    };
    assert_eq!(apt.pkgs_target(), None);
    assert_eq!(apt.upgrade_command(&pacstall_release), None);
    assert_eq!(apt.upgrade_hint(), "sudo apt install ./zap_*.deb");
    let dnf = PackageManager::Dnf {
        package_name: package_name.clone(),
    };
    assert_eq!(dnf.pkgs_target(), None);
    assert_eq!(dnf.upgrade_hint(), "sudo dnf install ./zap-*.rpm");
    let aur = PackageManager::PacmanAur {
        package_name: "zap-bin".to_owned(),
    };
    assert_eq!(aur.pkgs_target(), None);
    assert_eq!(aur.upgrade_hint(), "paru -Syu zap-bin / yay -Syu zap-bin");
}
