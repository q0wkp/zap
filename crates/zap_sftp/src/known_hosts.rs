//! Host key verification against OpenSSH's known_hosts files, the way `ssh` does it with
//! `StrictHostKeyChecking=accept-new`: a host seen for the first time is recorded, and a host
//! whose key of the same type has changed is refused.
//!
//! libssh2's own check (`KnownHosts::check_port`) compares the server's key with every entry for
//! the host whatever its type, so a host recorded with an RSA key that now offers ed25519 would
//! read as changed. The files are therefore parsed here, hashed host names included.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use sha1::Sha1;

/// libssh2's host key algorithms, in its default order of preference.
const DEFAULT_HOST_KEY_METHODS: &[&str] = &[
    "ssh-ed25519",
    "ecdsa-sha2-nistp256",
    "ecdsa-sha2-nistp384",
    "ecdsa-sha2-nistp521",
    "rsa-sha2-512",
    "rsa-sha2-256",
    "ssh-rsa",
    "ssh-dss",
];

/// What a known_hosts file says about the key a server presented.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// An entry for the host has this very key.
    Known,
    /// No entry for the host has a key of this type.
    Unknown,
    /// An entry for the host has a different key of the same type.
    Changed,
    /// The key is marked `@revoked`.
    Revoked,
}

#[derive(Debug, PartialEq, Eq)]
enum Marker {
    None,
    Revoked,
    CertAuthority,
}

#[derive(Debug)]
enum HostField {
    /// `|1|<salt>|<hmac-sha1 of the name>`.
    Hashed { salt: Vec<u8>, hash: Vec<u8> },
    /// Comma-separated patterns, each possibly negated with `!`.
    Patterns(String),
}

#[derive(Debug)]
struct Entry {
    marker: Marker,
    hosts: HostField,
    key_type: String,
    key: String,
}

/// The entries of one or more known_hosts files.
#[derive(Debug, Default)]
pub struct KnownHosts {
    entries: Vec<Entry>,
}

/// The name a host is recorded under: bare on port 22, `[host]:port` otherwise.
pub fn host_entry_name(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_owned()
    } else {
        format!("[{host}]:{port}")
    }
}

/// The user's known_hosts file, which a newly seen host is added to.
pub fn user_known_hosts_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".ssh").join("known_hosts"))
}

/// The system-wide file that `ssh` also reads.
pub const GLOBAL_KNOWN_HOSTS_PATH: &str = "/etc/ssh/ssh_known_hosts";

impl KnownHosts {
    /// Parses the files that exist among `paths`; a missing file adds nothing.
    pub fn load(paths: &[&Path]) -> io::Result<Self> {
        let mut known = KnownHosts::default();
        for path in paths {
            match fs::read_to_string(path) {
                Ok(text) => known.add_text(&text),
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(err) => return Err(err),
            }
        }
        Ok(known)
    }

    #[cfg(test)]
    pub fn parse(text: &str) -> Self {
        let mut known = KnownHosts::default();
        known.add_text(text);
        known
    }

    fn add_text(&mut self, text: &str) {
        self.entries.extend(text.lines().filter_map(parse_line));
    }

    fn entries_for<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Entry> + 'a {
        self.entries
            .iter()
            .filter(move |entry| entry.marker != Marker::CertAuthority && entry.hosts.matches(name))
    }

    /// The key types recorded for `name`, to be offered first in the key exchange so that the
    /// server presents a key that can be checked.
    pub fn key_types_for(&self, name: &str) -> Vec<String> {
        let mut types: Vec<String> = Vec::new();
        for entry in self.entries_for(name).filter(|entry| entry.marker == Marker::None) {
            if !types.contains(&entry.key_type) {
                types.push(entry.key_type.clone());
            }
        }
        types
    }

    /// Checks the key `name` presented, given as its type name and base64 encoding.
    pub fn verify(&self, name: &str, key_type: &str, key: &str) -> Verdict {
        let mut changed = false;
        for entry in self.entries_for(name) {
            if entry.key_type != key_type {
                continue;
            }
            match entry.marker {
                Marker::Revoked if entry.key == key => return Verdict::Revoked,
                Marker::Revoked | Marker::CertAuthority => {}
                Marker::None if entry.key == key => return Verdict::Known,
                Marker::None => changed = true,
            }
        }
        if changed {
            Verdict::Changed
        } else {
            Verdict::Unknown
        }
    }
}

