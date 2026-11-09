use super::*;

#[test]
fn test_user_friendly_path_with_home() {
    let home = "/Users/blue";
    assert_eq!(
        user_friendly_path("/Users/blue", Some(home)),
        "~".to_string(),
    );
    assert_eq!(
        user_friendly_path("/Users/blue/warp", Some(home)),
        "~/warp".to_string(),
    );
    assert_eq!(
        user_friendly_path("/Users/admin/warp", Some(home)),
        "/Users/admin/warp".to_string(),
    );
}

#[test]
fn test_to_relative_path() {
    use super::to_relative_path;
    use std::path::Path;

    // Basic relative path conversion
    {
        assert_eq!(
            to_relative_path(
                Path::new("/Users/john/projects/app/src/main.rs"),
                Path::new("/Users/john/projects")
            ),
            Some("app/src/main.rs".to_string())
        );

        // Same directory
        assert_eq!(
            to_relative_path(
                Path::new("/Users/john/projects"),
                Path::new("/Users/john/projects")
            ),
            Some(".".to_string())
        );

        // Parent directory
        assert_eq!(
            to_relative_path(Path::new("/Users/john"), Path::new("/Users/john/projects")),
            Some("..".to_string())
        );

        // Nested parent
        assert_eq!(
            to_relative_path(Path::new("/Users"), Path::new("/Users/john/projects")),
            Some("../..".to_string())
        );

        // Cross-branch paths
        assert_eq!(
            to_relative_path(
                Path::new("/Users/john/documents/file.txt"),
                Path::new("/Users/john/projects")
            ),
            Some("../documents/file.txt".to_string())
        );

        // Root to subdirectory
        assert_eq!(
            to_relative_path(Path::new("/var/log/system.log"), Path::new("/")),
            Some("var/log/system.log".to_string())
        );

        // Handles paths that would have leading slashes correctly
        assert_eq!(
            to_relative_path(Path::new("/home/user/file.txt"), Path::new("/home")),
            Some("user/file.txt".to_string())
        );

        // Test with current directory references that should be cleaned
        assert_eq!(
            to_relative_path(
                Path::new("/Users/john/projects/./app/src/main.rs"),
                Path::new("/Users/john/projects")
            ),
            Some("app/src/main.rs".to_string()),
        );
    }
}

#[test]
fn test_normalize_relative_path_for_glob() {
    use std::path::Path;

    assert_eq!(
        normalize_relative_path_for_glob(Path::new("app/src/main.rs")),
        "app/src/main.rs"
    );
    assert_eq!(
        normalize_relative_path_for_glob(Path::new("./app/src/main.rs")),
        "app/src/main.rs"
    );
    assert_eq!(
        normalize_relative_path_for_glob(Path::new("../app/src/main.rs")),
        "app/src/main.rs"
    );
    assert_eq!(normalize_relative_path_for_glob(Path::new("..")), "");
    assert_eq!(normalize_relative_path_for_glob(Path::new("")), "");
}

#[test]
fn test_posix_escape() {
    let shell_family = ShellFamily::Posix;
    assert_eq!(
        shell_family.escape("~/test_dir/library% 1$2"),
        "\\~/test_dir/library%\\ 1\\$2"
    );
    assert_eq!(shell_family.escape("あい"), "あい");
    assert_eq!(shell_family.escape("abc \n \t"), "abc\\ \\\n\\ \\\t");
    assert_eq!(shell_family.escape(""), "''");
    assert_eq!(
        shell_family.escape("foo '\"' bar"),
        "foo\\ \\'\\\"\\'\\ bar"
    );
}

#[test]
fn test_powershell_escape() {
    let shell_family = ShellFamily::PowerShell;
    assert_eq!(
        shell_family.escape("~/test_dir/library% 1$2"),
        "~/test_dir/library%` 1`$2"
    );
    assert_eq!(shell_family.escape("あい"), "あい");
    assert_eq!(shell_family.escape("abc \n \t"), "abc` `\n` `\t");
    assert_eq!(shell_family.escape(""), "''");
    assert_eq!(shell_family.escape("foo '\"' bar"), "foo` `'`\"`'` bar");
}

#[test]
fn test_posix_unescape() {
    let shell_family = ShellFamily::Posix;
    // Escaped spaces
    assert_eq!(shell_family.unescape("my\\ file.txt"), "my file.txt");
    // Multiple escaped characters
    assert_eq!(
        shell_family.unescape("path/to/my\\ file\\ \\(1\\).txt"),
        "path/to/my file (1).txt"
    );
    // No escaping needed — returns borrowed
    assert!(matches!(
        shell_family.unescape("simple.txt"),
        std::borrow::Cow::Borrowed(_)
    ));
    // Trailing backslash kept as-is
    assert_eq!(shell_family.unescape("trailing\\"), "trailing\\");
    // Roundtrip: unescape(escape(x)) == x
    let original = "hello world $HOME 'quotes'";
    assert_eq!(
        shell_family.unescape(&shell_family.escape(original)),
        original
    );
}

