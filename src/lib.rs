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
mod parser;
mod tokenizer;

pub use ast_builder::*;
pub use error::*;
pub use location::*;
pub use parser::*;
pub(crate) use tokenizer::*;
