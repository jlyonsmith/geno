//! Lexical tokenizer for the Geno schema language.

use crate::{
    FileResolver, Location, ParserError, Token, TokenKind, Tokenizer,
    ast::{self, Attributes, FieldType},
};
use anyhow::anyhow;
use fallible_iterator::FallibleIterator;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

type PeekableTokenizer<'a> = fallible_iterator::Peekable<Tokenizer<'a>>;

/// A parser for Geno schemas
pub struct Parser {
    resolver: Rc<RefCell<dyn FileResolver>>,
}

impl Parser {
    /// Creates a new parser for the given file path, using the given resolver to load the file contents.
    pub fn new(resolver: Rc<RefCell<dyn FileResolver>>) -> Self {
        Self {
            resolver: Rc::clone(&resolver),
        }
    }

    fn next_token(&self, tokenizer: &mut PeekableTokenizer) -> anyhow::Result<Token> {
        loop {
            let token = match tokenizer.next()? {
                Some(token) => token,
                None => {
                    return Err(anyhow!(ParserError::UnexpectedEndOfFile {
                        file_path: self.file_path(),
                    }));
                }
            };

            if matches!(token.kind, TokenKind::Comment(_)) {
                continue;
            }

            return Ok(token);
        }
    }

    fn peek_token(&self, tokenizer: &mut PeekableTokenizer) -> anyhow::Result<Token> {
        loop {
            let token = match tokenizer.peek()? {
                Some(token) => token.clone(),
                None => {
                    return Err(anyhow!(ParserError::UnexpectedEndOfFile {
                        file_path: self.file_path(),
                    }));
                }
            };

            if matches!(token.kind, TokenKind::Comment(_)) {
                tokenizer.next()?;
                continue;
            }

            return Ok(token);
        }
    }

    fn file_path(&self) -> PathBuf {
        self.resolver.borrow().current_path().unwrap().clone()
    }

