//! This module contains utilities for dealing with file/directory paths throughout Zap.
use std::borrow::Cow;
use std::collections::HashMap;
use std::env::{self, VarError};
use std::hash::Hash;
use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::standardized_path::StandardizedPath;

lazy_static! {
    /// Test home directory value for tests.
    pub static ref TEST_SESSION_HOME_DIR: Option<String> =
        dirs::home_dir().and_then(|home_buf| home_buf.to_str().map(|s| s.to_owned()));

    /// Special characters to escape in POSIX-based shells. Check for the full list here:
    /// https://mywiki.wooledge.org/BashGuide/SpecialCharacters
    static ref POSIX_SHELL_ESCAPE_PATTERN: Regex =
        Regex::new(r#"([ "\$'\\#=\[\]!><|;{}()\*\?&`~]|\n|\t)"#).expect("Shell escape regex should be valid");

    /// Special characters to escape in PowerShell. Mostly the same as [`POSIX_SHELL_ESCAPE_PATTERN`]
    /// but with the following differences:
    ///
    /// Omitted:
    /// * `\` - Backslashes are not escape characters in PowerShell.
    /// * `?` - In certain positions, `?` is the ternary operator. However, it is usually plain
    /// text. Actually "?" is a built-in alias for `Where-Object`.
    /// * `~` - Tilde is treated differently in PowerShell. It _cannot_ be tilde-escaped to avoid
    /// exansion. It has to be quoted to suppress conversion to the HOME dir.
    ///
    /// Added:
    /// * `@` - The `@` sigil creates array and object literals.
    /// * `,` - This separates array elements, and its presence causes an expression to become an
    /// array.
    static ref POWERSHELL_SHELL_ESCAPE_PATTERN: Regex =
        Regex::new(r#"([ "\$'#=\[\]!><|;{}()\*&`@,]|\n|\t)"#).expect("Shell escape regex should be valid");

    /// Regex for valid line and column number formats.
    static ref LINE_AND_COLUMN_REGEX: Vec<Regex> = vec![
        Regex::new(":(\\d+)").expect("Regex is valid"), // e.g. ":100".
        Regex::new(":(\\d+)-(?:\\d+)").expect("Regex is valid"), // e.g. ":100-200".
        Regex::new(":(\\d+):(\\d+)").expect("Regex is valid"), // e.g. ":100:300".
        Regex::new("\\[(\\d+), ?(\\d+)]").expect("Regex is valid"), // e.g. "[100, 300]".
        Regex::new("\", line (\\d+), column (\\d+)").expect("Regex is valid"), // e.g. `", line 100, column 300`.
        Regex::new("\", line (\\d+), in").expect("Regex is valid"), // e.g. `", line 100, in`.
        Regex::new("\\((\\d+), ?(\\d+)\\)").expect("Regex is valid"), // e.g. "(100, 300)".
        Regex::new("#L(\\d+)").expect("Regex is valid"), // e.g. "#L100".
        Regex::new("#L(\\d+):(\\d+)").expect("Regex is valid"), // e.g. "#L100:300"
    ];
}

/// Leading prefix for a path to the home directory using the $HOME environment variable.
pub const HOME_DIR_ENV_VAR_PREFIX: &str = "$HOME";

/// \return any override shell launch path, reading from the WARP_SHELL_PATH variable.
pub fn warp_shell_path() -> Option<String> {
    // TODO(peter): we ought to tolerate non-Unicode paths here.
    env::var("WARP_SHELL_PATH").ok()
}

/// Abbreviates the session home directory in the given path to '~', if it is in the given path,
/// otherwise returns the path unchanged.
pub fn user_friendly_path<'a>(path: &'a str, home_dir: Option<&str>) -> Cow<'a, str> {
    home_dir
        .and_then(|home| {
            if path.starts_with(home) {
                let user_friendly_path = match path.strip_prefix(home) {
                    Some("") => Cow::Owned(String::from("~")),
                    Some(path_without_home) => {
                        let next_char = path_without_home
                            .chars()
                            .next()
                            .expect("already verified `path_without_home` not empty");
                        if cfg!(unix) && next_char == '/' {
                            Cow::Owned("~".to_owned() + path_without_home)
                        } else {
                            Cow::Borrowed(path)
                        }
                    }
                    None => Cow::Borrowed(path),
                };
                Some(user_friendly_path)
            } else {
                None
            }
        })
        .unwrap_or(Cow::Borrowed(path))
}

/// Result after parsing a path string that mixes path and line and column numbers
/// into each individual components.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanPathResult {
    pub path: String,
    pub line_and_column_num: Option<LineAndColumnArg>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineAndColumnArg {
    // line number must exist for the LineAndColumnArg.
    pub line_num: usize,
    pub column_num: Option<usize>,
}