#[test]
fn test_powershell_unescape() {
    let shell_family = ShellFamily::PowerShell;
    // Escaped spaces
    assert_eq!(shell_family.unescape("my` file.txt"), "my file.txt");
    // Multiple escaped characters
    assert_eq!(shell_family.unescape("path` `$var"), "path $var");
    // No escaping needed — returns borrowed
    assert!(matches!(
        shell_family.unescape("simple.txt"),
        std::borrow::Cow::Borrowed(_)
    ));
    // Roundtrip: unescape(escape(x)) == x
    let original = "hello world $HOME";
    assert_eq!(
        shell_family.unescape(&shell_family.escape(original)),
        original
    );
}

#[test]
fn test_clean_path() {
    assert_eq!(
        CleanPathResult::with_line_and_column_number("Cargo.toml:10:5"),
        CleanPathResult {
            path: "Cargo.toml".into(),
            line_and_column_num: Some(LineAndColumnArg {
                line_num: 10,
                column_num: Some(5)
            }),
        }
    );

    assert_eq!(
        CleanPathResult::with_line_and_column_number("Cargo.toml:30:5abc"),
        CleanPathResult {
            path: "Cargo.toml:30:5abc".into(),
            line_and_column_num: None
        }
    );

    assert_eq!(
        CleanPathResult::with_line_and_column_number("Cargo.toml[30,5]"),
        CleanPathResult {
            path: "Cargo.toml".into(),
            line_and_column_num: Some(LineAndColumnArg {
                line_num: 30,
                column_num: Some(5)
            })
        }
    );

    assert_eq!(
        CleanPathResult::with_line_and_column_number("Cargo.toml(3,1)"),
        CleanPathResult {
            path: "Cargo.toml".into(),
            line_and_column_num: Some(LineAndColumnArg {
                line_num: 3,
                column_num: Some(1)
            })
        }
    );

    assert_eq!(
        CleanPathResult::with_line_and_column_number("Cargo.toml\", line 100, in"),
        CleanPathResult {
            path: "Cargo.toml".into(),
            line_and_column_num: Some(LineAndColumnArg {
                line_num: 100,
                column_num: None,
            })
        }
    );

    assert_eq!(
        CleanPathResult::with_line_and_column_number("Cargo.toml\", line 5, column 20"),
        CleanPathResult {
            path: "Cargo.toml".into(),
            line_and_column_num: Some(LineAndColumnArg {
                line_num: 5,
                column_num: Some(20),
            })
        }
    );

    assert_eq!(
        CleanPathResult::with_line_and_column_number("Cargo.toml#L100"),
        CleanPathResult {
            path: "Cargo.toml".into(),
            line_and_column_num: Some(LineAndColumnArg {
                line_num: 100,
                column_num: None
            })
        }
    );

    assert_eq!(
        CleanPathResult::with_line_and_column_number("Cargo.toml#L100:4"),
        CleanPathResult {
            path: "Cargo.toml".into(),
            line_and_column_num: Some(LineAndColumnArg {
                line_num: 100,
                column_num: Some(4)
            })
        }
    );

    // Line range format :start-end (should link to start line, ignore end line)
    assert_eq!(
        CleanPathResult::with_line_and_column_number("Cargo.toml:10-50"),
        CleanPathResult {
            path: "Cargo.toml".into(),
            line_and_column_num: Some(LineAndColumnArg {
                line_num: 10,
                column_num: None
            })
        }
    );

    assert_eq!(
        CleanPathResult::with_line_and_column_number("/path/to/file.rs:1-1000"),
        CleanPathResult {
            path: "/path/to/file.rs".into(),
            line_and_column_num: Some(LineAndColumnArg {
                line_num: 1,
                column_num: None
            })
        }
    );

    assert_eq!(
        CleanPathResult::with_line_and_column_number("src/main.rs:100-100"),
        CleanPathResult {
            path: "src/main.rs".into(),
            line_and_column_num: Some(LineAndColumnArg {
                line_num: 100,
                column_num: None
            })
        }
    );
}

// ── group_roots_by_common_ancestor tests ─────────────────────────────

mod group_roots_by_common_ancestor_tests {
    use crate::path::group_roots_by_common_ancestor;
    use std::path::PathBuf;

