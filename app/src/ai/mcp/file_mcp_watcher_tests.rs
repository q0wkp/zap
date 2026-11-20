// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: added tests that an unparsable file config is not taken for an empty one and that a missing one is reported as missing.

use super::{parse_mcp_config_file, substitute_env_vars, FileMCPConfigParseOutcome, MCPProvider};
use std::env;

fn cleanup_env_vars(vars: &[&str]) {
    for var in vars {
        env::remove_var(var);
    }
}

#[test]
fn test_substitute_env_vars_success() {
    let test_vars = ["FOO", "BAZ", "REPEATED"];

    // Setup environment variables
    env::set_var("FOO", "bar");
    env::set_var("BAZ", "qux");
    env::set_var("REPEATED", "value");

    // Test 1: Single variable substitution
    let input = r#"{"key": "${FOO}"}"#;
    let result = substitute_env_vars(input).expect("Single variable substitution should succeed");
    assert_eq!(
        result, r#"{"key": "bar"}"#,
        "Single variable FOO should be replaced with 'bar'"
    );

    // Test 2: Multiple different variables
    let input = r#"{"key": "${FOO}", "other": "${BAZ}"}"#;
    let result = substitute_env_vars(input).expect("Multiple variable substitution should succeed");
    assert_eq!(
        result, r#"{"key": "bar", "other": "qux"}"#,
        "Multiple variables FOO and BAZ should be replaced"
    );

    // Test 3: Multiple occurrences of same variable
    let input = r#"{"a": "${REPEATED}", "b": "${REPEATED}", "c": "prefix_${REPEATED}_suffix"}"#;
    let result = substitute_env_vars(input).expect("Repeated variable substitution should succeed");
    assert_eq!(
        result, r#"{"a": "value", "b": "value", "c": "prefix_value_suffix"}"#,
        "All occurrences of REPEATED should be replaced with 'value', including within context"
    );

    // Cleanup
    cleanup_env_vars(&test_vars);
}

#[test]
fn test_substitute_env_vars_missing_or_empty() {
    // Test 1: Missing variable
    // Ensure MISSING_VAR is not set
    env::remove_var("MISSING_VAR");

    let input = r#"{"key": "${MISSING_VAR}"}"#;
    let result = substitute_env_vars(input);
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Missing or empty environment variable: MISSING_VAR"),
        "Error message should mention MISSING_VAR, got: {err_msg}"
    );

    // Test 2: Empty variable
    env::set_var("EMPTY_VAR", "");

    let input = r#"{"key": "${EMPTY_VAR}"}"#;
    let result = substitute_env_vars(input);
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Missing or empty environment variable: EMPTY_VAR"),
        "Error message should mention EMPTY_VAR, got: {err_msg}"
    );

    // Cleanup
    cleanup_env_vars(&["EMPTY_VAR"]);
}

fn parse_config_with_contents(contents: Option<&str>) -> FileMCPConfigParseOutcome {
    let directory = tempfile::tempdir().expect("failed to create a temporary directory");
    let config_path = directory.path().join(".mcp.json");
    if let Some(contents) = contents {
        std::fs::write(&config_path, contents).expect("failed to write the config file");
    }
    futures::executor::block_on(parse_mcp_config_file(&config_path, MCPProvider::Claude))
}

/// A config that fails to parse (here, one caught half-written) must not be taken for a config
/// that defines no servers, which would stop the servers of the last valid snapshot.
#[test]
fn test_unparsable_config_is_not_an_empty_snapshot() {
    assert!(matches!(
        parse_config_with_contents(Some(r#"{"mcpServers": {"#)),
        FileMCPConfigParseOutcome::Error
    ));
}

#[test]
fn test_missing_config_is_reported_as_missing() {
    assert!(matches!(
        parse_config_with_contents(None),
        FileMCPConfigParseOutcome::Missing
    ));
}

#[test]
fn test_config_without_servers_is_an_empty_snapshot() {
    let outcome = parse_config_with_contents(Some(r#"{"mcpServers": {}}"#));
    assert!(
        matches!(&outcome, FileMCPConfigParseOutcome::Parsed(servers) if servers.is_empty()),
        "a valid config without servers should parse to an empty snapshot"
    );
}