impl LineAndColumnArg {
    pub fn to_string_suffix(&self) -> String {
        match self {
            LineAndColumnArg {
                line_num,
                column_num: Some(column_num),
            } => {
                format!(":{line_num}:{column_num}")
            }
            LineAndColumnArg {
                line_num,
                column_num: None,
            } => {
                format!(":{line_num}")
            }
        }
    }
}

impl CleanPathResult {
    /// Given a path string that contains a mix of path, line and column numbers,
    /// parse it into each individual component if the format is supported. Note
    /// that we only break it down when the whole string, rather than only part of
    /// the string, matches the format.
    pub fn with_line_and_column_number(path: &str) -> Self {
        let mut line_num = None;
        let mut column_num = None;
        let mut cleaned_path = path;
        for rg in LINE_AND_COLUMN_REGEX.iter() {
            match rg.captures(path) {
                // Need to match the entire running string rather than just part of it.
                Some(captured)
                    if captured.get(0).expect("First group always exists").end() == path.len() =>
                {
                    line_num = captured.get(1).and_then(|m| m.as_str().parse().ok());
                    column_num = captured.get(2).and_then(|m| m.as_str().parse().ok());
                    cleaned_path =
                        &path[..captured.get(0).expect("First group always exists").start()];
                }
                _ => (),
            }
        }

        Self {
            path: cleaned_path.to_owned(),
            line_and_column_num: line_num.map(|line_num| LineAndColumnArg {
                line_num,
                column_num,
            }),
        }
    }
}

/// Which character is used to escape, e.g. "\n"?
#[derive(Clone, Copy, Debug)]
pub enum EscapeChar {
    Backslash,
    Backtick,
}

impl EscapeChar {
    pub fn is_char(&self, c: char) -> bool {
        match self {
            Self::Backslash => c == '\\',
            Self::Backtick => c == '`',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Grouping of shells with related escaping behavior.
pub enum ShellFamily {
    /// Bash, Zsh, and Fish
    Posix,
    PowerShell,
}

impl ShellFamily {
    pub fn escape_char(&self) -> EscapeChar {
        match self {
            Self::Posix => EscapeChar::Backslash,
            Self::PowerShell => EscapeChar::Backtick,
        }
    }

    /// Escapes an input string so they will retain its meaning in a no-quote representation. This
    /// is done by prepending the escape character to special/meta characters like *, |, $, etc.
    pub fn escape<'s>(&self, input: &'s str) -> Cow<'s, str> {
        if input.is_empty() {
            return "''".into();
        }
        match self {
            Self::Posix => POSIX_SHELL_ESCAPE_PATTERN.replace_all(input, "\\$1"),
            Self::PowerShell => POWERSHELL_SHELL_ESCAPE_PATTERN.replace_all(input, "`$1"),
        }
    }

    /// Unescapes a shell-escaped string by removing escape characters that were prepended to
    /// special/meta characters. This is the inverse of [`Self::escape`].
    ///
    /// Returns [`Cow::Borrowed`] when the input contains no escape characters.
    pub fn unescape<'s>(&self, input: &'s str) -> Cow<'s, str> {
        let escape_char = self.escape_char();
        if !input.contains(|c| escape_char.is_char(c)) {
            return Cow::Borrowed(input);
        }

        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars();
        while let Some(c) = chars.next() {
            if escape_char.is_char(c) {
                match chars.next() {
                    Some(next) => result.push(next),
                    // Trailing escape char with nothing after it; keep as-is.
                    None => result.push(c),
                }
            } else {
                result.push(c);
            }
        }
        Cow::Owned(result)
    }

    /// Escapes the path to treat it as a single word within the shell.
    ///
    /// This function returns a [`Cow::Borrowed`] of the input string where possible and only
    /// returns owned data when the escaped version differs from the input string.
    pub fn shell_escape<'s>(&self, path: &'s str) -> Cow<'s, str> {
        // Special case if the path starts with "~/" or "~\": The escape function escapes the "~" to avoid
        // tilde expansion, but we still want tilde expansion with the rest of the path properly
        // escaped.
        for prefix in ["~", HOME_DIR_ENV_VAR_PREFIX] {
            if let Some(suffix) = path.strip_prefix(prefix) {
                if suffix.is_empty() {
                    return prefix.into();
                }
                let first_char = suffix.chars().next().expect("length already validated");
                return if first_char != '/' && first_char != '\\' {
                    self.escape(path)
                } else {
                    let escaped_sufix = self.escape(suffix);
                    // If there was no escaping to do, we can return the original path.
                    if matches!(escaped_sufix, Cow::Borrowed(_)) {
                        path.into()
                    } else {
                        Cow::Owned(format!("{prefix}{escaped_sufix}"))
                    }
                };
            }
        }
        self.escape(path)
    }
}