    fn pb(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn empty_input_produces_empty_grouping() {
        let grouping = group_roots_by_common_ancestor::<PathBuf>(&[]);
        assert!(grouping.roots.is_empty());
        assert!(grouping.absorbed_by_root.is_empty());
    }

    #[test]
    fn single_path_survives_with_no_absorbed() {
        let grouping = group_roots_by_common_ancestor(&[pb("/a")]);
        assert_eq!(grouping.roots, vec![pb("/a")]);
        assert!(grouping.absorbed_by_root.is_empty());
    }

    #[test]
    fn unrelated_siblings_both_survive() {
        let grouping = group_roots_by_common_ancestor(&[pb("/a"), pb("/b")]);
        assert_eq!(grouping.roots, vec![pb("/a"), pb("/b")]);
        assert!(grouping.absorbed_by_root.is_empty());
    }

    #[test]
    fn descendant_absorbed_into_ancestor() {
        // Ancestor listed first.
        let grouping = group_roots_by_common_ancestor(&[pb("/a"), pb("/a/b")]);
        assert_eq!(grouping.roots, vec![pb("/a")]);
        assert_eq!(grouping.absorbed_by_root.len(), 1);
        assert_eq!(grouping.absorbed_by_root[&pb("/a")], vec![pb("/a/b")]);
    }

    #[test]
    fn descendant_first_still_absorbed() {
        // Descendant listed first, ancestor second; survivor is still the
        // ancestor and its input order is preserved.
        let grouping = group_roots_by_common_ancestor(&[pb("/a/b"), pb("/a")]);
        assert_eq!(grouping.roots, vec![pb("/a")]);
        assert_eq!(grouping.absorbed_by_root[&pb("/a")], vec![pb("/a/b")]);
    }

    #[test]
    fn three_deep_chain_collapses_to_root() {
        // Descendant order in input is preserved in the absorbed list.
        let grouping = group_roots_by_common_ancestor(&[pb("/a/b/c"), pb("/a/b"), pb("/a")]);
        assert_eq!(grouping.roots, vec![pb("/a")]);
        assert_eq!(
            grouping.absorbed_by_root[&pb("/a")],
            vec![pb("/a/b/c"), pb("/a/b")]
        );
    }

    #[test]
    fn mixed_groups_absorb_independently() {
        let grouping =
            group_roots_by_common_ancestor(&[pb("/a"), pb("/x"), pb("/a/b"), pb("/x/y")]);
        assert_eq!(grouping.roots, vec![pb("/a"), pb("/x")]);
        assert_eq!(grouping.absorbed_by_root[&pb("/a")], vec![pb("/a/b")]);
        assert_eq!(grouping.absorbed_by_root[&pb("/x")], vec![pb("/x/y")]);
    }

    #[test]
    fn same_prefix_different_component_name_both_survive() {
        // /foo/a is NOT an ancestor of /foo/abc (component-aware match).
        let grouping = group_roots_by_common_ancestor(&[pb("/foo/a"), pb("/foo/abc")]);
        assert_eq!(grouping.roots, vec![pb("/foo/a"), pb("/foo/abc")]);
        assert!(grouping.absorbed_by_root.is_empty());
    }

    #[test]
    fn duplicate_inputs_collapse_to_single_survivor() {
        let grouping = group_roots_by_common_ancestor(&[pb("/a"), pb("/a")]);
        assert_eq!(grouping.roots, vec![pb("/a")]);
        assert!(grouping.absorbed_by_root.is_empty());
    }

    #[test]
    fn surviving_root_order_matches_input_order() {
        // Insert a descendant between two surviving ancestors; the survivors
        // should appear in their original input order even though processing
        // sorted by component count.
        let grouping = group_roots_by_common_ancestor(&[pb("/b"), pb("/a/x"), pb("/a"), pb("/c")]);
        assert_eq!(grouping.roots, vec![pb("/b"), pb("/a"), pb("/c")]);
        assert_eq!(grouping.absorbed_by_root[&pb("/a")], vec![pb("/a/x")]);
    }

    #[test]
    fn descendant_absorbed_by_closest_ancestor_not_furthest() {
        // Both /a and /a/b are surviving ancestors of /a/b/c... wait, /a/b is
        // itself absorbed into /a. So /a/b/c should be absorbed into /a as
        // well (the only surviving ancestor).
        let grouping = group_roots_by_common_ancestor(&[pb("/a"), pb("/a/b"), pb("/a/b/c")]);
        assert_eq!(grouping.roots, vec![pb("/a")]);
        assert_eq!(
            grouping.absorbed_by_root[&pb("/a")],
            vec![pb("/a/b"), pb("/a/b/c")]
        );
    }
}
