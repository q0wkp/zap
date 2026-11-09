use super::*;

#[test]
fn test_program_invalid_bash() {
    // This test assumes there is no bash binary at /some/weird/path/bash.
    let shell_path = "/some/weird/path/bash".to_owned();
    assert!(supported_shell_path_and_type(&shell_path).is_none());
}

#[test]
fn test_program_invalid_zsh() {
    // This test assumes there is no bash zsh at /some/weird/path/bash.
    let shell_path = "/some/weird/path/zsh".to_owned();
    assert!(supported_shell_path_and_type(&shell_path).is_none());
}

#[test]
fn test_program_unknown_shell() {
    let shell_path = "/some/weird/path/wtfsh".to_owned();
    assert!(supported_shell_path_and_type(&shell_path).is_none());
}