/// Returns `true` iff the given string is a valid POSIX portable pathname.
/// Source: https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap03.html#tag_03_271
pub fn is_posix_portable_pathname(s: &str) -> bool {
    s.split('/').all(|filename| {
        filename
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    })
}

#[derive(Error, Debug)]
pub enum TargetDirError {
    #[error("Could not retrieve the manifest directory: {0}")]
    CouldNotRetrieveManifestDir(#[from] VarError),
    #[error("No parent was found for the manifest directory")]
    NoManifestDirParent,
}

/// Retrieves the target directory.
pub fn app_target_dir(profile: &str) -> Result<PathBuf, TargetDirError> {
    // TODO(CORE-2805): Make sure this works in distribution.
    // Ideally we would use `CARGO_TARGET_DIR` but this isn't always available.
    // See https://github.com/rust-lang/cargo/issues/9661.
    let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
    let manifest_dir = Path::new(&manifest_dir);
    let Some(workspace_dir) = manifest_dir.parent().and_then(Path::parent) else {
        return Err(TargetDirError::NoManifestDirParent);
    };
    Ok(Path::new(workspace_dir).join("target").join(profile))
}

/// Converts an absolute path to a relative path from the given current working directory.
/// This function properly handles leading slashes and returns a clean relative path.
///
/// # Arguments
/// * `absolute_path` - The absolute path to convert
/// * `cwd` - The current working directory to make the path relative to
///
/// # Returns
/// * `Some(String)` - The relative path as a string, guaranteed to not have leading slashes
/// * `None` - If the paths cannot be made relative
///
/// # Examples
/// ```
/// use std::path::Path;
/// use warp_util::path::to_relative_path;
///
/// let abs_path = Path::new("/Users/john/projects/app/src/main.rs");
/// let cwd = Path::new("/Users/john/projects");
/// assert_eq!(to_relative_path(abs_path, cwd), Some("app/src/main.rs".to_string()));
/// ```
pub fn to_relative_path(absolute_path: &Path, cwd: &Path) -> Option<String> {
    pathdiff::diff_paths(absolute_path, cwd).map(|relative_path| {
        let path_str = relative_path.to_string_lossy();
        // Remove any leading slashes or current directory references
        let cleaned = path_str
            .strip_prefix("./")
            .or_else(|| path_str.strip_prefix("/"))
            .unwrap_or(&path_str);

        if cleaned.is_empty() || cleaned == "." {
            ".".to_string()
        } else {
            cleaned.to_string()
        }
    })
}

/// Converts a workspace-relative path into a normalized string for matching against glob patterns.
///
/// This joins path components with forward slashes (`/`) so the resulting string is comparable
/// across platforms (especially Windows).
///
/// Note: This drops any non-normal components (e.g. `.` and `..`).
pub fn normalize_relative_path_for_glob(path: &Path) -> String {
    let mut normalized = String::new();

    for component in path.components() {
        let std::path::Component::Normal(component) = component else {
            continue;
        };

        if !normalized.is_empty() {
            normalized.push('/');
        }

        normalized.push_str(&component.to_string_lossy());
    }

    normalized
}

/// Finds the common prefix path between some number of paths.
/// Returns `Some(PathBuf)` containing the common prefix, otherwise `None`.
///
/// # Examples
/// ```
/// use std::path::Path;
/// use warp_util::path::common_path;
///
/// let paths = [Path::new("/foo/bar/baz"), Path::new("/foo/bar/quux"), Path::new("/foo/bar/quuux")];
/// assert_eq!(common_path(paths), Some(Path::new("/foo/bar").to_path_buf()));
/// ```
pub fn common_path<P>(paths: impl IntoIterator<Item = P>) -> Option<PathBuf>
where
    P: AsRef<Path>,
{
    let paths: Vec<_> = paths.into_iter().collect();

    let mut common = paths.first()?.as_ref().to_path_buf();
    for p in paths.iter().skip(1) {
        common = common
            .components()
            .zip(p.as_ref().components())
            .take_while(|(l, r)| l == r)
            .map(|(l, _)| l.as_os_str())
            .collect::<PathBuf>();

        // Returns None if the common path is empty between any two paths
        if common.as_os_str().is_empty() {
            return None;
        }
    }

    Some(common)
}

/// Trait for path-like values that can participate in ancestor-aware
/// grouping. Implemented for [`PathBuf`] (component-aware matching via
/// [`Path::starts_with`]) and [`StandardizedPath`].
pub trait RootPath: Sized + Clone + Eq + Hash {
    /// Returns `true` if `self` is a path-prefix of `other` at component
    /// boundaries. Equal paths return `true`.
    fn is_prefix_of(&self, other: &Self) -> bool;

