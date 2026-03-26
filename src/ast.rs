use crate::{Location, case, error::*};
use serde::{Deserialize, Serialize};
use std::{cmp::Eq, collections::HashSet, hash::Hash, path::PathBuf};

/// Enum representing integer types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntegerType {
    /// Signed 8-bit integer
    I8,
    /// Signed 16-bit integer
    I16,
    /// Signed 32-bit integer
    I32,
    /// Signed 64-bit integer
    I64,
    /// Unsigned 8-bit integer
    U8,
    /// Unsigned 16-bit integer
    U16,
    /// Unsigned 32-bit integer
    U32,
    /// Unsigned 64-bit integer
    U64,
}

/// Enum representing integer values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq)]
pub enum IntegerValue {
    /// Signed 8-bit integer value
    I8(i8),
    /// Signed 16-bit integer value
    I16(i16),
    /// Signed 32-bit integer value
    I32(i32),
    /// Signed 64-bit integer value
    I64(i64),
    /// Unsigned 8-bit integer value
    U8(u8),
    /// Unsigned 16-bit integer value
    U16(u16),
    /// Unsigned 32-bit integer value
    U32(u32),
    /// Unsigned 64-bit integer value
    U64(u64),
}

/// Enum representing float values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FloatType {
    /// 32-bit floating-point value
    F32,
    /// 64-bit floating-point value
    F64,
}

/// Enum representing all built-in types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BuiltinType {
    /// Integer types
    Integer(IntegerType),
    /// Float types
    Float(FloatType),
    /// String type
    String,
    /// Bool type
    Bool,
}

/// Identifier type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ident {
    /// The name of the identifier
    pub name: String,
    /// The location of the identifier in the source file
    pub location: Location,
}

impl Ident {
    /// Returns a reference to the name of the identifier
    pub fn as_str(&self) -> &str {
        self.name.as_str()
    }

    /// Returns a reference to the location of the identifier
    pub fn as_location(&self) -> &Location {
        &self.location
    }
}

impl Hash for Ident {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Eq for Ident {}

/// Enum representing all field types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    /// Array type
    Array(Box<FieldType>, Option<usize>, bool),
    /// Map type
    Map(BuiltinType, Box<FieldType>, bool),
    /// Builtin type
    Builtin(BuiltinType, bool),
    /// User-defined type
    UserDefined(Ident, bool),
}

/// Enum representing metadata values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetadataValue {
    /// String value
    String(String),
    /// Integer value
    Integer(IntegerValue),
}

/// Enum representing declarations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Declaration {
    /// Enum declaration
    Enum {
        /// Enum identifier
        ident: Ident,
        /// Enum base integer type
        base_type: IntegerType,
        /// Enum variants
        variants: Vec<(Ident, IntegerValue)>,
    },
    /// Struct declaration
    Struct {
        /// Struct identifier
        ident: Ident,
        /// Struct fields
        fields: Vec<(Ident, FieldType)>,
    },
}

/// Schema declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Schema {
    /// Schema metadata
    pub metadata: Vec<(Ident, MetadataValue)>,
    /// Schema declarations
    pub declarations: Vec<Declaration>,
    /// Nested ASTs
    pub nested_asts: Vec<Schema>,
    /// Source file path of the schema
    pub file_path: PathBuf,
}

impl Schema {
    /// Validate the schema and all nested schemas
    pub fn validate(&self) -> Result<(), GenoError> {
        let mut type_names = HashSet::<String>::new();

        self.first_pass_validate(&mut type_names)?;
        self.second_pass_validate(&type_names)?;

        Ok(())
    }

