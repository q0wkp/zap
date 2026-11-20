// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

use chrono::{DateTime, Local};
use zap_ng_ui::{Entity, ModelContext};

use crate::code::editor::line::EditorLineLocation;
use crate::code_review::comments::{
    AttachedReviewComment, AttachedReviewCommentTarget, CommentId, CommentOrigin, LineDiffContent,
};

#[derive(Debug, Clone)]
pub enum PendingCommentEvent {
    NewPendingComment(EditorLineLocation),
    ReopenPendingComment {
        id: CommentId,
        line: EditorLineLocation,
        comment_text: String,
        origin: CommentOrigin,
    },
}

pub enum PendingComment {
    Closed,
    Open { line: EditorLineLocation },
}

pub struct EditorCommentsModel {
    pub pending_comment: PendingComment,
}

impl EditorCommentsModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            pending_comment: PendingComment::Closed,
        }
    }
}

impl Entity for EditorCommentsModel {
    type Event = PendingCommentEvent;
}

/// Used solely at the CodeEditorView level, when we don't know
/// the file path, and later converted to a full `AttachedReviewComment`.
#[derive(Clone, Debug)]
pub struct EditorReviewComment {
    pub id: CommentId,
    pub line: EditorLineLocation,
    pub diff_content: LineDiffContent,
    pub comment_content: String,
    pub last_update_time: DateTime<Local>,
}

impl EditorReviewComment {
    pub(crate) fn new(
        line: EditorLineLocation,
        diff_content: LineDiffContent,
        comment_content: String,
    ) -> Self {
        Self {
            id: CommentId::new(),
            line,
            diff_content,
            comment_content,
            last_update_time: Local::now(),
        }
    }

    pub(crate) fn new_with_id(
        id: CommentId,
        line: EditorLineLocation,
        diff_content: LineDiffContent,
        comment_content: String,
    ) -> Self {
        Self {
            id,
            line,
            diff_content,
            comment_content,
            last_update_time: Local::now(),
        }
    }
}

impl TryFrom<AttachedReviewComment> for EditorReviewComment {
    type Error = ();

    fn try_from(comment: AttachedReviewComment) -> Result<Self, Self::Error> {
        match comment.target {
            AttachedReviewCommentTarget::Line { content, line, .. } => Ok(EditorReviewComment {
                id: comment.id,
                line,
                diff_content: content,
                comment_content: comment.content,
                last_update_time: comment.last_update_time,
            }),
            _ => Err(()),
        }
    }
}
