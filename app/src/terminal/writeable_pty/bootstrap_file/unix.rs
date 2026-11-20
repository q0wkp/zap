// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: removed the permanent bootstrap file stub that only Windows used.

use std::io::Write;
use tempfile::NamedTempFile;


/// Represents a temporary file used for bootstrap.
pub struct TempBootstrapFile {
    temp_file: NamedTempFile,
}

impl TempBootstrapFile {
    pub fn new<C>(builder: tempfile::Builder, contents: C) -> std::io::Result<Self>
    where
        C: AsRef<[u8]>,
    {
        let mut file = builder.tempfile()?;
        file.write_all(contents.as_ref())?;
        file.flush()?;
        Ok(Self { temp_file: file })
    }

    pub fn path_as_bytes(&self) -> Option<Vec<u8>> {
        use std::os::unix::ffi::OsStringExt;
        Some(
            self.temp_file
                .path()
                .to_path_buf()
                .into_os_string()
                .into_vec(),
        )
    }
}

impl TryFrom<NamedTempFile> for TempBootstrapFile {
    type Error = tempfile::PersistError;

    fn try_from(temp_file: NamedTempFile) -> Result<Self, Self::Error> {
        Ok(Self { temp_file })
    }
}