    /// Parses the schema
    pub fn parse(&self, file_path: &Path) -> anyhow::Result<ast::Schema> {
        self.resolver.borrow_mut().push_path(&file_path)?;

        let mut schema_attrs: Option<ast::Attributes> = None;
        let mut attrs: Option<ast::Attributes> = None;
        let mut includes: Vec<(ast::Attributes, ast::Schema)> = vec![];
        let input = self.resolver.borrow().read_to_string()?;
        let mut tokenizer = Tokenizer::new(&input).peekable();
        let mut declarations: Vec<ast::Declaration> = vec![];

        loop {
            let token = match tokenizer.peek()? {
                Some(token) => token,
                None => {
                    // This is only time having no more tokens is OK
                    break;
                }
            };

            match token.kind {
                TokenKind::Comment(_) => {
                    tokenizer.next()?;
                }
                TokenKind::SchemaAttrOpen => {
                    if schema_attrs.is_some() {
                        return Err(anyhow!(ParserError::MultipleSchemaAttributes {
                            file_path: self.file_path(),
                            location: token.location
                        }));
                    }

                    schema_attrs = Some(self.parse_attributes(&mut tokenizer)?);
                }
                TokenKind::AttrOpen => {
                    if attrs.is_some() {
                        return Err(anyhow!(ParserError::MultipleAttributes {
                            file_path: self.file_path(),
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
                        file_path: self.file_path(),
                        token: token.clone(),
                    }));
                }
            }
        }

        let ast = ast::Schema {
            attributes: schema_attrs.unwrap_or(vec![]),
            declarations,
            includes,
            file_path: self.file_path(),
        };

        self.resolver.borrow_mut().pop_path();

        Ok(ast)
    }

    fn parse_attributes(
        &self,
        tokenizer: &mut PeekableTokenizer,
    ) -> anyhow::Result<ast::Attributes> {
        let mut attrs: Attributes = vec![];
        let mut accept_comma = false;

        // Consume the AttrOpen
        tokenizer.next()?;

        loop {
            let token = self.peek_token(tokenizer)?;

            match token.kind {
                TokenKind::Comma => {
                    if !accept_comma {
                        return Err(anyhow!(ParserError::UnexpectedComma {
                            file_path: self.file_path(),
                            location: token.location,
                        }));
                    }

                    tokenizer.next()?;
                    accept_comma = false;
                }
                TokenKind::BracketClose => {
                    tokenizer.next()?;
                    break;
                }
                TokenKind::Ident(ref name) => {
                    if accept_comma {
                        return Err(anyhow!(ParserError::MissingComma {
                            file_path: self.file_path(),
                            location: token.location,
                        }));
                    }

                    tokenizer.next()?;

                    let value = match self.peek_token(tokenizer)?.kind {
                        TokenKind::Equals => {
                            tokenizer.next()?;

                            match self.next_token(tokenizer)?.kind {
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
                                        file_path: self.file_path(),
                                        token
                                    }));
                                }
                            }
                        }
                        _ => ast::MetadataValue::Boolean,
                    };
                    attrs.push((
                        ast::Ident {
                            name: name.clone(),
                            location: token.location,
                        },
                        value,
                    ));
                    accept_comma = true;
                }
                _ => {
                    return Err(anyhow!(ParserError::UnexpectedToken {
                        file_path: self.file_path(),
                        token: token.clone()
                    }));
                }
            }
        }

        Ok(attrs)
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
                file_path: self.file_path(),
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
                        file_path: self.file_path(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::U16 => {
                return Ok(ast::IntegerValue::U16(
                    u16::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::U32 => {
                return Ok(ast::IntegerValue::U32(
                    u32::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::U64 => {
                return Ok(ast::IntegerValue::U64(
                    u64::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::I8 => {
                return Ok(ast::IntegerValue::I8(
                    i8::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::I16 => {
                return Ok(ast::IntegerValue::I16(
                    i16::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::I32 => {
                return Ok(ast::IntegerValue::I32(
                    i32::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path(),
                        location,
                    })?,
                ));
            }
            ast::IntegerType::I64 => {
                return Ok(ast::IntegerValue::I64(
                    i64::from_str_radix(digits, radix).map_err(|_| ParserError::NumberRange {
                        content: s.to_string(),
                        file_path: self.file_path(),
                        location,
                    })?,
                ));
            }
        };
    }

    fn parse_include(&self, tokenizer: &mut PeekableTokenizer) -> anyhow::Result<ast::Schema> {
        // Consume the Include token
        tokenizer.next()?;

        let token = self.next_token(tokenizer)?;
        let file_path = match token.kind {
            TokenKind::StringLit(s) => PathBuf::from(s),
            _ => {
                return Err(anyhow!(ParserError::UnexpectedToken {
                    file_path: self.file_path(),
                    token
                }));
            }
        };

        Ok(Parser::new(self.resolver.clone()).parse(&file_path)?)
    }

    fn parse_enum(
        &self,
        tokenizer: &mut PeekableTokenizer,
        attributes: Option<ast::Attributes>,
    ) -> anyhow::Result<ast::Declaration> {
        // Consume the Enum token
        tokenizer.next()?;

        // Grab the enum identifier
        let token = self.next_token(tokenizer)?;

        let ident = match token.kind {
            TokenKind::Ident(name) => ast::Ident {
                name,
                location: token.location,
            },
            _ => {
                return Err(anyhow!(ParserError::UnexpectedToken {
                    file_path: self.file_path(),
                    token
                }));
            }
        };

        // Check for a non standard base type for the enum
        let mut base_type: ast::IntegerType = ast::IntegerType::I32;
        let mut token = self.next_token(tokenizer)?;

        match token.kind {
            TokenKind::Colon => {
                base_type = match self.next_token(tokenizer)?.kind {
                    TokenKind::I8 => ast::IntegerType::I8,
                    TokenKind::U8 => ast::IntegerType::U8,
                    TokenKind::I16 => ast::IntegerType::I16,
                    TokenKind::U16 => ast::IntegerType::U16,
                    TokenKind::I32 => ast::IntegerType::I32,
                    TokenKind::U32 => ast::IntegerType::U32,
                    TokenKind::I64 => ast::IntegerType::I64,
                    TokenKind::U64 => ast::IntegerType::U64,
                    _ => {
                        return Err(anyhow!(ParserError::UnexpectedToken {
                            file_path: self.file_path(),
                            token
                        }));
                    }
                };

                let token = self.next_token(tokenizer)?;

                if token.kind != TokenKind::BraceOpen {
                    return Err(anyhow!(ParserError::UnexpectedToken {
                        file_path: self.file_path(),
                        token
                    }));
                }
            }
            TokenKind::BraceOpen => {}
            _ => {
                return Err(anyhow!(ParserError::UnexpectedToken {
                    file_path: self.file_path(),
                    token: token.clone()
                }));
            }
        }

        // Parse the enum variants
        let mut variants: Vec<(ast::Attributes, ast::Ident, ast::IntegerValue)> = vec![];
        let mut variant_attrs: Option<ast::Attributes> = None;
        let mut accept_comma = false;

        loop {
            token = self.peek_token(tokenizer)?;

            match token.kind {
                TokenKind::AttrOpen => {
                    if variant_attrs.is_some() {
                        return Err(anyhow!(ParserError::MultipleAttributes {
                            file_path: self.file_path(),
                            location: token.location,
                        }));
                    }

                    variant_attrs = Some(self.parse_attributes(tokenizer)?);
                }
                TokenKind::Comma => {
                    if !accept_comma {
                        return Err(anyhow!(ParserError::UnexpectedComma {
                            file_path: self.file_path(),
                            location: token.location,
                        }));
                    }
                    tokenizer.next()?;
                    accept_comma = false;
                }
                TokenKind::Ident(name) => {
                    if accept_comma {
                        return Err(anyhow!(ParserError::UnexpectedComma {
                            file_path: self.file_path(),
                            location: token.location,
                        }));
                    }

                    tokenizer.next()?;

                    let ident = ast::Ident {
                        name,
                        location: token.location,
                    };

                    token = self.next_token(tokenizer)?;

                    if TokenKind::Equals != token.kind {
                        return Err(anyhow!(ParserError::UnexpectedToken {
                            file_path: self.file_path(),
                            token
                        }));
                    }

                    token = self.next_token(tokenizer)?;

                    if let TokenKind::Integer(s) = token.kind {
                        let value = self.parse_integer_literal(&base_type, &s, token.location)?;
                        let variant = (variant_attrs.take().unwrap_or(vec![]), ident, value);

                        variants.push(variant);
                        accept_comma = true;
                    } else {
                        return Err(anyhow!(ParserError::UnexpectedToken {
                            file_path: self.file_path(),
                            token
                        }));
                    }
                }
                TokenKind::BraceClose => {
                    tokenizer.next()?;
                    break;
                }
                _ => {
                    return Err(anyhow!(ParserError::UnexpectedToken {
                        file_path: self.file_path(),
                        token
                    }));
                }
            }
        }

        Ok(ast::Declaration::Enum {
            attributes: attributes.unwrap_or(vec![]),
            ident,
            base_type,
            variants,
        })
    }

    fn parse_struct(
        &self,
        tokenizer: &mut PeekableTokenizer,
        attributes: Option<ast::Attributes>,
    ) -> anyhow::Result<ast::Declaration> {
        // Consume the Struct token
        tokenizer.next()?;

        // Grab the struct identifier
        let token = self.next_token(tokenizer)?;

        let ident = match token.kind {
            TokenKind::Ident(name) => ast::Ident {
                name,
                location: token.location,
            },
            _ => {
                return Err(anyhow!(ParserError::UnexpectedToken {
                    file_path: self.file_path(),
                    token
                }));
            }
        };

        let token = self.next_token(tokenizer)?;

        if token.kind != TokenKind::BraceOpen {
            return Err(anyhow!(ParserError::UnexpectedToken {
                file_path: self.file_path(),
                token
            }));
        }

        let mut fields: Vec<(ast::Attributes, ast::Ident, ast::FieldType)> = vec![];
        let mut field_attrs: Option<ast::Attributes> = None;
        let mut accept_comma = false;

        loop {
            let token = self.peek_token(tokenizer)?;

            match token.kind {
                TokenKind::AttrOpen => {
                    if field_attrs.is_some() {
                        return Err(anyhow!(ParserError::MultipleAttributes {
                            file_path: self.file_path(),
                            location: token.location,
                        }));
                    }

                    field_attrs = Some(self.parse_attributes(tokenizer)?);
                }
                TokenKind::Comma => {
                    if !accept_comma {
                        return Err(anyhow!(ParserError::UnexpectedComma {
                            file_path: self.file_path(),
                            location: token.location,
                        }));
                    }
                    tokenizer.next()?;
                    accept_comma = false;
                }
                TokenKind::Ident(name) => {
                    if accept_comma {
                        return Err(anyhow!(ParserError::UnexpectedComma {
                            file_path: self.file_path(),
                            location: token.location,
                        }));
                    }

                    tokenizer.next()?;

                    let ident = ast::Ident {
                        name,
                        location: token.location,
                    };

                    let token = self.next_token(tokenizer)?;

                    if TokenKind::Colon != token.kind {
                        return Err(anyhow!(ParserError::UnexpectedToken {
                            file_path: self.file_path(),
                            token
                        }));
                    }

                    let field_type = self.parse_field_type(tokenizer)?;

                    fields.push((field_attrs.take().unwrap_or(vec![]), ident, field_type));

                    accept_comma = true;
                }
                TokenKind::BraceClose => {
                    tokenizer.next()?;
                    break;
                }
                _ => {
                    return Err(anyhow!(ParserError::UnexpectedToken {
                        file_path: self.file_path(),
                        token
                    }));
                }
            }
        }

        Ok(ast::Declaration::Struct {
            attributes: attributes.unwrap_or(vec![]),
            ident,
            fields,
        })
    }

    fn parse_nullable(&self, tokenizer: &mut PeekableTokenizer) -> anyhow::Result<bool> {
        // Check if type is nullable
        let token = self.peek_token(tokenizer)?;

        if token.kind == TokenKind::Question {
            tokenizer.next()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn parse_field_type(
        &self,
        tokenizer: &mut PeekableTokenizer,
    ) -> anyhow::Result<ast::FieldType> {
        let token = self.peek_token(tokenizer)?;

        if token.kind == TokenKind::BracketOpen {
            tokenizer.next()?;

            let field_type = Box::new(self.parse_field_type(tokenizer)?);
            let size = if self.peek_token(tokenizer)?.kind == TokenKind::Semicolon {
                tokenizer.next()?;

                let token = self.next_token(tokenizer)?;

                if let TokenKind::Integer(s) = token.kind {
                    Some(self.parse_integer_literal(&ast::IntegerType::U32, &s, token.location)?)
                } else {
                    return Err(anyhow!(ParserError::UnexpectedToken {
                        file_path: self.file_path(),
                        token
                    }));
                }
            } else {
                None
            };
            let token = self.next_token(tokenizer)?;

            if token.kind != TokenKind::BracketClose {
                return Err(anyhow!(ParserError::MissingBracket {
                    file_path: self.file_path(),
                    location: token.location
                }));
            }

            let nullable = self.parse_nullable(tokenizer)?;

            Ok(ast::FieldType::Array(field_type, size, nullable))
        } else if token.kind == TokenKind::BraceOpen {
            tokenizer.next()?;

            let key_type = self.parse_builtin_type(tokenizer)?;

            let token = self.next_token(tokenizer)?;

            if token.kind != TokenKind::Colon {
                return Err(anyhow!(ParserError::MissingColon {
                    file_path: self.file_path(),
                    location: token.location
                }));
            }

            let value_type = Box::new(self.parse_field_type(tokenizer)?);

            let token = self.next_token(tokenizer)?;

            if token.kind != TokenKind::BraceClose {
                return Err(anyhow!(ParserError::MissingBrace {
                    file_path: self.file_path(),
                    location: token.location
                }));
            }

            let nullable = self.parse_nullable(tokenizer)?;

            Ok(FieldType::Map(key_type, value_type, nullable))
        } else if let TokenKind::Ident(name) = token.kind {
            tokenizer.next()?;

            Ok(FieldType::UserDefined(
                ast::Ident {
                    name,
                    location: token.location,
                },
                self.parse_nullable(tokenizer)?,
            ))
        } else {
            // Builtin
            Ok(FieldType::Builtin(
                self.parse_builtin_type(tokenizer)?,
                self.parse_nullable(tokenizer)?,
            ))
        }
    }

    fn parse_builtin_type(
        &self,
        tokenizer: &mut PeekableTokenizer,
    ) -> anyhow::Result<ast::BuiltinType> {
        let token = self.next_token(tokenizer)?;

        match token.kind {
            TokenKind::I8 => Ok(ast::BuiltinType::Integer(ast::IntegerType::I8)),
            TokenKind::U8 => Ok(ast::BuiltinType::Integer(ast::IntegerType::U8)),
            TokenKind::I16 => Ok(ast::BuiltinType::Integer(ast::IntegerType::I16)),
            TokenKind::U16 => Ok(ast::BuiltinType::Integer(ast::IntegerType::U16)),
            TokenKind::I32 => Ok(ast::BuiltinType::Integer(ast::IntegerType::I32)),
            TokenKind::U32 => Ok(ast::BuiltinType::Integer(ast::IntegerType::U32)),
            TokenKind::I64 => Ok(ast::BuiltinType::Integer(ast::IntegerType::I64)),
            TokenKind::U64 => Ok(ast::BuiltinType::Integer(ast::IntegerType::U64)),
            TokenKind::F32 => Ok(ast::BuiltinType::Float(ast::FloatType::F32)),
            TokenKind::F64 => Ok(ast::BuiltinType::Float(ast::FloatType::F64)),
            TokenKind::String => Ok(ast::BuiltinType::String),
            TokenKind::Bool => Ok(ast::BuiltinType::Bool),
            _ => Err(anyhow!(ParserError::UnexpectedToken {
                file_path: self.file_path(),
                token
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast, *};
    use phf::phf_map;
    use std::{
        cell::RefCell,
        path::{Path, PathBuf},
        rc::Rc,
    };

    static FILE_PATHS: phf::Map<&'static str, &'static str> = phf_map! {
        "example.geno" => include_str!("../examples/example.geno"),
        "include.geno" => include_str!("../examples/include.geno"),
        "eof_1.geno" => include_str!("../examples/test/eof_1.geno"),
        "number_range.geno" => include_str!("../examples/test/number_range.geno"),
    };

    struct TestFileResolver {
        files: Vec<PathBuf>,
    }

    impl TestFileResolver {
        fn new() -> Self {
            Self { files: Vec::new() }
        }
    }

    impl FileResolver for TestFileResolver {
        fn push_path(&mut self, path: &Path) -> Result<(), ResolverError> {
            if self.files.iter().find(|p| *p == path).is_none() {
                self.files.push(path.to_path_buf());
                Ok(())
            } else {
                Err(ResolverError::DuplicateInclude(path.to_path_buf()))
            }
        }

        fn pop_path(&mut self) {}

        fn current_path(&self) -> Option<&PathBuf> {
            self.files.iter().last()
        }

        fn read_to_string(&self) -> Result<String, ResolverError> {
            let path = self
                .current_path()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            FILE_PATHS
                .get(path)
                .ok_or(ResolverError::Io(
                    PathBuf::from(path),
                    std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
                ))
                .map(|s| s.to_string())
        }
    }

    #[test]
    fn happy_path() {
        let ast = Parser::new(Rc::new(RefCell::new(TestFileResolver::new())))
            .parse(&Path::new("example.geno"))
            .unwrap();

        ast.validate().unwrap();

        let meta_other = ast
            .attributes
            .iter()
            .find(|(ident, _)| ident.name == "other");

        assert!(meta_other.is_some());
        assert_eq!(meta_other.unwrap().0.as_str(), "other");

        let struct_type1 = ast
            .declarations
            .iter()
            .find(|d| matches!(d, ast::Declaration::Struct { ident, .. } if ident.name == "Type1"));

        let decls = ast.flatten_decls();

        assert!(struct_type1.is_some());

        let enum_enum1 = decls
            .iter()
            .find(|d| matches!(d, ast::Declaration::Enum { ident, .. } if ident.name == "Enum1"));

        assert!(enum_enum1.is_some());
    }

    #[test]
    fn end_of_file() {
        let result = Parser::new(Rc::new(RefCell::new(TestFileResolver::new())))
            .parse(&Path::new("eof_1.geno"));

        match result {
            Err(err) => {
                let err = err.downcast_ref::<ParserError>();
                assert!(err.is_some());
                assert!(matches!(
                    err.unwrap(),
                    ParserError::UnexpectedEndOfFile { .. }
                ));
            }
            _ => {
                panic!("expected an error");
            }
        }
    }

    #[test]
    fn number_range() {
        let result = Parser::new(Rc::new(RefCell::new(TestFileResolver::new())))
            .parse(&Path::new("number_range.geno"));

        match result {
            Err(err) => {
                assert!(err.downcast_ref::<ParserError>().is_some());
            }
            _ => {
                panic!("expected an error");
            }
        }
    }
}
