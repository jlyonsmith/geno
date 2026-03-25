use pest::{Span, error::LineColLocation};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// A location within a source file
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Location {
    /// The one-based line number of the error.
    pub line: usize,
    /// The one-based column number of the error.
    pub column: usize,
}

/// Create a location from a pest span
impl From<&Span<'_>> for Location {
    fn from(s: &Span<'_>) -> Self {
        let (line, column) = s.start_pos().line_col();
        Self { line, column }
    }
}

impl From<LineColLocation> for Location {
    fn from(lc: LineColLocation) -> Self {
        match lc {
            LineColLocation::Pos(pos) => Self {
                line: pos.0,
                column: pos.1,
            },
            LineColLocation::Span(start, _) => Self {
                line: start.0,
                column: start.1,
            },
        }
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}
