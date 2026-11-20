// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use zap_ng_ui::elements::MouseStateHandle;
use zap_ng_ui::{AppContext, ViewHandle};

use crate::code::local_code_editor::LocalCodeEditorView;

pub struct CodeReviewEditorState {
    pub editor: ViewHandle<LocalCodeEditorView>,
    unsaved_changes_mouse_state: MouseStateHandle,
    pub(super) editor_mouse_state: MouseStateHandle,
    /// Whether the buffer content has been loaded from disk (for global buffer mode).
    /// This is set to true when LocalCodeEditorEvent::DelayedRenderingFlushed or FailedToLoad fires.
    is_loaded: bool,
}

impl CodeReviewEditorState {
    #[cfg(not(target_family = "wasm"))]
    pub fn new(editor: ViewHandle<LocalCodeEditorView>) -> Self {
        Self {
            editor,
            unsaved_changes_mouse_state: MouseStateHandle::default(),
            editor_mouse_state: MouseStateHandle::default(),
            is_loaded: false,
        }
    }

    /// Creates a new editor state that is already marked as loaded.
    /// Used for non-global buffer mode where content is loaded synchronously.
    pub fn new_loaded(editor: ViewHandle<LocalCodeEditorView>) -> Self {
        Self {
            editor,
            unsaved_changes_mouse_state: MouseStateHandle::default(),
            editor_mouse_state: MouseStateHandle::default(),
            is_loaded: true,
        }
    }

    /// Returns whether the buffer content has been loaded.
    pub fn is_loaded(&self) -> bool {
        self.is_loaded
    }

    /// Marks the editor as loaded.
    pub fn set_loaded(&mut self) {
        self.is_loaded = true;
    }

    pub fn editor(&self) -> &ViewHandle<LocalCodeEditorView> {
        &self.editor
    }

    pub fn unsaved_changes_mouse_state(&self) -> MouseStateHandle {
        self.unsaved_changes_mouse_state.clone()
    }

    pub fn has_unsaved_changes(&self, ctx: &AppContext) -> bool {
        self.editor.as_ref(ctx).has_unsaved_changes(ctx)
    }
}
