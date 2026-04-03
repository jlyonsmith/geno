use crate::{Location, ast, error::*};
use pest::{Parser as PestParser, iterators::Pair};
use pest_derive::Parser;
use std::{
    collections::HashSet,
    path::{self, Path, PathBuf},
};

// Put the Pest parser in a private module to suppress doc warnings
// See [Issue #326](https://github.com/pest-parser/pest/issues/326)
mod parser {
    use super::*;

    #[derive(Parser)]
    #[grammar = "geno.pest"]
    pub struct GenoParser;
}

use crate::ast::IntegerType;
use parser::{GenoParser, Rule};

fn remove_quotes(value: &str) -> &str {
    let mut chars = value.chars();

    chars.next(); // Consume the first character
    chars.next_back(); // Consume the last character
    chars.as_str() // Return the remaining slice
}

impl From<Pair<'_, Rule>> for ast::Ident {
    fn from(pair: Pair<'_, Rule>) -> Self {
        ast::Ident {
            name: pair.as_str().to_string(),
            location: Location::from(&pair.as_span()),
        }
    }
}

/// A Geno AST builder
pub struct GenoAstBuilder {
    file_path: PathBuf,
}

impl GenoAstBuilder {
    /// Create a new Geno AST builder from a file path.  A file path is required
    /// in order to give meaningful error messages.
    pub fn new(file_path: PathBuf) -> Result<Self, std::io::Error> {
        Ok(GenoAstBuilder {
            file_path: path::absolute(file_path.clone())?,
        })
    }

    /// Build and validate the AST
    pub fn build(
        &self,
        read_to_string: &impl Fn(&Path) -> std::io::Result<String>,
    ) -> Result<ast::Schema, GenoError> {
        let mut file_paths = HashSet::<PathBuf>::new();

        file_paths.insert(self.file_path.clone());

        let input = read_to_string(&self.file_path).map_err(|e| GenoError::Io {
            file_path: self.file_path.clone(),
            error: e,
        })?;

        let mut schema_pairs = match GenoParser::parse(Rule::_schema, &input) {
            Ok(pairs) => pairs,
            Err(err) => {
                return Err(GenoError::Parse {
                    content: err.line().to_string(),
                    file_path: self.file_path.clone(),
                    location: Location::from(err.line_col),
                });
            }
        };
        let attributes = self.build_attributes(schema_pairs.next().unwrap())?;
        let mut declarations = Vec::new();
        let mut includes = Vec::new();

        while let Some(pair) = schema_pairs.next() {
            if pair.as_rule() == Rule::EOI {
                break;
            }

            let rule = pair.as_rule();
            match rule {
                Rule::enum_decl => declarations.push(self.build_enum_decl(pair)?),
                Rule::struct_decl => declarations.push(self.build_struct_decl(pair)?),
                Rule::include_stmt => {
                    let nested_file_path =
                        Path::new(remove_quotes(&pair.into_inner().next().unwrap().as_str()));
                    let include_path = self
                        .file_path
                        .parent()
                        .unwrap_or(Path::new("/"))
                        .join(nested_file_path);
                    let builder = GenoAstBuilder::new(include_path).map_err(|e| GenoError::Io {
                        file_path: self.file_path.clone(),
                        error: e,
                    })?;

                    if !file_paths.contains(&builder.file_path) {
                        file_paths.insert(builder.file_path.clone());
                        includes.push((vec![], builder.build(read_to_string)?));
                    }
                }
                _ => {
                    unreachable!(); // Pest problem?
                }
            };
        }

        Ok(ast::Schema {
            attributes,
            declarations,
            includes,
            file_path: self.file_path.clone(),
        })
    }

    fn build_attributes(
        &self,
        pair: Pair<'_, Rule>,
    ) -> Result<Vec<(ast::Ident, ast::MetadataValue)>, GenoError> {
        let mut inner_pairs = pair.into_inner();
        let inner_pair = inner_pairs.next().unwrap();
        let mut metadata: Vec<(ast::Ident, ast::MetadataValue)> = Vec::new();

        // Parse 'attribute_entry' pairs
        for entry_pair in inner_pair.into_inner() {
            let mut inner_pairs = entry_pair.into_inner();
            let ident_pair = inner_pairs.next().unwrap();
            let ident = ast::Ident::from(ident_pair);
            let value_pair = inner_pairs.next().unwrap();
            let value = match value_pair.as_rule() {
                Rule::string_literal => {
                    ast::MetadataValue::String(remove_quotes(&value_pair.as_str()).to_string())
                }
                Rule::integer_literal => ast::MetadataValue::Integer(
                    self.build_integer_literal(IntegerType::I64, value_pair)?,
                ),
                _ => {
                    unreachable!(); // Pest problem?
                }
            };

            metadata.push((ident, value));
        }

        Ok(metadata)
    }

