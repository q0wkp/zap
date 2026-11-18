//! The hashed entries in `HASHED` were written by OpenSSH itself (`ssh-keygen -H`, OpenSSH 9.6p1)
//! from `example.com {ED25519}` and `[example.com]:2222 {RSA}`, and `ssh-keygen -F` finds them
//! under those names, so they check the hashing independently of the code under test.

use super::*;

const ED25519: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIP794g2XkN9qcX1m/F+M+f8g7wAKr+NUSxgFGlOLtsOt";
const RSA: &str = "AAAAB3NzaC1yc2EAAAADAQABAAABgQDqwznSv4YG0zMTSLVIqFTodibvlm+3b8y6qQnZyS7i58MEY/glxNnKV7lN1WDHxwgdzeeajK++A/R7R4fvP00wXoCcyCcKJb96bob6MatzECHJfL87i16M97Mvxrj19llDiHJcorv8v2zimQC1oLaEgaLLLWZHysTzkkSnEECse8SXkaKfHwyiqCs130JZuEMEnql0ylL/XhLfNAzf3P+byoc/D5EeYdd+H9iz+nC2pMSZK6dw8R4puBWM/ga3uUWHVsrfX9qRhTPCFtnOpl7JVzJPBuO4X4Jcy6tLO7gwV0GSmHWSe5ZGtXXcItkNRjZHwnCefElnN+fUt5wVE2Pdh88llQZLykw6YpjQJgqFmPi2wQ2LdTnmF8G6fBG+MUrgXlgOkuP35Nui1fPn99swj7luNplKpkP7aNUBs2ZXEewETTyxQoim2EEkXDx3tOp89qp9uJ+pwCBSICG4KU6c7E6V+m99+v0MM4q4xEI+URGR4qsl9iVgdfdC+GWGA3E=";
const ECDSA: &str = "AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBFSJxh2DMbyw4RbMaULO5Vzse/ituC6TVzqWFhqYQudd4YHQx+Ufx9PfWmHHHoAH3SUE2vtJM6D7OO/qJIZlfS4=";

const HASHED: &str = "|1|HAqqW4pP+rYTiyLHKOU9hQq6MtI=|YGzhQKJlMhE+QGYfc8x6/Uah9Nw= ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIP794g2XkN9qcX1m/F+M+f8g7wAKr+NUSxgFGlOLtsOt\n|1|vq/apsIzbCNZnWqQyt1Ce1NHZyw=|IivaeipVtYIn4mBqp0t12irTq0Y= ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQDqwznSv4YG0zMTSLVIqFTodibvlm+3b8y6qQnZyS7i58MEY/glxNnKV7lN1WDHxwgdzeeajK++A/R7R4fvP00wXoCcyCcKJb96bob6MatzECHJfL87i16M97Mvxrj19llDiHJcorv8v2zimQC1oLaEgaLLLWZHysTzkkSnEECse8SXkaKfHwyiqCs130JZuEMEnql0ylL/XhLfNAzf3P+byoc/D5EeYdd+H9iz+nC2pMSZK6dw8R4puBWM/ga3uUWHVsrfX9qRhTPCFtnOpl7JVzJPBuO4X4Jcy6tLO7gwV0GSmHWSe5ZGtXXcItkNRjZHwnCefElnN+fUt5wVE2Pdh88llQZLykw6YpjQJgqFmPi2wQ2LdTnmF8G6fBG+MUrgXlgOkuP35Nui1fPn99swj7luNplKpkP7aNUBs2ZXEewETTyxQoim2EEkXDx3tOp89qp9uJ+pwCBSICG4KU6c7E6V+m99+v0MM4q4xEI+URGR4qsl9iVgdfdC+GWGA3E=";