    /// Returns the number of path components in this path. Used only to
    /// order paths by length so potential ancestors are examined before
    /// their descendants.
    fn component_count(&self) -> usize;
}

impl RootPath for PathBuf {
    fn is_prefix_of(&self, other: &Self) -> bool {
        other.starts_with(self)
    }

    fn component_count(&self) -> usize {
        self.components().count()
    }
}

impl RootPath for StandardizedPath {
    fn is_prefix_of(&self, other: &Self) -> bool {
        other.starts_with(self)
    }

    fn component_count(&self) -> usize {
        self.as_typed_path().components().count()
    }
}

/// Result of grouping a set of root paths by ancestor/descendant
/// relationship. See [`group_roots_by_common_ancestor`].
#[derive(Debug, Clone)]
pub struct RootGrouping<P> {
    /// Ancestor-deduped set of roots. The input order of surviving
    /// entries is preserved.
    pub roots: Vec<P>,
    /// For each surviving root, the input paths that were absorbed
    /// because they were (non-strict) descendants of that root. Keyed
    /// by the closest surviving ancestor. Absorbed paths are recorded
    /// in input order.
    pub absorbed_by_root: HashMap<P, Vec<P>>,
}

/// Returns the ancestor-deduped set of `roots`. If any input path has an
/// ancestor already present in the set, it is dropped from `roots` and
/// recorded in `absorbed_by_root` under its closest surviving ancestor.
///
/// Exact duplicates in the input are collapsed to a single surviving
/// entry with no absorbed list (they are not treated as ancestors of
/// "themselves").
///
/// Ordering: `roots` preserves the input order for surviving entries, and
/// each `absorbed_by_root[ancestor]` preserves the input order of
/// absorbed descendants.
///
/// Component-aware matching is used, so `/a` is not treated as an
/// ancestor of `/ab`.
///
/// # Examples
/// ```
/// use std::path::PathBuf;
/// use warp_util::path::group_roots_by_common_ancestor;
///
/// let grouping = group_roots_by_common_ancestor(&[
///     PathBuf::from("/code/a/z"),
///     PathBuf::from("/code/a"),
///     PathBuf::from("/code"),
/// ]);
/// assert_eq!(grouping.roots, vec![PathBuf::from("/code")]);
/// assert_eq!(
///     grouping.absorbed_by_root[&PathBuf::from("/code")],
///     vec![PathBuf::from("/code/a/z"), PathBuf::from("/code/a")],
/// );
/// ```
pub fn group_roots_by_common_ancestor<P: RootPath>(roots: &[P]) -> RootGrouping<P> {
    if roots.is_empty() {
        return RootGrouping {
            roots: Vec::new(),
            absorbed_by_root: HashMap::new(),
        };
    }

    // Phase 1: Drop exact duplicates while preserving input order.
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<P> = roots
        .iter()
        .filter(|p| seen.insert((*p).clone()))
        .cloned()
        .collect();

    // Phase 2: Sort by component count ascending (stable) so that any
    // potential ancestor is processed before its descendants. For each
    // path, either accept it as a survivor or record which already-
    // accepted ancestor absorbs it.
    let mut sorted: Vec<(usize, P)> = deduped.iter().cloned().enumerate().collect();
    sorted.sort_by_key(|(_, p)| p.component_count());

    let mut accepted: Vec<P> = Vec::new();
    // Index in `deduped` -> closest surviving ancestor, if absorbed.
    let mut absorbed_ancestor_by_index: HashMap<usize, P> = HashMap::new();
    for (idx, path) in &sorted {
        let closest = accepted
            .iter()
            .filter(|s| s.is_prefix_of(path))
            .max_by_key(|s| s.component_count())
            .cloned();
        match closest {
            Some(ancestor) => {
                absorbed_ancestor_by_index.insert(*idx, ancestor);
            }
            None => {
                accepted.push(path.clone());
            }
        }
    }

    // Phase 3: Walk `deduped` in input order to produce the final
    // ordered `roots` vector and the input-ordered absorbed lists.
    let mut out_roots: Vec<P> = Vec::new();
    let mut absorbed_by_root: HashMap<P, Vec<P>> = HashMap::new();
    for (idx, path) in deduped.iter().enumerate() {
        match absorbed_ancestor_by_index.get(&idx) {
            Some(ancestor) => {
                absorbed_by_root
                    .entry(ancestor.clone())
                    .or_default()
                    .push(path.clone());
            }
            None => {
                out_roots.push(path.clone());
            }
        }
    }

    RootGrouping {
        roots: out_roots,
        absorbed_by_root,
    }
}

#[cfg(test)]
#[path = "path_test.rs"]
mod tests;
