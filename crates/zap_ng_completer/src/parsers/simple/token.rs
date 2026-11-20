// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

#[derive(Debug, PartialEq, Eq)]
pub enum Token<'a> {
    Literal(&'a str),
    /// Whitespace excluding newline
    Whitespace(&'a str),
    /// | Operator
    Pipe,
    /// || operator
    LogicalOr,
    /// & operator
    Ampersand,
    /// && operator
    LogicalAnd,
    Semicolon,
    Newline,
    Backtick,
    OpenParen,
    CloseParen,
    OpenCurly,
    CloseCurly,
    Dollar,
    SingleQuote,
    DoubleQuote,
    /// \ or `
    EscapeChar(&'a str),
    RedirectInput,
    RedirectOutput,
}

impl Token<'_> {
    pub fn as_str(&self) -> &str {
        use Token::*;
        match self {
            Literal(value) | Whitespace(value) | EscapeChar(value) => value,
            Pipe => "|",
            LogicalOr => "||",
            Ampersand => "&",
            LogicalAnd => "&&",
            Semicolon => ";",
            Newline => "\n",
            Backtick => "`",
            OpenParen => "(",
            CloseParen => ")",
            OpenCurly => "{",
            CloseCurly => "}",
            Dollar => "$",
            SingleQuote => "'",
            DoubleQuote => "\"",
            RedirectInput => "<",
            RedirectOutput => ">",
        }
    }
}
