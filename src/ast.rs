use crate::{Location, ParserError, case};
use serde::{Deserialize, Serialize};
use std::{cmp::Eq, collections::HashSet, hash::Hash, path::PathBuf};
use topo_sort::TopoSort;

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

/// Enum representing all field types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    /// Array type
    Array(Box<FieldType>, Option<IntegerValue>, bool),
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
    /// Boolean value; present when the value is `true`
    Boolean,
    /// String value
    String(String),
    /// Integer value
    Integer(IntegerValue),
}

/// Enum representing elements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Element {
    /// Enum declaration
    Enum {
        /// Enum attributes
        attributes: Attributes,
        /// Enum identifier
        ident: Ident,
        /// Enum base integer type
        base_type: IntegerType,
        /// Enum variants
        variants: Vec<(Attributes, Ident, IntegerValue)>,
    },
    /// Struct declaration
    Struct {
        /// Struct attributes
        attributes: Attributes,
        /// Struct identifier
        ident: Ident,
        /// Struct fields
        fields: Vec<(Attributes, Ident, FieldType)>,
    },
    /// Include directive
    Include {
        /// Include attributes
        attributes: Attributes,
        /// Include path
        schema: Box<Schema>,
    },
}

/// A list of attributes associated with a declaration
pub type Attributes = Vec<(Ident, MetadataValue)>;

/// Schema declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Schema {
    /// Schema metadata
    pub attributes: Attributes,
    /// Schema elements
    pub elements: Vec<Element>,
    /// Source file path of the schema
    pub file_path: PathBuf,
}

impl Schema {
    /// Validate the schema and all nested schemas
    pub fn validate(&self) -> Result<(), ParserError> {
        let mut type_names = HashSet::<String>::new();
        let mut topo_sort = TopoSort::<String>::new();

        self.first_pass_validate(&mut type_names)?;
        self.second_pass_validate(&type_names, &mut topo_sort)?;

        Ok(())
    }

    fn first_pass_validate(&self, type_names: &mut HashSet<String>) -> Result<(), ParserError> {
        self.validate_metadata_format()?;

        // Check for duplicate type definitions and duplicate fields/variants within each declaration
        for decl in &self.elements {
            match decl {
                Element::Enum {
                    ident, variants, ..
                } => {
                    // Ensure that the ident is PascalCase
                    if !case::is_pascal_case(ident.as_str()) {
                        return Err(ParserError::MustBePascalCase {
                            name: ident.as_str().to_string(),
                            file_path: self.file_path.clone(),
                            location: ident.as_location().clone(),
                        });
                    }

                    // Don't allow enum with no variants
                    if variants.is_empty() {
                        return Err(ParserError::EmptyEnum {
                            name: ident.as_str().to_string(),
                            file_path: self.file_path.clone(),
                            location: ident.as_location().clone(),
                        });
                    }

                    let mut variant_names = HashSet::new();
                    let mut variant_values = HashSet::new();

                    for (_, varinat_ident, variant_value) in variants {
                        // Ensure that the variant name is camelCase
                        if !case::is_camel_case(varinat_ident.as_str()) {
                            return Err(ParserError::MustBeCamelCase {
                                name: varinat_ident.as_str().to_string(),
                                file_path: self.file_path.clone(),
                                location: varinat_ident.as_location().clone(),
                            });
                        }

                        // Check for duplicate variant names
                        if !variant_names.insert(varinat_ident.as_str()) {
                            return Err(ParserError::DuplicateVariant {
                                enum_name: ident.as_str().to_string(),
                                name: varinat_ident.as_str().to_string(),
                                file_path: self.file_path.clone(),
                                location: varinat_ident.as_location().clone(),
                            });
                        }

                        let value_str = Self::integer_value_str(variant_value);

                        // Check for duplicate variant values
                        if !variant_values.insert(value_str.clone()) {
                            return Err(ParserError::DuplicateVariantValue {
                                enum_name: ident.as_str().to_string(),
                                value: value_str,
                                file_path: self.file_path.clone(),
                                location: varinat_ident.as_location().clone(),
                            });
                        }
                    }

                    // Record type name, checking for duplicates
                    if !type_names.insert(ident.as_str().to_string()) {
                        return Err(ParserError::DuplicateType {
                            type_name: ident.as_str().to_string(),
                            file_path: self.file_path.clone(),
                            location: ident.as_location().clone(),
                        });
                    }
                }

                Element::Struct {
                    attributes: _,
                    ident,
                    fields,
                } => {
                    // Ensure that the ident is PascalCase
                    if !case::is_pascal_case(ident.as_str()) {
                        return Err(ParserError::MustBePascalCase {
                            name: ident.as_str().to_string(),
                            file_path: self.file_path.clone(),
                            location: ident.as_location().clone(),
                        });
                    }

                    let mut field_names = HashSet::new();

                    for (_, file_ident, _) in fields {
                        // Ensure that the field name is camelCase
                        if !case::is_camel_case(file_ident.as_str()) {
                            return Err(ParserError::MustBeCamelCase {
                                name: file_ident.as_str().to_string(),
                                file_path: self.file_path.clone(),
                                location: file_ident.as_location().clone(),
                            });
                        }

                        // Ensure that the field name is unique
                        if !field_names.insert(file_ident.as_str()) {
                            return Err(ParserError::DuplicateField {
                                struct_name: ident.as_str().to_string(),
                                name: file_ident.as_str().to_string(),
                                file_path: self.file_path.clone(),
                                location: file_ident.as_location().clone(),
                            });
                        }
                    }

                    // Record type name, checking for duplicates
                    if !type_names.insert(ident.as_str().to_string()) {
                        return Err(ParserError::DuplicateType {
                            type_name: ident.as_str().to_string(),
                            file_path: self.file_path.clone(),
                            location: ident.as_location().clone(),
                        });
                    }
                }

                Element::Include {
                    attributes: _,
                    schema,
                } => {
                    schema.first_pass_validate(type_names)?;
                }
            }
        }

        Ok(())
    }

