//! Lexical tokenizer for the Geno schema language.

use crate::{
    Location, Token, TokenKind, Tokenizer,
    ast::{self, Attributes},
};
use anyhow::anyhow;
use fallible_iterator::FallibleIterator;
use std::{
    cell::RefCell,
    error::Error,
    fmt,
    path::{Path, PathBuf},
    rc::Rc,
};

/// Error produced by the parser.
#[derive(Debug, PartialEq)]
pub enum ParserError {
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
            _ => write!(f, "unknown"),
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

impl Error for ParserError {}

/// Trait for resolving file paths and reading file contents.
pub trait FileResolver {
    /// Resolves a path string to a [`PathBuf`].
    fn resolve_path(&mut self, path: &str) -> Result<PathBuf, ResolverError>;
    /// Reads the contents of a file at the given path as a string.
    fn read_to_string(&self, path: &Path) -> Result<String, ResolverError>;
}

/// Error type for file resolver operations.
#[derive(Debug)]
pub enum ResolverError {
    /// A duplicate include path was encountered.
    DuplicateInclude(PathBuf),
    /// The file was not found.
    FileNotFound(PathBuf),
    /// An IO error occurred.
    Io(std::io::Error),
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO failure: {}", e),
            Self::DuplicateInclude(p) => write!(f, "Duplicate include: {}", p.display()),
            Self::FileNotFound(p) => write!(f, "File not found: {}", p.display()),
        }
    }
}

impl Error for ResolverError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(e) => Some(e), // Return the underlying IO error
            Self::DuplicateInclude(_) => None,
            Self::FileNotFound(_) => None,
        }
    }
}

/// A parser for Geno schemas
pub struct Parser {
    file_path: PathBuf,
    resolver: Rc<RefCell<dyn FileResolver>>,
}

impl Parser {
    /// Creates a new parser for the given file path, using the given resolver to load the file contents.
    pub fn new(file_path: PathBuf, resolver: Rc<RefCell<dyn FileResolver>>) -> Self {
        Self {
            file_path,
            resolver: Rc::clone(&resolver),
        }
    }

    /// Parses the schema
    pub fn parse(&self) -> anyhow::Result<ast::Schema> {
        let mut schema_attrs: Option<ast::Attributes> = None;
        let mut attrs: Option<ast::Attributes> = None;
        let mut includes: Vec<(ast::Attributes, ast::Schema)> = vec![];
        let input = self.resolver.borrow().read_to_string(&self.file_path)?;
        let mut tokenizer = Tokenizer::new(&input);
        let mut declarations: Vec<ast::Declaration> = vec![];

        while let Some(token) = tokenizer.next()? {
            match token.kind {
                TokenKind::Comment(_) => {}
                TokenKind::SchemaAttrOpen => {
                    if schema_attrs.is_some() {
                        return Err(anyhow!(ParserError::MultipleSchemaAttributes {
                            file_path: self.file_path.clone(),
                            location: token.location
                        }));
                    }

                    schema_attrs = Some(self.parse_attributes(&mut tokenizer)?);
                }
                TokenKind::AttrOpen => {
                    if attrs.is_some() {
                        return Err(anyhow!(ParserError::MultipleAttributes {
                            file_path: self.file_path.clone(),
                            location: token.location
                        }));
                    }

                    attrs = Some(self.parse_attributes(&mut tokenizer)?);
                }
                TokenKind::Struct => {
                    let struct_decl = self.parse_struct(&mut tokenizer, attrs.take())?;

                    declarations.push(struct_decl);
                }
                TokenKind::Enum => {
                    let enum_decl = self.parse_enum(&mut tokenizer, attrs.take())?;

                    declarations.push(enum_decl);
                }
                TokenKind::Include => {
                    let include = (
                        attrs.take().unwrap_or(vec![]),
                        self.parse_include(&mut tokenizer)?,
                    );

                    includes.push(include);
                }
                _ => {
                    return Err(anyhow!(ParserError::UnexpectedToken {
                        file_path: self.file_path.clone(),
                        token
                    }));
                }
            }
        }

        Ok(ast::Schema {
            attributes: schema_attrs.unwrap_or(vec![]),
            declarations: vec![],
            includes,
            file_path: self.file_path.clone(),
        })
    }

    fn next_token(&self, tokenizer: &mut Tokenizer) -> anyhow::Result<Token> {
        match tokenizer.next()? {
            Some(token) => Ok(token),
            None => {
                return Err(anyhow!(ParserError::UnexpectedEndOfFile {
                    file_path: self.file_path.clone(),
                }));
            }
        }
    }

