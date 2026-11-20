//! A settings store kept in two TOML files: the main settings file, and a second file for the
//! settings named in a routing table, which live there under paths of their own.

use std::path::PathBuf;

use super::toml_backed::TomlBackedUserPreferences;
use super::{Error, UserPreferences};

/// One routed setting: the dotted path it is defined with, and the dotted path it has in the
/// second file.
pub type Route = (&'static str, &'static str);

pub struct SplitTomlUserPreferences {
    main: TomlBackedUserPreferences,
    routed: TomlBackedUserPreferences,
    routes: &'static [Route],
}

impl SplitTomlUserPreferences {
    /// Opens both files as [`TomlBackedUserPreferences::new`] does. The error returned is the
    /// main file's parse error, or else the second file's.
    pub fn new(
        main_file_path: PathBuf,
        routed_file_path: PathBuf,
        routes: &'static [Route],
    ) -> (Self, Option<Error>) {
        let (main, main_error) = TomlBackedUserPreferences::new(main_file_path);
        let (routed, routed_error) = TomlBackedUserPreferences::new(routed_file_path);
        (
            Self {
                main,
                routed,
                routes,
            },
            main_error.or(routed_error),
        )
    }

    /// Where the setting defined at `hierarchy`.`key` is kept in the second file, if it is
    /// routed there: the hierarchy and the key within that file.
    fn route(
        &self,
        key: &str,
        hierarchy: Option<&str>,
    ) -> Option<(Option<&'static str>, &'static str)> {
        let hierarchy = hierarchy?;
        self.routes
            .iter()
            .find(|(defined, _)| {
                defined
                    .rsplit_once('.')
                    .is_some_and(|(defined_hierarchy, defined_key)| {
                        defined_hierarchy == hierarchy && defined_key == key
                    })
            })
            .map(|(_, routed)| match routed.rsplit_once('.') {
                Some((routed_hierarchy, routed_key)) => (Some(routed_hierarchy), routed_key),
                None => (None, *routed),
            })
    }
}

impl UserPreferences for SplitTomlUserPreferences {
    fn write_value(&self, key: &str, value: String) -> Result<(), Error> {
        self.main.write_value(key, value)
    }

    fn read_value(&self, key: &str) -> Result<Option<String>, Error> {
        self.main.read_value(key)
    }

    fn remove_value(&self, key: &str) -> Result<(), Error> {
        self.main.remove_value(key)
    }

    fn write_value_with_hierarchy(
        &self,
        key: &str,
        value: String,
        hierarchy: Option<&str>,
        max_table_depth: Option<u32>,
    ) -> Result<(), Error> {
        match self.route(key, hierarchy) {
            Some((routed_hierarchy, routed_key)) => self.routed.write_value_with_hierarchy(
                routed_key,
                value,
                routed_hierarchy,
                max_table_depth,
            ),
            None => self
                .main
                .write_value_with_hierarchy(key, value, hierarchy, max_table_depth),
        }
    }

    fn read_value_with_hierarchy(
        &self,
        key: &str,
        hierarchy: Option<&str>,
    ) -> Result<Option<String>, Error> {
        match self.route(key, hierarchy) {
            Some((routed_hierarchy, routed_key)) => self
                .routed
                .read_value_with_hierarchy(routed_key, routed_hierarchy),
            None => self.main.read_value_with_hierarchy(key, hierarchy),
        }
    }

    fn remove_value_with_hierarchy(&self, key: &str, hierarchy: Option<&str>) -> Result<(), Error> {
        match self.route(key, hierarchy) {
            Some((routed_hierarchy, routed_key)) => self
                .routed
                .remove_value_with_hierarchy(routed_key, routed_hierarchy),
            None => self.main.remove_value_with_hierarchy(key, hierarchy),
        }
    }

    fn is_settings_file(&self) -> bool {
        true
    }

    fn reload_from_disk(&self) -> Result<(), Error> {
        // Reload both even when the first fails, so that one broken file doesn't hold back the
        // other's changes.
        let main_result = self.main.reload_from_disk();
        let routed_result = self.routed.reload_from_disk();
        main_result.and(routed_result)
    }

    fn inhibit_writes_for_key(&self, key: &str, hierarchy: Option<&str>) {
        match self.route(key, hierarchy) {
            Some((routed_hierarchy, routed_key)) => self
                .routed
                .inhibit_writes_for_key(routed_key, routed_hierarchy),
            None => self.main.inhibit_writes_for_key(key, hierarchy),
        }
    }

    fn clear_all_write_inhibitions(&self) {
        self.main.clear_all_write_inhibitions();
        self.routed.clear_all_write_inhibitions();
    }
}

#[cfg(test)]
#[path = "split_toml_tests.rs"]
mod tests;
