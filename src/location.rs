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

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}
