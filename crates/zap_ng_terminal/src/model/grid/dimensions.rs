// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

/// Grid dimensions.
pub trait Dimensions {
    /// Total number of lines in the buffer, this includes scrollback and visible lines.
    fn total_rows(&self) -> usize;

    /// Number of rows in the viewport
    #[allow(dead_code)]
    fn visible_rows(&self) -> usize;

    /// Width of the terminal in columns.
    fn columns(&self) -> usize;

    /// Number of invisible lines part of the scrollback history.
    #[inline]
    fn history_size(&self) -> usize {
        self.total_rows() - self.visible_rows()
    }
}

#[cfg(test)]
impl Dimensions for (usize, usize) {
    fn total_rows(&self) -> usize {
        self.0
    }

    fn visible_rows(&self) -> usize {
        self.0
    }

    fn columns(&self) -> usize {
        self.1
    }
}

#[cfg(test)]
impl Dimensions for (crate::model::VisibleRow, usize) {
    fn total_rows(&self) -> usize {
        self.0 .0
    }

    fn visible_rows(&self) -> usize {
        self.0 .0
    }

    fn columns(&self) -> usize {
        self.1
    }
}
