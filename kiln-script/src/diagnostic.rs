//! Diagnostics with span information for error reporting.

use std::fmt;

/// A span in source code (byte offset and length).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn length(&self) -> usize {
        self.end - self.start
    }
}

/// A diagnostic message (error or warning).
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    /// Format a compiler diagnostic with a file name and one-based Unicode
    /// line/column coordinates. Out-of-range spans are clamped safely.
    pub fn with_source_location(&self, path: &str, source: &str) -> String {
        let mut offset = self.span.start.min(source.len());
        while !source.is_char_boundary(offset) {
            offset -= 1;
        }
        let prefix = &source[..offset];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .chars()
            .count()
            + 1;
        format!("{path}:{line}:{column}: {}", self.message)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}..{}]: {}",
            self.span.start, self.span.end, self.message
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_locations_handle_multiline_unicode_and_invalid_offsets() {
        let source = "first\nλ bad";
        assert_eq!(
            Diagnostic::new(Span::new(9, 9), "error").with_source_location("enemy.ks", source),
            "enemy.ks:2:3: error"
        );
        assert_eq!(
            Diagnostic::new(Span::new(7, 7), "error").with_source_location("enemy.ks", source),
            "enemy.ks:2:1: error"
        );
        assert!(Diagnostic::new(Span::new(999, 999), "error")
            .with_source_location("enemy.ks", source)
            .starts_with("enemy.ks:2:6:"));
    }
}
