use crate::{Location, ResolverError, TokenizeError};
use std::path::PathBuf;
use thiserror::Error;

/// Error produced by the parser.
#[derive(Debug, Error)]
pub enum ParserError {
    /// Tokenizer error
    #[error("tokenizer error ({file_path}:{location})")]
    TokenizerError {
        /// The tokenizer error
        #[source]
        error: TokenizeError,
        /// [Location] of the parse error
        location: Location,
        /// The path to the source file containing the error
        file_path: PathBuf,
    },
    /// Resolver error
    #[error("unable to resolve file ({file_path})")]
    ResolverError {
        /// The resolver error
        #[source]
        error: ResolverError,
        /// The path to the file being resolved
        file_path: PathBuf,
    },
    /// Unexpected token
    #[error("unexpected token ({file_path}:{location})")]
    UnexpectedToken {
        /// The token that was unexpected
        token: String,
        /// [Location] of the parse error
        location: Location,
        /// The path to the source file containing the error
        file_path: PathBuf,
    },
    /// Unexpected end-of-file
    #[error("unexpected end of file ({file_path})")]
    UnexpectedEndOfFile {
        /// The path to the source file containing the error
        file_path: PathBuf,
    },
    /// Multiple schema attributes
    #[error("multiple schema attributes ({file_path}:{location})")]
    MultipleSchemaAttributes {
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Multiple attributes
    #[error("multiple attributes ({file_path}:{location})")]
    MultipleAttributes {
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Missing bracket
    #[error("missing bracket ({file_path}:{location})")]
    MissingBracket {
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    #[error("missing brace ({file_path}:{location})")]
    /// Missing brace
    MissingBrace {
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Missing colon
    #[error("missing colon ({file_path}:{location})")]
    MissingColon {
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Number out of range error
    #[error("number out of range ({file_path}:{location})")]
    NumberRange {
        /// The content that caused the error
        content: String,
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Unexpected comma
    #[error("unexpected comma ({file_path}:{location})")]
    UnexpectedComma {
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Missing comma
    #[error("missing comma ({file_path}:{location})")]
    MissingComma {
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },

    /// Duplicate type error
    #[error("duplicate type ({file_path}:{location})")]
    DuplicateType {
        /// The type that was duplicated
        type_name: String,
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Undefined type error
    #[error("undefined type ({file_path}:{location})")]
    UndefinedType {
        /// The name of the undefined type
        name: String,
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Duplicate field error
    #[error("duplicate field ({file_path}:{location})")]
    DuplicateField {
        /// The name of the struct that has the duplicate field
        struct_name: String,
        /// The name of the duplicate field
        name: String,
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Duplicate enum variant name
    #[error("duplicate enum variant ({file_path}:{location})")]
    DuplicateVariant {
        /// The name of the enum that has the duplicate variant
        enum_name: String,
        /// The name of the duplicate variant
        name: String,
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Duplicate enum value
    #[error("duplicate enum value ({file_path}:{location})")]
    DuplicateVariantValue {
        /// The name of the enum that has the duplicate value
        enum_name: String,
        /// The value that was duplicated
        value: String,
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Enumeration has no variants
    #[error("empty enum ({file_path}:{location})")]
    EmptyEnum {
        /// The name of the empty enum
        name: String,
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Metadata format is not valid
    #[error("invalid metadata format ({file_path}:{location})")]
    InvalidMetadataFormat {
        /// The value that was invalid
        value: String,
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Metadata format is missing
    #[error("missing metadata format ({file_path}:{location})")]
    MissingMetadataFormat {
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Must start with an uppercase letter
    #[error("must start with an uppercase letter ({file_path}:{location})")]
    MustBePascalCase {
        /// The name of the identifier
        name: String,
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Must start with a lowercase letter
    #[error("must start with a lowercase letter ({file_path}:{location})")]
    MustBeCamelCase {
        /// The name of the identifier
        name: String,
        /// The path to the source file containing the error
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
}

impl TokenizeError {
    /// Converts a [TokenizeError] into a [ParseError]
    pub fn to_parser_error(&self, file_path: PathBuf) -> ParserError {
        match self {
            TokenizeError::UnexpectedChar { location, .. } => ParserError::TokenizerError {
                error: self.clone(),
                location: *location,
                file_path,
            },
            TokenizeError::UnterminatedString { location, .. } => ParserError::TokenizerError {
                error: self.clone(),
                location: *location,
                file_path,
            },
            TokenizeError::InvalidNumber { location, .. } => ParserError::TokenizerError {
                error: self.clone(),
                location: *location,
                file_path,
            },
        }
    }
}

impl ResolverError {
    /// Converts a [ResolverError] into a [ParseError]
    pub fn to_parser_error(&self) -> ParserError {
        match self {
            Self::Io(file_path, _) => ParserError::ResolverError {
                error: self.clone(),
                file_path: file_path.clone(),
            },
            Self::DuplicateInclude(file_path) => ParserError::ResolverError {
                error: self.clone(),
                file_path: file_path.clone(),
            },
        }
    }
}
