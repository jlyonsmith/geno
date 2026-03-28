use crate::Location;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// This crates error enum
#[derive(Error, Debug)]
pub enum GenoError {
    /// I/O error
    #[error("{error} on '{file_path}'")]
    Io {
        /// Path of the file that caused the error
        file_path: PathBuf,
        /// The I/O error that occurred
        error: std::io::Error,
    },
    /// Parsing error
    #[error("unable to parse '{content}' ({file_path}:{location})")]
    Parse {
        /// Content that caused the parse failure
        content: String,
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Number out of range error
    #[error("value out of range '{content}' ({file_path}:{location})")]
    NumberRange {
        /// The content that caused the error
        content: String,
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Duplicate type error
    #[error("duplicate type definition '{type_name}' ({file_path}:{location})")]
    DuplicateType {
        /// The type that was duplicated
        type_name: String,
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Undefined type error
    #[error("undefined type '{name}' ({file_path}:{location})")]
    UndefinedType {
        /// The name of the undefined type
        name: String,
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Duplicate field error
    #[error("duplicate field '{name}' in struct '{struct_name}' ({file_path}:{location})")]
    DuplicateField {
        /// The name of the struct that has the duplicate field
        struct_name: String,
        /// The name of the duplicate field
        name: String,
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Duplicate enum variant name
    #[error("duplicate variant name '{name}' in enum '{enum_name}' ({file_path}:{location})")]
    DuplicateVariant {
        /// The name of the enum that has the duplicate variant
        enum_name: String,
        /// The name of the duplicate variant
        name: String,
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Duplicate enum value
    #[error("duplicate variant value '{value}' in enum '{enum_name}' ({file_path}:{location})")]
    DuplicateVariantValue {
        /// The name of the enum that has the duplicate value
        enum_name: String,
        /// The value that was duplicated
        value: String,
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Enumeration has no variants
    #[error("enum '{name}' has no variants ({file_path}:{location})")]
    EmptyEnum {
        /// The name of the empty enum
        name: String,
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Metadata format is not valid
    #[error("metadata format {value} invalid ({file_path}:{location})")]
    InvalidMetadataFormat {
        /// The value that was invalid
        value: String,
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Metadata format is missing
    #[error("metadata format missing ({file_path}:{location})")]
    MissingMetadataFormat {
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Must start with an uppercase letter
    #[error("identifier {name} must be Pascal case ({file_path}:{location})")]
    MustBePascalCase {
        /// The name of the identifier
        name: String,
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
    /// Must start with a lowercase letter
    #[error("identifier {name} must be camel case ({file_path}:{location})")]
    MustBeCamelCase {
        /// The name of the identifier
        name: String,
        /// File path of the schema
        file_path: PathBuf,
        /// [Location] of the parse error
        location: Location,
    },
}

macro_rules! define_error_new {
    ($func:ident, $error:ident) => {
        /// Creates a new [`$error`] error with the given file path and location
        pub fn $func(file_path: &Path, location: &Location) -> Self {
            Self::$error {
                file_path: file_path.to_path_buf(),
                location: location.clone(),
            }
        }
    };
    ($func:ident, $error:ident, $name:ident) => {
        /// Creates a new [`$error`] error with the given name, file path and location
        pub fn $func($name: &str, file_path: &Path, location: &Location) -> Self {
            Self::$error {
                $name: $name.to_string(),
                file_path: file_path.to_path_buf(),
                location: location.clone(),
            }
        }
    };
    ($func:ident, $error:ident, $parent_name:ident, $name:ident) => {
        /// Creates a new [`$error`] error with the given parent name, name, file path and location
        pub fn $func(
            $parent_name: &str,
            $name: &str,
            file_path: &Path,
            location: &Location,
        ) -> Self {
            Self::$error {
                $parent_name: $parent_name.to_string(),
                $name: $name.to_string(),
                file_path: file_path.to_path_buf(),
                location: location.clone(),
            }
        }
    };
}

impl GenoError {
    define_error_new!(new_number_range, NumberRange, content);
    define_error_new!(new_duplicate_type, DuplicateType, type_name);
    define_error_new!(new_duplicate_field, DuplicateField, struct_name, name);
    define_error_new!(new_duplicate_variant, DuplicateVariant, enum_name, name);
    define_error_new!(new_undefined_type, UndefinedType, name);
    define_error_new!(
        new_duplicate_variant_value,
        DuplicateVariantValue,
        enum_name,
        value
    );
    define_error_new!(new_empty_enum, EmptyEnum, name);
    define_error_new!(new_invalid_metadata_format, InvalidMetadataFormat, value);
    define_error_new!(new_missing_metadata_format, MissingMetadataFormat);
    define_error_new!(new_must_be_pascal_case, MustBePascalCase, name);
    define_error_new!(new_must_be_camel_case, MustBeCamelCase, name);
}