    fn first_pass_validate(&self, type_names: &mut HashSet<String>) -> Result<(), GenoError> {
        self.validate_metadata_format()?;

        // Check for duplicate type definitions and duplicate fields/variants within each declaration
        for decl in &self.declarations {
            match decl {
                Declaration::Enum {
                    ident, variants, ..
                } => {
                    // Ensure that the ident starts with an uppercase letter
                    if !case::is_first_char_uppercase(ident.as_str()) {
                        return Err(GenoError::new_must_start_with_uppercase(
                            ident.as_str(),
                            &self.file_path,
                            ident.as_location(),
                        ));
                    }

                    // Don't allow enum with no variants
                    if variants.is_empty() {
                        return Err(GenoError::new_empty_enum(
                            ident.as_str(),
                            &self.file_path,
                            ident.as_location(),
                        ));
                    }

                    let mut variant_names = HashSet::new();
                    let mut variant_values = HashSet::new();

                    for (variant_name, variant_value) in variants {
                        // Ensure that the variant name starts with a lowercase letter
                        if !case::is_first_char_lowercase(variant_name.as_str()) {
                            return Err(GenoError::new_must_start_with_lowercase(
                                variant_name.as_str(),
                                &self.file_path,
                                variant_name.as_location(),
                            ));
                        }

                        // Check for duplicate variant names
                        if !variant_names.insert(variant_name.as_str()) {
                            return Err(GenoError::new_duplicate_variant(
                                ident.as_str(),
                                variant_name.as_str(),
                                &self.file_path,
                                variant_name.as_location(),
                            ));
                        }

                        let value_str = Self::integer_value_str(variant_value);

                        // Check for duplicate variant values
                        if !variant_values.insert(value_str.clone()) {
                            return Err(GenoError::new_duplicate_variant_value(
                                variant_name.as_str(),
                                &value_str,
                                &self.file_path,
                                variant_name.as_location(),
                            ));
                        }
                    }

                    // Record type name, checking for duplicates
                    if !type_names.insert(ident.as_str().to_string()) {
                        return Err(GenoError::new_duplicate_type(
                            ident.as_str(),
                            &self.file_path,
                            ident.as_location(),
                        ));
                    }
                }

                Declaration::Struct { ident, fields } => {
                    // Ensure that the ident starts with an uppercase letter
                    if !case::is_first_char_uppercase(ident.as_str()) {
                        return Err(GenoError::new_must_start_with_uppercase(
                            ident.as_str(),
                            &self.file_path,
                            ident.as_location(),
                        ));
                    }

                    let mut field_names = HashSet::new();

                    for (file_ident, _) in fields {
                        // Ensure that the field name starts with a lowercase letter
                        if !case::is_first_char_lowercase(file_ident.as_str()) {
                            return Err(GenoError::new_must_start_with_lowercase(
                                file_ident.as_str(),
                                &self.file_path,
                                file_ident.as_location(),
                            ));
                        }

                        // Ensure that the field name is unique
                        if !field_names.insert(file_ident.as_str()) {
                            return Err(GenoError::new_duplicate_field(
                                ident.as_str(),
                                file_ident.as_str(),
                                &self.file_path,
                                file_ident.as_location(),
                            ));
                        }
                    }

                    // Record type name, checking for duplicates
                    if !type_names.insert(ident.as_str().to_string()) {
                        return Err(GenoError::new_duplicate_type(
                            ident.as_str(),
                            &self.file_path,
                            ident.as_location(),
                        ));
                    }
                }
            }
        }

        // Perform first pass on nested ASTs
        for ast in &self.nested_asts {
            ast.first_pass_validate(type_names)?;
        }

        Ok(())
    }

    fn second_pass_validate(&self, type_names: &HashSet<String>) -> Result<(), GenoError> {
        // Check for undefined types in structs
        for decl in &self.declarations {
            if let Declaration::Struct { fields, .. } = decl {
                for (_, field_type) in fields {
                    self.check_for_undefined_types(field_type, &type_names)?;
                }
            }
        }

        // Perform first pass on nested ASTs
        for ast in &self.nested_asts {
            ast.second_pass_validate(type_names)?;
        }

        Ok(())
    }

    fn validate_metadata_format(&self) -> Result<(), GenoError> {
        const EXPECTED_FORMAT: i64 = 1;
        let actual_format = self.metadata.iter().find(|(k, _)| k.name == "format");

        if let Some(actual_format) = actual_format {
            if let MetadataValue::Integer(IntegerValue::I64(value)) = &actual_format.1 {
                if *value != EXPECTED_FORMAT {
                    return Err(GenoError::new_invalid_metadata_format(
                        actual_format.0.as_str(),
                        &self.file_path,
                        &actual_format.0.as_location(),
                    ));
                }
            }
        } else {
            return Err(GenoError::new_missing_metadata_format(
                &self.file_path,
                &Location { line: 1, column: 1 },
            ));
        }

        Ok(())
    }

    fn integer_value_str(v: &IntegerValue) -> String {
        match v {
            IntegerValue::I8(n) => n.to_string(),
            IntegerValue::I16(n) => n.to_string(),
            IntegerValue::I32(n) => n.to_string(),
            IntegerValue::I64(n) => n.to_string(),
            IntegerValue::U8(n) => n.to_string(),
            IntegerValue::U16(n) => n.to_string(),
            IntegerValue::U32(n) => n.to_string(),
            IntegerValue::U64(n) => n.to_string(),
        }
    }

    fn check_for_undefined_types(
        &self,
        field_type: &FieldType,
        type_names: &HashSet<String>,
    ) -> Result<(), GenoError> {
        match field_type {
            FieldType::UserDefined(ident, _) => {
                if !type_names.contains(ident.as_str()) {
                    return Err(GenoError::new_undefined_type(
                        ident.as_str(),
                        &self.file_path,
                        ident.as_location(),
                    ));
                }
            }
            FieldType::Array(inner, _, _) => {
                self.check_for_undefined_types(inner, type_names)?;
            }
            FieldType::Map(_, value_type, _) => {
                self.check_for_undefined_types(value_type, type_names)?;
            }
            FieldType::Builtin(_, _) => {}
        }
        Ok(())
    }

    /// Flattens the all nested AST declarations
    pub fn flatten_decls<'a>(&'a self) -> Vec<&'a Declaration> {
        let mut declarations = Vec::new();

        self.flatten_nested(&mut declarations);

        declarations
    }

    fn flatten_nested<'a>(&'a self, declarations: &mut Vec<&'a Declaration>) {
        for decl in self.declarations.iter() {
            declarations.push(&decl);
        }

        for ast in &self.nested_asts {
            ast.flatten_nested(declarations);
        }
    }
}