/// Host key algorithm preferences for the key exchange: the types recorded for the host first,
/// then libssh2's defaults.
pub fn host_key_method_prefs(known_types: &[String]) -> String {
    let mut prefs: Vec<&str> = Vec::new();
    for key_type in known_types {
        let methods: &[&str] = match key_type.as_str() {
            // An RSA key is also used with the SHA-2 signature algorithms.
            "ssh-rsa" => &["rsa-sha2-512", "rsa-sha2-256", "ssh-rsa"],
            other => match DEFAULT_HOST_KEY_METHODS.iter().find(|method| **method == other) {
                Some(method) => std::slice::from_ref(method),
                None => &[],
            },
        };
        for method in methods {
            if !prefs.contains(method) {
                prefs.push(method);
            }
        }
    }
    for method in DEFAULT_HOST_KEY_METHODS {
        if !prefs.contains(method) {
            prefs.push(method);
        }
    }
    prefs.join(",")
}

/// Appends one entry to `path`, creating the file (and its directory) private to the user. The
/// rest of the file is left as it is.
pub fn append_entry(path: &Path, name: &str, key_type: &str, key: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
            set_mode(dir, 0o700)?;
        }
    }
    let needs_newline = match fs::read(path) {
        Ok(bytes) => bytes.last().is_some_and(|last| *last != b'\n'),
        Err(err) if err.kind() == io::ErrorKind::NotFound => false,
        Err(err) => return Err(err),
    };
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    let separator = if needs_newline { "\n" } else { "" };
    file.write_all(format!("{separator}{name} {key_type} {key}\n").as_bytes())
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> io::Result<()> {
    Ok(())
}

fn parse_line(line: &str) -> Option<Entry> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut fields = line.split_whitespace();
    let mut first = fields.next()?;
    let marker = match first {
        "@revoked" => Marker::Revoked,
        "@cert-authority" => Marker::CertAuthority,
        other if other.starts_with('@') => return None,
        _ => Marker::None,
    };
    if marker != Marker::None {
        first = fields.next()?;
    }
    let key_type = fields.next()?.to_owned();
    let key = fields.next()?.to_owned();
    let hosts = match first.strip_prefix("|1|") {
        Some(hashed) => {
            let (salt, hash) = hashed.split_once('|')?;
            HostField::Hashed {
                salt: STANDARD.decode(salt).ok()?,
                hash: STANDARD.decode(hash).ok()?,
            }
        }
        None => HostField::Patterns(first.to_owned()),
    };
    Some(Entry {
        marker,
        hosts,
        key_type,
        key,
    })
}

impl HostField {
    fn matches(&self, name: &str) -> bool {
        match self {
            HostField::Hashed { salt, hash } => {
                let Ok(mut mac) = Hmac::<Sha1>::new_from_slice(salt) else {
                    return false;
                };
                mac.update(name.as_bytes());
                mac.verify_slice(hash).is_ok()
            }
            HostField::Patterns(patterns) => {
                let name = name.to_ascii_lowercase();
                let mut matched = false;
                for pattern in patterns.split(',') {
                    let (negated, pattern) = match pattern.strip_prefix('!') {
                        Some(pattern) => (true, pattern),
                        None => (false, pattern),
                    };
                    if wildcard_match(pattern.to_ascii_lowercase().as_bytes(), name.as_bytes()) {
                        if negated {
                            return false;
                        }
                        matched = true;
                    }
                }
                matched
            }
        }
    }
}

/// `*` and `?` matching, as in OpenSSH's `match_pattern`.
fn wildcard_match(pattern: &[u8], text: &[u8]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some((b'*', rest)) => (0..=text.len()).any(|skip| wildcard_match(rest, &text[skip..])),
        Some((b'?', rest)) => !text.is_empty() && wildcard_match(rest, &text[1..]),
        Some((c, rest)) => text.first() == Some(c) && wildcard_match(rest, &text[1..]),
    }
}

#[cfg(test)]
#[path = "known_hosts_tests.rs"]
mod tests;
