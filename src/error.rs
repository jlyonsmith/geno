use crate::Rule;
use pest::iterators::Pair;
use pest::{Span, error::LineColLocation};
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// A location within source file
#[derive(Copy, Clone, Debug, PartialEq)]
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

/// This crates error enum
#[derive(Error, Debug)]
pub enum GenoError {
    /// I/O error
    #[error("{error} on '{file}'")]
    Io {
        /// Path of the file that caused the error
        file: PathBuf,
        /// The I/O error that occurred
        error: std::io::Error,
    },
    /// Parsing error
    #[error("unable to parse '{content}' ({file}:{location})")]
    Parse {
        /// Content that caused the parse failure
        content: String,
        /// File path of the schema
        file: String,
        /// [Location] of the parse error
        location: Location,
    },
    /// Number out of range error
    #[error("value out of range '{content}' ({file}:{location})")]
    NumberRange {
        /// The content that caused the error
        content: String,
        /// File path of the schema
        file: String,
        /// [Location] of the parse error
        location: Location,
    },
    /// Duplicate type error
    #[error("duplicate type '{0}'")]
    DuplicateType(String),
    /// Undefined type error
    #[error("undefined type '{0}'")]
    UndefinedType(String),
    /// Duplicate field error
    #[error("duplicate field '{1}' in struct '{0}'")]
    DuplicateField(String, String),
    /// Duplicate enum variant name
    #[error("duplicate variant name '{1}' in enum '{0}'")]
    DuplicateVariant(String, String),
    /// Duplicate enum value
    #[error("duplicate variant value '{1}' in enum '{0}'")]
    DuplicateVariantValue(String, String),
    /// Enumeration has no variants
    #[error("enum '{0}' has no variants")]
    EmptyEnum(String),
    /// Metadata format is not valid
    #[error("metadata format is invalid")]
    InvalidMetadataFormat(),
}

impl GenoError {
    /// Create a new number range error
    pub fn new_number_range_error(pair: &Pair<'_, Rule>, file_path: &Path) -> Self {
        Self::NumberRange {
            content: pair.as_str().to_string(),
            file: file_path.to_string_lossy().into_owned(),
            location: Location::from(&pair.as_span()),
        }
    }
}
