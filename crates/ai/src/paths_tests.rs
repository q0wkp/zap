use super::*;

#[cfg(unix)]
#[test]
fn test_host_native_absolute_path() {
    // Test with absolute path
    assert_eq!(
        host_native_absolute_path("/home/user/file.txt", &Some("/current/dir".to_string())),
        "/home/user/file.txt"
    );

    // Test with relative path
    assert_eq!(
        host_native_absolute_path("file.txt", &Some("/current/dir".to_string())),
        "/current/dir/file.txt"
    );

    // Test with tilde expansion
    assert_eq!(
        host_native_absolute_path("~/file.txt", &Some("/current/dir".to_string())),
        shellexpand::tilde("~/file.txt").into_owned()
    );

    // Test with ..
    assert_eq!(
        host_native_absolute_path("../user/file.txt", &Some("/current/dir".to_string())),
        "/current/user/file.txt"
    );

    // Test with .
    assert_eq!(
        host_native_absolute_path("./user/file.txt", &Some("/current/dir".to_string())),
        "/current/dir/user/file.txt"
    );

    // Test with no current working directory
    assert_eq!(host_native_absolute_path("file.txt", &None), "file.txt");

    // Test with empty current working directory
    assert_eq!(
        host_native_absolute_path("file.txt", &Some("".to_string())),
        "file.txt"
    );
}

#[cfg(unix)]
#[test]
fn test_shell_native_absolute_path() {
    // Test with absolute path
    let cwd = Some("/current/dir".to_string());
    assert_eq!(
        shell_native_absolute_path("/home/user/file.txt", cwd.as_ref()),
        "/home/user/file.txt"
    );

    // Test with relative path
    let cwd = Some("/current/dir".to_string());
    assert_eq!(
        shell_native_absolute_path("file.txt", cwd.as_ref()),
        "/current/dir/file.txt"
    );

    // Test with tilde expansion
    let cwd = Some("/current/dir".to_string());
    assert_eq!(
        shell_native_absolute_path("~/file.txt", cwd.as_ref()),
        shellexpand::tilde("~/file.txt").into_owned()
    );

    // Test with ..
    let cwd = Some("/current/dir".to_string());
    assert_eq!(
        shell_native_absolute_path("../user/file.txt", cwd.as_ref()),
        "/current/user/file.txt"
    );

    // Test with .
    let cwd = Some("/current/dir".to_string());
    assert_eq!(
        shell_native_absolute_path("./user/file.txt", cwd.as_ref()),
        "/current/dir/user/file.txt"
    );

    // Test with no current working directory
    assert_eq!(shell_native_absolute_path("file.txt", None), "file.txt");

    // Test with empty current working directory
    let cwd = Some("".to_string());
    assert_eq!(
        shell_native_absolute_path("file.txt", cwd.as_ref()),
        "file.txt"
    );
}
