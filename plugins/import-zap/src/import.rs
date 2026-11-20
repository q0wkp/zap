// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::Path;

use serde::Serialize;
use toml_edit::{DocumentMut, Item, Table};

/// Zap's directories and ZapNG's directory for the same, relative to the home directory. The
/// first is in the home directory on every system, the next three are Linux's, the last macOS's.
pub const DIRECTORIES: &[(&str, &str)] = &[
    (".zap", ".zap-ng"),
    (".config/zap", ".config/zap-ng"),
    (".local/share/zap", ".local/share/zap-ng"),
    (".local/state/zap", ".local/state/zap-ng"),
    (
        "Library/Application Support/dev.zap.Zap",
        "Library/Application Support/dev.q0wkp.zapng",
    ),
];

/// Files and directories that belong to a running Zap or to one machine's past, not to the
/// user's configuration.
fn is_left_behind(name: &str) -> bool {
    matches!(name, "logs" | "remote-server" | "plugins")
        || [".log", ".sock", ".lock", ".pid", ".dmp"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
        || name.contains(".log.")
}

#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Report {
    /// Files copied.
    pub files: u64,
    /// Settings moved from `settings.toml` to `zap-ng.toml`.
    pub settings_moved: u64,
    /// Secrets copied in the keychain.
    pub secrets: u64,
    /// What could not be done. The rest of the import still went through.
    pub errors: Vec<String>,
}

/// The directories of a Zap install that exist under `home`.
pub fn detect(home: &Path) -> Vec<&'static str> {
    DIRECTORIES
        .iter()
        .map(|(zap, _)| *zap)
        .filter(|zap| home.join(zap).is_dir())
        .collect()
}

/// Copies Zap's directories under `home` to ZapNG's, then splits each copied `settings.toml`.
pub fn import_files(home: &Path, settings_routes: &[(String, String)]) -> Report {
    let mut report = Report::default();
    for (zap, zap_ng) in DIRECTORIES {
        let from = home.join(zap);
        if !from.is_dir() {
            continue;
        }
        let to = home.join(zap_ng);
        copy_dir(&from, &to, &mut report);
        let settings = to.join("settings.toml");
        if settings.is_file() {
            match split_settings_file(&settings, &to.join("zap-ng.toml"), settings_routes) {
                Ok(moved) => report.settings_moved += moved,
                Err(error) => report
                    .errors
                    .push(format!("{}: {error}", settings.display())),
            }
        }
    }
    report
}

fn copy_dir(from: &Path, to: &Path, report: &mut Report) {
    if let Err(error) = std::fs::create_dir_all(to) {
        report.errors.push(format!("{}: {error}", to.display()));
        return;
    }
    let entries = match std::fs::read_dir(from) {
        Ok(entries) => entries,
        Err(error) => {
            report.errors.push(format!("{}: {error}", from.display()));
            return;
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if is_left_behind(&name.to_string_lossy()) {
            continue;
        }
        let (source, target) = (entry.path(), to.join(&name));
        // Links are not followed: what they point at may lie outside what was granted.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            copy_dir(&source, &target, report);
        } else if file_type.is_file() {
            match std::fs::copy(&source, &target) {
                Ok(_) => report.files += 1,
                Err(error) => report.errors.push(format!("{}: {error}", source.display())),
            }
        }
    }
}

fn split_settings_file(
    settings: &Path,
    zap_ng: &Path,
    routes: &[(String, String)],
) -> Result<u64, String> {
    let settings_text = std::fs::read_to_string(settings).map_err(|error| error.to_string())?;
    let zap_ng_text = match std::fs::read_to_string(zap_ng) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.to_string()),
    };
    let (settings_text, zap_ng_text, moved) = split_settings(&settings_text, &zap_ng_text, routes)?;
    if moved > 0 {
        std::fs::write(zap_ng, zap_ng_text).map_err(|error| error.to_string())?;
        std::fs::write(settings, settings_text).map_err(|error| error.to_string())?;
    }
    Ok(moved)
}

/// Moves the routed settings out of the text of a `settings.toml` into the text of a
/// `zap-ng.toml`, each to its path there, and returns both texts and how many were moved.
/// Tables the move leaves empty are removed; everything else of `settings.toml` is kept as it is.
pub fn split_settings(
    settings: &str,
    zap_ng: &str,
    routes: &[(String, String)],
) -> Result<(String, String, u64), String> {
    let mut settings: DocumentMut = settings.parse().map_err(|error| format!("{error}"))?;
    let mut zap_ng: DocumentMut = zap_ng.parse().map_err(|error| format!("{error}"))?;
    let mut moved = 0;
    for (from, to) in routes {
        let from: Vec<&str> = from.split('.').collect();
        let Some(value) = take(settings.as_table_mut(), &from) else {
            continue;
        };
        let to: Vec<&str> = to.split('.').collect();
        put(zap_ng.as_table_mut(), &to, value)?;
        moved += 1;
    }
    let mut settings = settings.to_string();
    if moved > 0 {
        // A table removed from the top leaves its blank line behind.
        settings = settings.trim_start().to_owned();
    }
    Ok((settings, zap_ng.to_string(), moved))
}

/// Removes the item at `path`, and the tables this leaves empty.
fn take(table: &mut Table, path: &[&str]) -> Option<Item> {
    let (first, rest) = path.split_first()?;
    if rest.is_empty() {
        return table.remove(first);
    }
    let child = table.get_mut(first)?.as_table_mut()?;
    let item = take(child, rest);
    if item.is_some() && child.is_empty() {
        table.remove(first);
    }
    item
}

/// Puts `item` at `path`, creating the tables on the way.
fn put(table: &mut Table, path: &[&str], item: Item) -> Result<(), String> {
    let Some((first, rest)) = path.split_first() else {
        return Err("empty path".to_owned());
    };
    if rest.is_empty() {
        table.insert(first, item);
        return Ok(());
    }
    let child = table.entry(first).or_insert_with(|| {
        let mut child = Table::new();
        // No header of its own for a table that only holds other tables.
        child.set_implicit(true);
        Item::Table(child)
    });
    match child.as_table_mut() {
        Some(child) => put(child, rest, item),
        None => Err(format!("{first} is not a table")),
    }
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