    fn build_integer_type(&self, pair: Pair<'_, Rule>) -> Result<ast::IntegerType, GenoError> {
        let s = pair.as_str();

        match s {
            "i8" => Ok(ast::IntegerType::I8),
            "u8" => Ok(ast::IntegerType::U8),
            "i16" => Ok(ast::IntegerType::I16),
            "u16" => Ok(ast::IntegerType::U16),
            "i32" => Ok(ast::IntegerType::I32),
            "u32" => Ok(ast::IntegerType::U32),
            "i64" => Ok(ast::IntegerType::I64),
            "u64" => Ok(ast::IntegerType::U64),
            _ => unreachable!(),
        }
    }

    fn build_integer_literal(
        &self,
        base_type: IntegerType,
        pair: Pair<'_, Rule>,
    ) -> Result<ast::IntegerValue, GenoError> {
        let s = pair.as_str();
        let radix = if s.starts_with("0b") {
            2
        } else if s.starts_with("0x") {
            16
        } else {
            10
        };
        let is_signed = matches!(
            base_type,
            IntegerType::I8 | IntegerType::I16 | IntegerType::I32 | IntegerType::I64
        );

        if is_signed && (radix == 16 || radix == 2) {
            return Err(GenoError::new_number_range(
                &pair.as_str(),
                &self.file_path,
                &Location::from(&pair.as_span()),
            ));
        }

        let digits = if radix == 2 || radix == 16 {
            &s[2..]
        } else {
            s
        };

        match base_type {
            IntegerType::U8 => {
                return Ok(ast::IntegerValue::U8(
                    u8::from_str_radix(digits, radix).map_err(|_| {
                        GenoError::new_number_range(
                            pair.as_str(),
                            &self.file_path,
                            &Location::from(&pair.as_span()),
                        )
                    })?,
                ));
            }
            IntegerType::U16 => {
                return Ok(ast::IntegerValue::U16(
                    u16::from_str_radix(digits, radix).map_err(|_| {
                        GenoError::new_number_range(
                            pair.as_str(),
                            &self.file_path,
                            &Location::from(&pair.as_span()),
                        )
                    })?,
                ));
            }
            IntegerType::U32 => {
                return Ok(ast::IntegerValue::U32(
                    u32::from_str_radix(digits, radix).map_err(|_| {
                        GenoError::new_number_range(
                            pair.as_str(),
                            &self.file_path,
                            &Location::from(&pair.as_span()),
                        )
                    })?,
                ));
            }
            IntegerType::U64 => {
                return Ok(ast::IntegerValue::U64(
                    u64::from_str_radix(digits, radix).map_err(|_| {
                        GenoError::new_number_range(
                            pair.as_str(),
                            &self.file_path,
                            &Location::from(&pair.as_span()),
                        )
                    })?,
                ));
            }
            IntegerType::I8 => {
                return Ok(ast::IntegerValue::I8(
                    i8::from_str_radix(digits, radix).map_err(|_| {
                        GenoError::new_number_range(
                            pair.as_str(),
                            &self.file_path,
                            &Location::from(&pair.as_span()),
                        )
                    })?,
                ));
            }
            IntegerType::I16 => {
                return Ok(ast::IntegerValue::I16(
                    i16::from_str_radix(digits, radix).map_err(|_| {
                        GenoError::new_number_range(
                            pair.as_str(),
                            &self.file_path,
                            &Location::from(&pair.as_span()),
                        )
                    })?,
                ));
            }
            IntegerType::I32 => {
                return Ok(ast::IntegerValue::I32(
                    i32::from_str_radix(digits, radix).map_err(|_| {
                        GenoError::new_number_range(
                            pair.as_str(),
                            &self.file_path,
                            &Location::from(&pair.as_span()),
                        )
                    })?,
                ));
            }
            IntegerType::I64 => {
                return Ok(ast::IntegerValue::I64(
                    i64::from_str_radix(digits, radix).map_err(|_| {
                        GenoError::new_number_range(
                            pair.as_str(),
                            &self.file_path,
                            &Location::from(&pair.as_span()),
                        )
                    })?,
                ));
            }
        };
    }

