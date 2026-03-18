//! A cross-language schema compiler that generates type definitions and serialization code from a simple, declarative schema language.
//! This crate contains the Abstract Syntaxt Tree (AST), errors and parsing code for the Geno tool.

#![warn(missing_docs)]

/// Namespace containing the AST structures
pub mod ast; // Keep the `ast::` module prefixwhen exporting from this crate
mod error;

pub use error::*;
use pest::{Parser as PestParser, iterators::Pair};
use pest_derive::Parser;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
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

fn remove_first_and_last(value: &str) -> &str {
    let mut chars = value.chars();

    chars.next(); // Consume the first character
    chars.next_back(); // Consume the last character
    chars.as_str() // Return the remaining slice
}

/// A Geno AST builder
pub struct GenoAstBuilder {
    file_path: PathBuf,
}

impl GenoAstBuilder {
    /// Create a new Geno AST builder from a file path.  A file path is required
    /// in order to give meaningful error messages.
    pub fn new(file_path: PathBuf) -> Self {
        GenoAstBuilder { file_path }
    }

    /// Build and validate the AST
    pub fn build(
        &self,
        read_to_string: &impl Fn(&Path) -> std::io::Result<String>,
    ) -> Result<ast::Schema, GenoError> {
        let input = read_to_string(&self.file_path).map_err(|e| GenoError::Io {
            file: self.file_path.clone(),
            error: e,
        })?;
        let mut schema_pairs = match GenoParser::parse(Rule::_schema, &input) {
            Ok(pairs) => pairs,
            Err(err) => {
                return Err(GenoError::Parse {
                    content: err.line().to_string(),
                    file: self.file_path.to_string_lossy().into_owned(),
                    location: Location::from(err.line_col),
                });
            }
        };
        let metadata = self.build_meta_decl(schema_pairs.next().unwrap())?;
        let mut declarations = Vec::new();

        while let Some(pair) = schema_pairs.next() {
            if pair.as_rule() == Rule::EOI {
                break;
            }

            let rule = pair.as_rule();
            match rule {
                Rule::enum_decl => declarations.push(self.build_enum_decl(pair)?),
                Rule::struct_decl => declarations.push(self.build_struct_decl(pair)?),
                Rule::include_stmt => {
                    let string = remove_first_and_last(&pair.into_inner().next().unwrap().as_str());
                    let include_path = self
                        .file_path
                        .parent()
                        .unwrap_or(Path::new(""))
                        .join(string);
                    let builder = GenoAstBuilder::new(include_path);

                    declarations.extend(builder.build(read_to_string)?.declarations);
                }
                _ => {
                    unreachable!(); // Pest problem?
                }
            };
        }

        let schema = ast::Schema {
            metadata,
            declarations,
        };

        schema.validate()?;

        Ok(schema)
    }

    fn build_meta_decl(
        &self,
        pair: Pair<'_, Rule>,
    ) -> Result<HashMap<String, ast::MetadataValue>, GenoError> {
        let mut inner_pairs = pair.into_inner();
        let inner_pair = inner_pairs.next().unwrap();
        let mut metadata = HashMap::new();

        // Parse 'meta_data_entry' pairs
        for entry_pair in inner_pair.into_inner() {
            let mut inner_pairs = entry_pair.into_inner();
            let ident = inner_pairs.next().unwrap().as_str().to_string();
            let value_pair = inner_pairs.next().unwrap();
            let value = match value_pair.as_rule() {
                Rule::string_literal => ast::MetadataValue::String(
                    remove_first_and_last(&value_pair.as_str()).to_string(),
                ),
                Rule::integer_literal => ast::MetadataValue::Integer(
                    self.build_integer_literal(IntegerType::I64, value_pair)?,
                ),
                _ => {
                    unreachable!(); // Pest problem?
                }
            };

            metadata.insert(ident, value);
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
            return Err(GenoError::new_number_range_error(&pair, &self.file_path));
        }

        let digits = if radix == 2 || radix == 16 {
            &s[2..]
        } else {
            s
        };

        match base_type {
            IntegerType::U8 => {
                return Ok(ast::IntegerValue::U8(
                    u8::from_str_radix(digits, radix)
                        .map_err(|_| GenoError::new_number_range_error(&pair, &self.file_path))?,
                ));
            }
            IntegerType::U16 => {
                return Ok(ast::IntegerValue::U16(
                    u16::from_str_radix(digits, radix)
                        .map_err(|_| GenoError::new_number_range_error(&pair, &self.file_path))?,
                ));
            }
            IntegerType::U32 => {
                return Ok(ast::IntegerValue::U32(
                    u32::from_str_radix(digits, radix)
                        .map_err(|_| GenoError::new_number_range_error(&pair, &self.file_path))?,
                ));
            }
            IntegerType::U64 => {
                return Ok(ast::IntegerValue::U64(
                    u64::from_str_radix(digits, radix)
                        .map_err(|_| GenoError::new_number_range_error(&pair, &self.file_path))?,
                ));
            }
            IntegerType::I8 => {
                return Ok(ast::IntegerValue::I8(
                    i8::from_str_radix(digits, radix)
                        .map_err(|_| GenoError::new_number_range_error(&pair, &self.file_path))?,
                ));
            }
            IntegerType::I16 => {
                return Ok(ast::IntegerValue::I16(
                    i16::from_str_radix(digits, radix)
                        .map_err(|_| GenoError::new_number_range_error(&pair, &self.file_path))?,
                ));
            }
            IntegerType::I32 => {
                return Ok(ast::IntegerValue::I32(
                    i32::from_str_radix(digits, radix)
                        .map_err(|_| GenoError::new_number_range_error(&pair, &self.file_path))?,
                ));
            }
            IntegerType::I64 => {
                return Ok(ast::IntegerValue::I64(
                    i64::from_str_radix(digits, radix)
                        .map_err(|_| GenoError::new_number_range_error(&pair, &self.file_path))?,
                ));
            }
        };
    }

