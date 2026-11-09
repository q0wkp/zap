#[cfg_attr(unix, path = "unix.rs")]
mod imp;

use crate::terminal::shell::ShellType;
pub use imp::TempBootstrapFile;

/// Creates a `NamedTempFile` with the given bootstrap contents
///
/// Return `None` if any part of the operation fails
#[cfg(feature = "local_fs")]
pub fn create_bootstrap_file<C>(contents: C, shell_type: ShellType) -> Option<TempBootstrapFile>
where
    C: AsRef<[u8]>,
{
    let mut builder = tempfile::Builder::new();
    // PowerShell will only source a file with the "ps1" extension.
    if shell_type == ShellType::PowerShell {
        builder.suffix(".ps1");
    }

    match TempBootstrapFile::new(builder, contents) {
        Ok(bootstrap_file) => Some(bootstrap_file),
        Err(err) => {
            log::warn!("Error when creating temporary bootstrap file: {err:#}");
            None
        }
    }
}
