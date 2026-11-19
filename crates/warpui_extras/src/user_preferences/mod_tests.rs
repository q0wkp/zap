use std::fs;
use std::path::Path;

use super::write_file_atomically;

fn file_names_in(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .expect("failed to read the directory")
        .map(|entry| {
            entry
                .expect("failed to read a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn atomic_write_creates_the_file_and_its_parent_directory() {
    let directory = tempfile::tempdir().expect("failed to create a temporary directory");
    let file_path = directory.path().join("nested").join("settings.toml");

    write_file_atomically(&file_path, b"first").expect("the write should succeed");

    assert_eq!(
        fs::read(&file_path).expect("failed to read the file"),
        b"first"
    );
    assert_eq!(
        file_names_in(file_path.parent().expect("the file has a parent")),
        ["settings.toml"]
    );
}

#[test]
fn atomic_write_replaces_the_contents_and_leaves_no_temporary_file() {
    let directory = tempfile::tempdir().expect("failed to create a temporary directory");
    let file_path = directory.path().join("settings.toml");
    fs::write(&file_path, "a much longer first version").expect("failed to write the file");

    write_file_atomically(&file_path, b"second").expect("the write should succeed");

    assert_eq!(
        fs::read(&file_path).expect("failed to read the file"),
        b"second"
    );
    assert_eq!(file_names_in(directory.path()), ["settings.toml"]);
}

/// The file must be replaced, never truncated and rewritten: a handle opened before the write
/// still reads the previous contents in full, which is also what is left on disk if the write
/// stops halfway.
#[cfg(unix)]
#[test]
fn atomic_write_does_not_modify_the_previous_file_in_place() {
    use std::io::Read as _;

    let directory = tempfile::tempdir().expect("failed to create a temporary directory");
    let file_path = directory.path().join("settings.toml");
    fs::write(&file_path, "first version").expect("failed to write the file");
    let mut previous_file = fs::File::open(&file_path).expect("failed to open the file");

    write_file_atomically(&file_path, b"second").expect("the write should succeed");

    let mut previous_contents = String::new();
    previous_file
        .read_to_string(&mut previous_contents)
        .expect("failed to read the previous file");
    assert_eq!(previous_contents, "first version");
    assert_eq!(
        fs::read(&file_path).expect("failed to read the file"),
        b"second"
    );
}

#[cfg(unix)]
#[test]
fn atomic_write_keeps_a_symbolic_link_and_replaces_its_target() {
    let directory = tempfile::tempdir().expect("failed to create a temporary directory");
    let real_path = directory.path().join("dotfiles").join("settings.toml");
    fs::create_dir_all(real_path.parent().expect("the file has a parent"))
        .expect("failed to create the directory");
    fs::write(&real_path, "first").expect("failed to write the file");
    let link_path = directory.path().join("settings.toml");
    std::os::unix::fs::symlink(&real_path, &link_path).expect("failed to create the link");

    write_file_atomically(&link_path, b"second").expect("the write should succeed");

    assert!(fs::symlink_metadata(&link_path)
        .expect("failed to read the link")
        .file_type()
        .is_symlink());
    assert_eq!(
        fs::read(&real_path).expect("failed to read the file"),
        b"second"
    );
}

#[cfg(unix)]
#[test]
fn atomic_write_keeps_the_permissions_of_an_existing_file() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = tempfile::tempdir().expect("failed to create a temporary directory");
    let file_path = directory.path().join("settings.toml");
    fs::write(&file_path, "first").expect("failed to write the file");
    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600))
        .expect("failed to set the permissions");

    write_file_atomically(&file_path, b"second").expect("the write should succeed");

    let mode = fs::metadata(&file_path)
        .expect("failed to read the metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}