    fn build_enum_decl<'a>(
        &self,
        enum_decl_pair: Pair<'a, Rule>,
    ) -> Result<ast::Declaration, GenoError> {
        let mut inner_pairs = enum_decl_pair.into_inner();

        let ident = inner_pairs.next().unwrap().as_str().to_string();
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
        let mut variants: Vec<(String, ast::IntegerValue)> = Vec::new();

        for enum_variant_pair in next_pair.into_inner() {
            let mut variant_inner = enum_variant_pair.into_inner();
            let variant_ident = variant_inner.next().unwrap().as_str().to_string();
            let variant_value =
                self.build_integer_literal(base_type.clone(), variant_inner.next().unwrap())?;

            variants.push((variant_ident, variant_value));
        }

        Ok(ast::Declaration::Enum {
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

        let ident = inner_pairs.next().unwrap().as_str().to_string();
        let next_pair = inner_pairs.next().unwrap();

        // next_pair is now a 'struct_field_list'
        let mut fields: Vec<(String, ast::FieldType)> = Vec::new();

        for struct_field_pair in next_pair.into_inner() {
            let mut struct_field_inner = struct_field_pair.into_inner();
            let field_ident = struct_field_inner.next().unwrap().as_str().to_string();

            fields.push((
                field_ident,
                self.build_field_type(struct_field_inner.next().unwrap())?,
            ));
        }

        // Parse struct declaration
        Ok(ast::Declaration::Struct { ident, fields })
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
                        GenoError::new_number_range_error(&length_pair, &self.file_path)
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
                inner_pair.as_str().to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn happy_path() {
        let input_a = r#"
meta {
    format = 1,
    other = "value",
}
include "b.geno"
// Another comment
struct Type1 {
    alpha: i8,
    alpha_beta: u8,
    alphaBeta: i16,
    a4: u16,
    a5: i32,
    a6: u32,
    a7: i64,
    a8: u64,
    a9: f32,
    a10: f64,
    n1: i8?,
    n2: u8?,
    n3: i16?,
    n4: u16?,
    n5: i16?,
    n6: u16?,
    n7: i32?,
    n8: u32?,
    n9: i64?,
    n10: u64?,
    s1: string,
    s2: string?,
    b1: bool,
    b2: bool?,
    e1: Enum1,
    e2: Enum1?,
    r1: [ string ],
    r2: [ string ]?,
    r3: [ string; 10],
    m1: { string : f64 },
    m2: { string : string },
    m3: { string : bool },
    t1: Type1?,
}"#
        .to_string();
        let input_b = r#"
meta { format = 1 }
enum Enum1: i16 {
    default = -1,
    banana = 0,
    apple = 1,
    orange = 2,
    kiwiFruit = 3,
    pear = 4,
}"#
        .to_string();

        let ast = GenoAstBuilder::new(Path::new("a.geno").to_path_buf())
            .build(&|path: &Path| {
                if path == Path::new("b.geno") {
                    Result::Ok(input_b.clone())
                } else if path == Path::new("a.geno") {
                    Result::Ok(input_a.clone())
                } else {
                    panic!("Bad path: {:?}", path)
                }
            })
            .unwrap();

        let meta_other = ast.metadata.get_key_value("other");

        assert!(meta_other.is_some());
        assert_eq!(
            *meta_other.unwrap().1,
            MetadataValue::String("value".to_string())
        );

        let enum_enum1 = ast
            .declarations
            .iter()
            .find(|d| matches!(d, Declaration::Enum { ident, .. } if ident == "Enum1"));

        assert!(enum_enum1.is_some());

        let struct_type1 = ast
            .declarations
            .iter()
            .find(|d| matches!(d, Declaration::Struct { ident, .. } if ident == "Type1"));

        assert!(struct_type1.is_some());
    }

    #[test]
    fn bad_parse() {
        let input = "meta { ".to_string();
        let result =
            GenoAstBuilder::new(PathBuf::new()).build(&|_path: &Path| Result::Ok(input.clone()));

        match result {
            Err(GenoError::Parse { .. }) => {
                assert!(true);
            }
            _ => {
                panic!("expected GenoError::Parse");
            }
        }
    }

    #[test]
    fn number_range() {
        let input = r#"
meta { format = 1 }
enum A:i16 { v = 0xffffffff, }
"#
        .to_string();

        let result =
            GenoAstBuilder::new(PathBuf::new()).build(&|_path: &Path| Result::Ok(input.clone()));

        match result {
            Err(GenoError::NumberRange { .. }) => {
                assert!(true);
            }
            _ => {
                panic!("expected GenoError::NumberRange");
            }
        }
    }
}
