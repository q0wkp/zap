//! Deciding which of the plugins found are loaded, and in which order.

use std::collections::{HashMap, HashSet};

use crate::manifest::{Origin, Plugin};

/// A plugin that is not loaded, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skipped {
    pub id: String,
    pub reason: String,
}

/// Orders the plugins so that each comes after the ones providing what it requires, leaving out
/// the ones that can't be loaded.
///
/// `enabled` is what the user set for a plugin, by id; a plugin not in it follows its manifest.
/// A plugin the user put in place is refused anything beyond its own working directory: there is
/// no way yet for the user to approve such a request.
pub fn load_order(
    plugins: Vec<Plugin>,
    enabled: &HashMap<String, bool>,
) -> (Vec<Plugin>, Vec<Skipped>) {
    let mut skipped = Vec::new();
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for plugin in plugins {
        let id = plugin.manifest.id.clone();
        let reason = if !seen.insert(id.clone()) {
            // The first one found wins, so list bundled plugins before the user's.
            Some("another plugin has the same id".to_owned())
        } else if !enabled
            .get(&id)
            .copied()
            .unwrap_or(plugin.manifest.enabled_by_default)
        {
            Some("disabled".to_owned())
        } else if plugin.origin == Origin::User && !plugin.manifest.capabilities.is_empty() {
            Some(
                "asks for directories or the keychain, which only bundled plugins may have"
                    .to_owned(),
            )
        } else {
            None
        };
        match reason {
            Some(reason) => skipped.push(Skipped { id, reason }),
            None => candidates.push(plugin),
        }
    }

    // Repeatedly take the plugins whose requirements the ones already taken provide.
    let mut ordered: Vec<Plugin> = Vec::new();
    let mut provided: HashSet<String> = HashSet::new();
    loop {
        let (ready, waiting): (Vec<Plugin>, Vec<Plugin>) =
            candidates.into_iter().partition(|plugin| {
                plugin
                    .manifest
                    .requires
                    .iter()
                    .all(|name| provided.contains(name))
            });
        candidates = waiting;
        if ready.is_empty() {
            break;
        }
        for plugin in ready {
            provided.extend(plugin.manifest.provides.iter().cloned());
            ordered.push(plugin);
        }
    }
    for plugin in candidates {
        let missing: Vec<&str> = plugin
            .manifest
            .requires
            .iter()
            .filter(|name| !provided.contains(*name))
            .map(String::as_str)
            .collect();
        skipped.push(Skipped {
            id: plugin.manifest.id.clone(),
            reason: format!("no enabled plugin provides {}", missing.join(", ")),
        });
    }
    (ordered, skipped)
}

#[cfg(test)]
#[path = "load_order_tests.rs"]
mod tests;