    fn parse_attributes(&self, tokenizer: &mut Tokenizer) -> anyhow::Result<ast::Attributes> {
        let mut attrs: Attributes = vec![];
        let mut accept_comma = false;

        loop {
            let token = self.next_token(tokenizer)?;

            match token.kind {
                TokenKind::Comment(_) => {}
                TokenKind::Comma => {
                    if !accept_comma {
                        return Err(anyhow!(ParserError::UnexpectedComma {
                            file_path: self.file_path.clone(),
                            location: token.location,
                        }));
                    }
                    accept_comma = false;
                }
                TokenKind::BracketClose => {
                    return Ok(attrs);
                }
                TokenKind::Ident(name) => {
                    let value = match tokenizer.next()? {
                        Some(token) if token.kind == TokenKind::Equals => {
                            match tokenizer.next()? {
                                Some(token) => match token.kind {
                                    TokenKind::StringLit(s) => ast::MetadataValue::String(s),
                                    TokenKind::Integer(s) => {
                                        ast::MetadataValue::Integer(self.parse_integer_literal(
                                            &ast::IntegerType::I64,
                                            s.as_str(),
                                            token.location,
                                        )?)
                                    }
                                    _ => {
                                        return Err(anyhow!(ParserError::UnexpectedToken {
                                            file_path: self.file_path.clone(),
                                            token
                                        }));
                                    }
                                },
                                _ => {
                                    return Err(anyhow!(ParserError::UnexpectedToken {
                                        file_path: self.file_path.clone(),
                                        token
                                    }));
                                }
                            }
                        }
                        _ => ast::MetadataValue::Boolean,
                    };
                    attrs.push((
                        ast::Ident {
                            name,
                            location: token.location,
                        },
                        value,
                    ));
                    accept_comma = true;
                }
                _ => {
                    return Err(anyhow!(ParserError::UnexpectedToken {
                        file_path: self.file_path.clone(),
                        token
                    }));
                }
            }
        }
    }