    fn second_pass_validate(
        &self,
        type_names: &HashSet<String>,
        topo_sort: &mut TopoSort<String>,
    ) -> Result<(), ParserError> {
        // Check for undefined types in structs
        for element in &self.elements {
            match element {
                Element::Struct {
                    fields,
                    ident: struct_ident,
                    ..
                } => {
                    for (_, ident, field_type) in fields {
                        self.check_for_undefined_types(field_type, &type_names)?;

                        if self.has_struct_cycle(struct_ident.name.as_str(), field_type, topo_sort)
                        {
                            return Err(ParserError::StructCycle {
                                field: ident.name.clone(),
                                location: ident.location.clone(),
                                file_path: self.file_path.clone(),
                            });
                        }
                    }
                }
                Element::Include {
                    attributes: _,
                    schema,
                } => {
                    schema.second_pass_validate(type_names, topo_sort)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn validate_metadata_format(&self) -> Result<(), ParserError> {
        const EXPECTED_FORMAT: i64 = 1;
        let actual_format = self.attributes.iter().find(|(k, _)| k.name == "format");

        if let Some(actual_format) = actual_format {
            if let MetadataValue::Integer(IntegerValue::I64(value)) = &actual_format.1 {
                if *value != EXPECTED_FORMAT {
                    return Err(ParserError::InvalidMetadataFormat {
                        value: actual_format.0.as_str().to_string(),
                        file_path: self.file_path.clone(),
                        location: actual_format.0.as_location().clone(),
                    });
                }
            }
        } else {
            return Err(ParserError::MissingMetadataFormat {
                file_path: self.file_path.clone(),
                location: Location { line: 1, column: 1 },
            });
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
    ) -> Result<(), ParserError> {
        match field_type {
            FieldType::UserDefined(ident, _) => {
                if !type_names.contains(ident.as_str()) {
                    return Err(ParserError::UndefinedType {
                        name: ident.as_str().to_string(),
                        file_path: self.file_path.clone(),
                        location: ident.as_location().clone(),
                    });
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

    fn has_struct_cycle(
        &self,
        parent_name: &str,
        field_type: &FieldType,
        topo_sort: &mut TopoSort<String>,
    ) -> bool {
        match field_type {
            FieldType::Array(array_type, _, nullable) => {
                !nullable && self.has_struct_cycle(parent_name, array_type, topo_sort)
            }
            FieldType::Map(_, value_type, nullable) => {
                !nullable && self.has_struct_cycle(parent_name, value_type, topo_sort)
            }
            FieldType::UserDefined(ident, nullable) => {
                if !nullable {
                    topo_sort.insert_from_slice(parent_name.to_string(), &[ident.name.clone()]);
                    topo_sort.cycle_detected()
                } else {
                    false
                }
            }
            FieldType::Builtin(_, _) => false,
        }
    }

    /// Flattens all nested AST elementarations
    pub fn flatten_elements<'a>(&'a self) -> Vec<&'a Element> {
        let mut elements = Vec::new();

        self.flatten_nested_elements(&mut elements);

        elements
    }

    fn flatten_nested_elements<'a>(&'a self, elements: &mut Vec<&'a Element>) {
        for element in &self.elements {
            match element {
                Element::Include {
                    attributes: _,
                    schema,
                } => {
                    schema.flatten_nested_elements(elements);
                }
                _ => {
                    elements.push(&element);
                }
            }
        }
    }
}
