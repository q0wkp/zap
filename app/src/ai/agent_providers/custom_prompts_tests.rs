use std::path::Path;

use minijinja::{context, Environment, Value};

use super::*;

fn builtins() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template("partials/greeting.j2", "Hello {{ name }}")
        .unwrap();
    env
}

fn write(dir: &Path, relative: &str, content: &str) {
    let path = dir.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn render_in(dir: &Path, relative: &str, ctx: Value) -> anyhow::Result<String> {
    render(&builtins(), dir, relative, ctx)
}

#[test]
fn renders_with_context_and_builtin_partials() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "agent.j2",
        "{% include \"partials/greeting.j2\" %}, model {{ model_id }}",
    );

    let out = render_in(
        dir.path(),
        "agent.j2",
        context! { name => "Zap", model_id => "m1" },
    )
    .unwrap();
    assert_eq!(out, "Hello Zap, model m1");
}

#[test]
fn includes_other_custom_files_by_prefix() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "shared/rules.j2", "RULES");
    write(
        dir.path(),
        "agent.j2",
        "{% include \"custom/shared/rules.j2\" %} end",
    );

    assert_eq!(
        render_in(dir.path(), "agent.j2", context! {}).unwrap(),
        "RULES end"
    );
}

#[test]
fn reads_the_file_again_after_an_edit() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "title.md", "first");
    assert_eq!(
        render_in(dir.path(), "title.md", context! {}).unwrap(),
        "first"
    );

    write(dir.path(), "title.md", "second");
    assert_eq!(
        render_in(dir.path(), "title.md", context! {}).unwrap(),
        "second"
    );
}

#[test]
fn refuses_paths_that_leave_the_directory() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("prompts");
    write(&dir, "inside.j2", "ok");
    write(root.path(), "outside.j2", "secret");

    for relative in [
        "../outside.j2",
        "sub/../../outside.j2",
        "./inside.j2",
        "",
        root.path().join("outside.j2").to_str().unwrap(),
    ] {
        assert!(
            render_in(&dir, relative, context! {}).is_err(),
            "{relative:?} should be refused"
        );
    }
}

#[test]
fn refuses_includes_that_leave_the_directory() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("prompts");
    write(root.path(), "outside.j2", "secret");
    write(&dir, "agent.j2", "{% include \"custom/../outside.j2\" %}");

    let err = render_in(&dir, "agent.j2", context! {}).unwrap_err();
    assert!(!format!("{err:#}").contains("secret"), "{err:#}");
}

#[cfg(unix)]
#[test]
fn refuses_symlinks_that_lead_outside() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("prompts");
    std::fs::create_dir_all(&dir).unwrap();
    write(root.path(), "outside.j2", "secret");
    std::os::unix::fs::symlink(root.path().join("outside.j2"), dir.join("link.j2")).unwrap();

    assert!(render_in(&dir, "link.j2", context! {}).is_err());
}

#[test]
fn reports_missing_files_and_template_errors() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "broken.j2", "{% if unclosed %}");

    assert!(render_in(dir.path(), "missing.j2", context! {}).is_err());
    assert!(render_in(dir.path(), "broken.j2", context! {}).is_err());
    assert!(render_in(&dir.path().join("absent"), "broken.j2", context! {}).is_err());
}

#[test]
fn lists_visible_files_recursively_and_sorted() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "b.j2", "");
    write(dir.path(), "a.md", "");
    write(dir.path(), "team/agent.j2", "");
    write(dir.path(), ".hidden.j2", "");
    write(dir.path(), ".git/config", "");

    assert_eq!(
        list(dir.path()),
        vec![
            "a.md".to_owned(),
            "b.j2".to_owned(),
            "team/agent.j2".to_owned()
        ]
    );
}

#[test]
fn missing_directory_lists_nothing() {
    let dir = tempfile::tempdir().unwrap();
    assert!(list(&dir.path().join("absent")).is_empty());
}
