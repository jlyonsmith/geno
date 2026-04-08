use crate::{Location, tokenizer::Token, tokenizer::TokenizeError};
use std::{error::Error, fmt, path::PathBuf};

/// Error produced by the parser.
#[derive(Debug, PartialEq)]
pub enum ParserError {
    /// Tokenizer error
    TokenizerError {
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// The tokenizer error
        error: TokenizeError,
    },
    /// Unexpected token
    UnexpectedToken {
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// The token that was unexpected
        token: Token,
    },
    /// Unexpected end-of-file
    UnexpectedEndOfFile {
        /// The path to the file that caused the error
        file_path: PathBuf,
    },
    /// Multiple schema attributes
    MultipleSchemaAttributes {
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Multiple attributes
    MultipleAttributes {
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Missing bracket
    MissingBracket {
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Missing brace
    MissingBrace {
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Missing colon
    MissingColon {
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Number out of range error
    NumberRange {
        /// The content that caused the error
        content: String,
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Unexpected comma
    UnexpectedComma {
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Missing comma
    MissingComma {
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },

    /// Duplicate type error
    DuplicateType {
        /// The type that was duplicated
        type_name: String,
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Undefined type error
    UndefinedType {
        /// The name of the undefined type
        name: String,
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Duplicate field error
    DuplicateField {
        /// The name of the struct that has the duplicate field
        struct_name: String,
        /// The name of the duplicate field
        name: String,
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Duplicate enum variant name
    DuplicateVariant {
        /// The name of the enum that has the duplicate variant
        enum_name: String,
        /// The name of the duplicate variant
        name: String,
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Duplicate enum value
    DuplicateVariantValue {
        /// The name of the enum that has the duplicate value
        enum_name: String,
        /// The value that was duplicated
        value: String,
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Enumeration has no variants
    EmptyEnum {
        /// The name of the empty enum
        name: String,
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Metadata format is not valid
    InvalidMetadataFormat {
        /// The value that was invalid
        value: String,
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Metadata format is missing
    MissingMetadataFormat {
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Must start with an uppercase letter
    MustBePascalCase {
        /// The name of the identifier
        name: String,
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Must start with a lowercase letter
    MustBeCamelCase {
        /// The name of the identifier
        name: String,
        /// The path to the file that caused the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UnexpectedToken { file_path, token } => {
                write!(
                    f,
                    "unexpected token '{0}' ({1}:{2})",
                    token.kind,
                    file_path.display(),
                    token.location
                )
            }
            Self::MultipleSchemaAttributes {
                file_path,
                location,
            } => {
                write!(
                    f,
                    "multiple schema attributes ({0}:{location})",
                    file_path.display()
                )
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

// #[error("value out of range '{content}' ({file_path}:{location})")]
// #[error("duplicate type definition '{type_name}' ({file_path}:{location})")]
// #[error("undefined type '{name}' ({file_path}:{location})")]
// #[error("duplicate field '{name}' in struct '{struct_name}' ({file_path}:{location})")]
// #[error("duplicate variant name '{name}' in enum '{enum_name}' ({file_path}:{location})")]
// #[error("duplicate variant value '{value}' in enum '{enum_name}' ({file_path}:{location})")]
// #[error("enum '{name}' has no variants ({file_path}:{location})")]
// #[error("metadata format {value} invalid ({file_path}:{location})")]
// #[error("metadata format missing ({file_path}:{location})")]
// #[error("identifier {name} must be Pascal case ({file_path}:{location})")]
// #[error("identifier {name} must be camel case ({file_path}:{location})")]

impl Error for ParserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParserError::TokenizerError { error, .. } => Some(error),
            _ => None,
        }
    }
}
