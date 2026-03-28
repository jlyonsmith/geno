//! A cross-language schema compiler that generates type definitions and serialization code from a simple, declarative schema language.
//! This crate contains the Abstract Syntaxt Tree (AST), errors and parsing code for the Geno tool.

#![warn(missing_docs)]

/// Namespace containing the AST structures
pub mod ast; // Keep the `ast::` module prefixwhen exporting from this crate
/// Namespace containing case conversion utilities
pub mod case; // Keep the `case::` module prefix when exporting from this crate

mod ast_builder;
mod error;
mod location;

pub use ast_builder::*;
pub use error::*;
pub use location::*;

#[cfg(test)]
mod tests {
    use crate::{ast, ast_builder::*, error::*};
    use std::path::Path;

    #[test]
    fn happy_path() {
        let input_a = r#"
#[format = 1, other = "value",]

include "b.geno"
// Another comment
struct Type1 {
    alpha: i8,
    beta: u8,
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
#[format = 1]
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
            .expect("failed to initialize ast builder")
            .build(&|path: &Path| {
                if path.ends_with("b.geno") {
                    Result::Ok(input_b.clone())
                } else if path.ends_with("a.geno") {
                    Result::Ok(input_a.clone())
                } else {
                    panic!("Bad path: {:?}", path)
                }
            })
            .unwrap();

        ast.validate().unwrap();

        let meta_other = ast.metadata.iter().find(|(ident, _)| ident.name == "other");

        assert!(meta_other.is_some());
        assert_eq!(meta_other.unwrap().0.as_str(), "other");

        let decls = ast.flatten_decls();

        let enum_enum1 = decls
            .iter()
            .find(|d| matches!(d, ast::Declaration::Enum { ident, .. } if ident.name == "Enum1"));

        assert!(enum_enum1.is_some());

        let struct_type1 = decls
            .iter()
            .find(|d| matches!(d, ast::Declaration::Struct { ident, .. } if ident.name == "Type1"));

        assert!(struct_type1.is_some());
    }

    #[test]
    fn bad_parse() {
        let input = "meta { ".to_string();
        let result = GenoAstBuilder::new(Path::new("/a.geno").to_path_buf())
            .expect("failed to initialize ast builder")
            .build(&|_path: &Path| Result::Ok(input.clone()));

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
#[ format = 1 ]
enum A:i16 { v = 0xffffffff, }
"#
        .to_string();

        let result = GenoAstBuilder::new(Path::new("a.geno").to_path_buf())
            .expect("failed to initialize ast builder")
            .build(&|_path: &Path| Result::Ok(input.clone()));

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
