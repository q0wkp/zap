//! Prompt templates the user keeps in ZapNG's config directory.
//!
//! A profile can point a prompt slot at a file in [`custom_prompts_dir`] (`prompts/` in the
//! home config directory, next to `skills/`). The file is a minijinja template rendered in a
//! copy of the built-in environment, so it can include the built-in templates
//! (`{% include "partials/env.j2" %}`) and, under the `custom/` prefix, other custom files
//! (`{% include "custom/common.j2" %}`). Files are read from disk on every render, so an edit
//! applies to the next request without a restart.
//!
//! Paths stay inside the directory: absolute paths, `..` and symlinks that lead elsewhere are
//! refused, for the file a profile names as well as for everything it includes.

use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context as _};
use minijinja::{Environment, Error, ErrorKind, Value};

/// The name prefix under which custom files are visible to the template environment.
const CUSTOM_TEMPLATE_PREFIX: &str = "custom/";

/// How deep [`list`] descends into subdirectories.
const MAX_LIST_DEPTH: usize = 4;

/// The directory the custom prompt files live in, `None` without a home directory.
pub fn custom_prompts_dir() -> Option<PathBuf> {
    zap_ng_core::paths::zap_ng_home_config_dir().map(|dir| dir.join("prompts"))
}

/// Resolves `relative` to a file inside `dir`, refusing any path that leaves it.
fn resolve(dir: &Path, relative: &str) -> anyhow::Result<PathBuf> {
    let is_plain = !relative.is_empty()
        && Path::new(relative)
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if !is_plain {
        bail!("{relative:?} is not a relative path inside the custom prompts directory");
    }
    let dir = dir
        .canonicalize()
        .with_context(|| format!("can't open {}", dir.display()))?;
    let path = dir
        .join(relative)
        .canonicalize()
        .with_context(|| format!("can't open {relative:?} in {}", dir.display()))?;
    if !path.starts_with(&dir) {
        bail!("{relative:?} leads outside {}", dir.display());
    }
    Ok(path)
}

fn read(dir: &Path, relative: &str) -> anyhow::Result<String> {
    let path = resolve(dir, relative)?;
    std::fs::read_to_string(&path).with_context(|| format!("can't read {}", path.display()))
}

/// Renders the custom template `relative` from `dir` with `ctx`, in a copy of `builtins`.
pub(super) fn render(
    builtins: &Environment<'static>,
    dir: &Path,
    relative: &str,
    ctx: Value,
) -> anyhow::Result<String> {
    let mut env = builtins.clone();
    let loader_dir = dir.to_owned();
    env.set_loader(move |name| {
        let Some(relative) = name.strip_prefix(CUSTOM_TEMPLATE_PREFIX) else {
            return Ok(None);
        };
        read(&loader_dir, relative)
            .map(Some)
            .map_err(|e| Error::new(ErrorKind::TemplateNotFound, format!("{e:#}")))
    });
    let template = env.get_template(&format!("{CUSTOM_TEMPLATE_PREFIX}{relative}"))?;
    Ok(template.render(ctx)?)
}

/// The files in `dir` a profile can choose, as paths relative to it with `/` separators,
/// sorted. Hidden files and directories are left out; a missing directory lists nothing.
pub fn list(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    collect(dir, "", 0, &mut files);
    files.sort();
    files
}

fn collect(dir: &Path, prefix: &str, depth: usize, files: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        // Follows symlinks, so a linked file or directory is listed like any other.
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        let relative = format!("{prefix}{name}");
        if metadata.is_dir() {
            if depth < MAX_LIST_DEPTH {
                collect(&path, &format!("{relative}/"), depth + 1, files);
            }
        } else if metadata.is_file() {
            files.push(relative);
        }
    }
}

#[cfg(test)]
#[path = "custom_prompts_tests.rs"]
mod tests;