    fn build_enum_decl<'a>(
        &self,
        enum_decl_pair: Pair<'a, Rule>,
    ) -> Result<ast::Declaration, GenoError> {
        let mut inner_pairs = enum_decl_pair.into_inner();

        let ident_pair = inner_pairs.next().unwrap();
        let ident = ast::Ident::from(ident_pair);
        let mut next_pair = inner_pairs.next().unwrap();
        let base_type;

        if next_pair.as_rule() == Rule::integer_type {
            base_type = self.build_integer_type(next_pair)?;
            next_pair = inner_pairs.next().unwrap();
        } else {
            // No base type specified, default to i32
            base_type = ast::IntegerType::I32
        };

        // next_pair is now an 'enum_variant_list'
        let mut variants = Vec::new();

        for enum_variant_pair in next_pair.into_inner() {
            let mut variant_inner = enum_variant_pair.into_inner();
            let variant_ident_pair = variant_inner.next().unwrap();
            let variant_ident = ast::Ident::from(variant_ident_pair);
            let variant_value =
                self.build_integer_literal(base_type.clone(), variant_inner.next().unwrap())?;

            variants.push((vec![], variant_ident, variant_value));
        }

        Ok(ast::Declaration::Enum {
            attributes: vec![],
            ident,
            base_type,
            variants,
        })
    }

    fn build_struct_decl<'a>(
        &self,
        struct_decl_pair: Pair<'a, Rule>,
    ) -> Result<ast::Declaration, GenoError> {
        let mut inner_pairs = struct_decl_pair.into_inner();

        let ident_pair = inner_pairs.next().unwrap();
        let ident = ast::Ident::from(ident_pair);
        let next_pair = inner_pairs.next().unwrap();

        // next_pair is now a 'struct_field_list'
        let mut fields = Vec::new();

        for struct_field_pair in next_pair.into_inner() {
            let mut struct_field_inner = struct_field_pair.into_inner();
            let field_ident_pair = struct_field_inner.next().unwrap();
            let field_ident = ast::Ident::from(field_ident_pair);

            fields.push((
                vec![],
                field_ident,
                self.build_field_type(struct_field_inner.next().unwrap())?,
            ));
        }

        // Parse struct declaration
        Ok(ast::Declaration::Struct {
            attributes: vec![],
            ident,
            fields,
        })
    }

    fn build_field_type<'a>(&self, pair: Pair<'a, Rule>) -> Result<ast::FieldType, GenoError> {
        let mut inner_pairs = pair.into_inner();
        let inner_pair = inner_pairs.next().unwrap();

        let nullable = if let Some(nullable_pair) = inner_pairs.peek() {
            if nullable_pair.as_rule() == Rule::nullable {
                true
            } else {
                false
            }
        } else {
            false
        };

        match inner_pair.as_rule() {
            Rule::array_type => {
                let mut inner_pairs = inner_pair.into_inner();
                let element_type_pair = inner_pairs.next().unwrap();
                let length = if let Some(length_pair) = inner_pairs.next() {
                    Some(length_pair.as_str().parse::<usize>().map_err(|_| {
                        GenoError::new_number_range(
                            length_pair.as_str(),
                            &self.file_path,
                            &Location::from(&length_pair.as_span()),
                        )
                    })?)
                } else {
                    None
                };
                Ok(ast::FieldType::Array(
                    Box::new(self.build_field_type(element_type_pair)?),
                    length,
                    nullable,
                ))
            }
            Rule::map_type => {
                let mut inner_pairs = inner_pair.into_inner();
                let key_type_pair = inner_pairs.next().unwrap();
                let value_type_pair = inner_pairs.next().unwrap();

                Ok(ast::FieldType::Map(
                    self.build_builtin_type(key_type_pair)?,
                    Box::new(self.build_field_type(value_type_pair)?),
                    nullable,
                ))
            }
            Rule::builtin_type => Ok(ast::FieldType::Builtin(
                self.build_builtin_type(inner_pair)?,
                nullable,
            )),
            Rule::identifier => Ok(ast::FieldType::UserDefined(
                ast::Ident::from(inner_pair),
                nullable,
            )),
            _ => unreachable!(),
        }
    }

    fn build_builtin_type(&self, pair: Pair<'_, Rule>) -> Result<ast::BuiltinType, GenoError> {
        let mut inner_pairs = pair.into_inner();
        let inner_pair = inner_pairs.next().unwrap();

        match inner_pair.as_rule() {
            Rule::integer_type => self
                .build_integer_type(inner_pair)
                .map(ast::BuiltinType::Integer),
            Rule::float_type => {
                let s = inner_pair.as_str();
                match s {
                    "f32" => Ok(ast::BuiltinType::Float(ast::FloatType::F32)),
                    "f64" => Ok(ast::BuiltinType::Float(ast::FloatType::F64)),
                    _ => unreachable!(),
                }
            }
            Rule::string_type => Ok(ast::BuiltinType::String),
            Rule::bool_type => Ok(ast::BuiltinType::Bool),
            _ => unreachable!(),
        }
    }
}