#[test]
fn hashed_entries_match_the_names_ssh_keygen_hashed() {
    let known = KnownHosts::parse(HASHED);
    assert_eq!(known.verify("example.com", "ssh-ed25519", ED25519), Verdict::Known);
    assert_eq!(
        known.verify(&host_entry_name("example.com", 2222), "ssh-rsa", RSA),
        Verdict::Known
    );
    // The port-22 entry does not stand for another port, nor for another host.
    assert_eq!(
        known.verify(&host_entry_name("example.com", 2222), "ssh-ed25519", ED25519),
        Verdict::Unknown
    );
    assert_eq!(known.verify("example.org", "ssh-ed25519", ED25519), Verdict::Unknown);
}

#[test]
fn a_different_key_of_the_same_type_is_a_change() {
    let known = KnownHosts::parse(HASHED);
    assert_eq!(known.verify("example.com", "ssh-ed25519", RSA), Verdict::Changed);
}

#[test]
fn a_key_of_a_type_not_recorded_is_unknown_not_changed() {
    let known = KnownHosts::parse(HASHED);
    assert_eq!(known.verify("example.com", "ecdsa-sha2-nistp256", ECDSA), Verdict::Unknown);
    assert_eq!(known.key_types_for("example.com"), vec!["ssh-ed25519".to_owned()]);
}

#[test]
fn plain_patterns_wildcards_and_negation() {
    let text = format!(
        "# comment\n\nalias,Example.NET ssh-ed25519 {ED25519}\n*.lan,!secret.lan ssh-rsa {RSA}\n"
    );
    let known = KnownHosts::parse(&text);
    assert_eq!(known.verify("example.net", "ssh-ed25519", ED25519), Verdict::Known);
    assert_eq!(known.verify("alias", "ssh-ed25519", ED25519), Verdict::Known);
    assert_eq!(known.verify("box.lan", "ssh-rsa", RSA), Verdict::Known);
    assert_eq!(known.verify("secret.lan", "ssh-rsa", RSA), Verdict::Unknown);
}

#[test]
fn revoked_keys_and_cert_authorities() {
    let text = format!(
        "@revoked example.com ssh-ed25519 {ED25519}\n\
         @cert-authority example.com ssh-rsa {RSA}\n\
         example.com ssh-ed25519 {ED25519}\n"
    );
    let known = KnownHosts::parse(&text);
    assert_eq!(known.verify("example.com", "ssh-ed25519", ED25519), Verdict::Revoked);
    // A CA key is not the host's own key.
    assert_eq!(known.verify("example.com", "ssh-rsa", RSA), Verdict::Unknown);
    assert_eq!(known.key_types_for("example.com"), vec!["ssh-ed25519".to_owned()]);
}

#[test]
fn method_prefs_put_recorded_types_first() {
    assert_eq!(
        host_key_method_prefs(&["ssh-rsa".to_owned()]),
        "rsa-sha2-512,rsa-sha2-256,ssh-rsa,ssh-ed25519,ecdsa-sha2-nistp256,\
         ecdsa-sha2-nistp384,ecdsa-sha2-nistp521,ssh-dss"
    );
    assert_eq!(
        host_key_method_prefs(&[]),
        "ssh-ed25519,ecdsa-sha2-nistp256,ecdsa-sha2-nistp384,ecdsa-sha2-nistp521,\
         rsa-sha2-512,rsa-sha2-256,ssh-rsa,ssh-dss"
    );
}

#[test]
fn append_keeps_the_existing_file_and_adds_one_line() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ssh").join("known_hosts");
    append_entry(&path, "first", "ssh-ed25519", ED25519).unwrap();
    fs::write(&path, format!("{}# kept\n@cert-authority *.lan ssh-rsa {RSA}", fs::read_to_string(&path).unwrap()))
        .unwrap();
    append_entry(&path, "[second]:2222", "ssh-rsa", RSA).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    assert_eq!(
        text,
        format!(
            "first ssh-ed25519 {ED25519}\n# kept\n@cert-authority *.lan ssh-rsa {RSA}\n\
             [second]:2222 ssh-rsa {RSA}\n"
        )
    );
    let known = KnownHosts::load(&[&path, Path::new("/nonexistent/known_hosts")]).unwrap();
    assert_eq!(known.verify("[second]:2222", "ssh-rsa", RSA), Verdict::Known);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(fs::metadata(path.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700);
    }
}