    fn parse_integer_literal(
        &self,
        base_type: &ast::IntegerType,
        s: &str,
        location: Location,
    ) -> Result<ast::IntegerValue, ParserError> {
        let radix = if s.starts_with("0b") {
            2
        } else if s.starts_with("0x") {
            16
        } else {
            10
        };
        let is_signed = matches!(
            base_type,
            ast::IntegerType::I8
                | ast::IntegerType::I16
                | ast::IntegerType::I32
                | ast::IntegerType::I64
        );

        if is_signed && (radix == 16 || radix == 2) {
            return Err(ParserError::NumberRange {
                content: s.to_string(),
                file_path: self.file_path.clone(),
                location,
            });
        }

        let digits = if radix == 2 || radix == 16 {
            &s[2..]
        } else {
            s
        };

        match base_type {
            ast::IntegerType::U8 => {
                return Ok(ast::IntegerValue::U8(
                    u8::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path.clone(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::U16 => {
                return Ok(ast::IntegerValue::U16(
                    u16::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path.clone(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::U32 => {
                return Ok(ast::IntegerValue::U32(
                    u32::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path.clone(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::U64 => {
                return Ok(ast::IntegerValue::U64(
                    u64::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path.clone(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::I8 => {
                return Ok(ast::IntegerValue::I8(
                    i8::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path.clone(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::I16 => {
                return Ok(ast::IntegerValue::I16(
                    i16::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path.clone(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::I32 => {
                return Ok(ast::IntegerValue::I32(
                    i32::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path.clone(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::I64 => {
                return Ok(ast::IntegerValue::I64(
                    i64::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path.clone(),
                        location,
                    })?,
                ));
            }
        };
    }

    fn parse_include(&self, tokenizer: &mut Tokenizer) -> anyhow::Result<ast::Schema> {
        let token = self.next_token(tokenizer)?;
        let s = match token.kind {
            TokenKind::StringLit(s) => s,
            _ => {
                return Err(anyhow!(ParserError::UnexpectedToken {
                    file_path: self.file_path.clone(),
                    token
                }));
            }
        };
        let file_path = self.resolver.borrow_mut().resolve_path(&s)?;
        let ast = Parser::new(file_path, self.resolver.clone()).parse()?;

        Ok(ast)
    }

    fn parse_ident(&self, tokenizer: &mut Tokenizer) -> anyhow::Result<ast::Ident> {
        let token = tokenizer.next()?.ok_or_else(|| {
            anyhow!(ParserError::UnexpectedEndOfFile {
                file_path: self.file_path.clone()
            })
        })?;

        if let TokenKind::Ident(name) = token.kind {
            Ok(ast::Ident {
                name,
                location: token.location,
            })
        } else {
            Err(anyhow!(ParserError::UnexpectedToken {
                file_path: self.file_path.clone(),
                token
            }))
        }
    }

    fn parse_integer_type(&self, tokenizer: &mut Tokenizer) -> anyhow::Result<ast::IntegerType> {
        let token = self.next_token(tokenizer)?;

        match token.kind {
            TokenKind::I8 => Ok(ast::IntegerType::I8),
            TokenKind::U8 => Ok(ast::IntegerType::U8),
            TokenKind::I16 => Ok(ast::IntegerType::I16),
            TokenKind::U16 => Ok(ast::IntegerType::U16),
            TokenKind::I32 => Ok(ast::IntegerType::I32),
            TokenKind::U32 => Ok(ast::IntegerType::U32),
            TokenKind::I64 => Ok(ast::IntegerType::I64),
            TokenKind::U64 => Ok(ast::IntegerType::U64),
            _ => Err(anyhow!(ParserError::UnexpectedToken {
                file_path: self.file_path.clone(),
                token
            })),
        }
    }

    fn parse_enum(
        &self,
        tokenizer: &mut Tokenizer,
        attributes: Option<ast::Attributes>,
    ) -> anyhow::Result<ast::Declaration> {
        // Grab the enum identifier
        let ident = self.parse_ident(tokenizer)?;

        // Check for a non standard base type for the enum
        let mut base_type: ast::IntegerType = ast::IntegerType::I32;
        let mut token = self.next_token(tokenizer)?;

        match token.kind {
            TokenKind::Colon => {
                base_type = self.parse_integer_type(tokenizer)?;
            }
            TokenKind::BraceOpen => {}
            _ => {
                return Err(anyhow!(ParserError::UnexpectedToken {
                    file_path: self.file_path.clone(),
                    token: token.clone()
                }));
            }
        }

        // Parse the enum variants
        let mut variants: Vec<(ast::Attributes, ast::Ident, ast::IntegerValue)> = vec![];
        let mut accept_comma = false;
        let mut variant_attrs: Option<ast::Attributes> = None;
        loop {
            token = self.next_token(tokenizer)?;

            match token.kind {
                TokenKind::Comment(_) => {}
                TokenKind::AttrOpen => {
                    if variant_attrs.is_some() {
                        return Err(anyhow!(ParserError::MultipleAttributes {
                            file_path: self.file_path.clone(),
                            location: token.location,
                        }));
                    }

                    variant_attrs = Some(self.parse_attributes(tokenizer)?);
                }
                TokenKind::Comma => {
                    if !accept_comma {
                        return Err(anyhow!(ParserError::UnexpectedComma {
                            file_path: self.file_path.clone(),
                            location: token.location,
                        }));
                    }
                    accept_comma = false;
                }
                TokenKind::Ident(name) => {
                    let ident = ast::Ident {
                        name,
                        location: token.location,
                    };

                    token = self.next_token(tokenizer)?;

                    if TokenKind::Colon != token.kind {
                        return Err(anyhow!(ParserError::UnexpectedToken {
                            file_path: self.file_path.clone(),
                            token
                        }));
                    }

                    if let TokenKind::Integer(s) = token.kind {
                        let value = self.parse_integer_literal(&base_type, &s, token.location)?;
                        let variant = (variant_attrs.take().unwrap_or(vec![]), ident, value);

                        variants.push(variant);
                        accept_comma = true;
                    } else {
                        return Err(anyhow!(ParserError::UnexpectedToken {
                            file_path: self.file_path.clone(),
                            token
                        }));
                    }
                }
                TokenKind::BraceClose => break,
                _ => {
                    return Err(anyhow!(ParserError::UnexpectedToken {
                        file_path: self.file_path.clone(),
                        token
                    }));
                }
            }
        }

        Ok(ast::Declaration::Enum {
            attributes: attributes.unwrap_or(vec![]),
            ident,
            base_type,
            variants: vec![],
        })
    }

    fn parse_struct(
        &self,
        tokenizer: &mut Tokenizer,
        attributes: Option<ast::Attributes>,
    ) -> anyhow::Result<ast::Declaration> {
        let ident = self.parse_ident(tokenizer)?;

        Ok(ast::Declaration::Struct {
            attributes: attributes.unwrap_or(vec![]),
            ident,
            fields: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast, error::*, *};
    use std::{
        cell::RefCell,
        collections::HashSet,
        path::{Path, PathBuf},
        rc::Rc,
    };

    #[test]
    fn happy_path() {
        struct TestFileResolver {
            files: HashSet<PathBuf>,
        }

        impl TestFileResolver {
            fn new() -> Self {
                Self {
                    files: HashSet::new(),
                }
            }
        }

        impl FileResolver for TestFileResolver {
            fn resolve_path(&mut self, path: &str) -> Result<PathBuf, ResolverError> {
                let path = Path::new(path);

                if self.files.insert(path.to_path_buf()) {
                    Ok(path.to_path_buf())
                } else {
                    Err(ResolverError::DuplicateInclude(path.to_path_buf()))
                }
            }

            fn read_to_string(&self, path: &Path) -> Result<String, ResolverError> {
                if path.ends_with("example.geno") {
                    Result::Ok(include_str!("../examples/example.geno").to_string())
                } else if path.ends_with("include.geno") {
                    Result::Ok(include_str!("../examples/include.geno").to_string())
                } else {
                    Err(ResolverError::FileNotFound(path.to_path_buf()))
                }
            }
        }

        let ast = Parser::new(
            PathBuf::from("../examples/example.geno"),
            Rc::new(RefCell::new(TestFileResolver::new())),
        )
        .parse()
        .unwrap();

        ast.validate().unwrap();

        let meta_other = ast
            .attributes
            .iter()
            .find(|(ident, _)| ident.name == "other");

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
