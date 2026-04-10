//! Lexical tokenizer for the Geno schema language.

use crate::Location;
use fallible_iterator::FallibleIterator;
use thiserror::Error;

/// Error produced by the tokenizer.
#[derive(Error, Debug, PartialEq, Clone)]
pub enum TokenizeError {
    /// An unexpected character was encountered.
    #[error("unexpected character '{ch}' ({location})")]
    UnexpectedChar {
        /// The unexpected character.
        ch: char,
        /// Where it appeared.
        location: Location,
    },
    /// A string literal was never closed.
    #[error("unterminated string literal ({location})")]
    UnterminatedString {
        /// Where the string opened.
        location: Location,
    },
    /// A numeric literal was malformed (e.g. `0b` with no digits).
    #[error("invalid number literal ({location})")]
    InvalidNumber {
        /// Where the number started.
        location: Location,
    },
}

/// Every distinct token kind in the Geno language.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// `include`
    Include,
    /// `enum`
    Enum,
    /// `struct`
    Struct,

    /// `i8`
    I8,
    /// `u8`
    U8,
    /// `i16`
    I16,
    /// `u16`
    U16,
    /// `i32`
    I32,
    /// `u32`
    U32,
    /// `i64`
    I64,
    /// `u64`
    U64,
    /// `f32`
    F32,
    /// `f64`
    F64,
    /// `string`
    String,
    /// `bool`
    Bool,

    /// `#![`
    SchemaAttrOpen,
    /// `#[`
    AttrOpen,
    /// `{`
    BraceOpen,
    /// `}`
    BraceClose,
    /// `[`
    BracketOpen,
    /// `]`
    BracketClose,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `=`
    Equals,
    /// `;`
    Semicolon,
    /// `?`
    Question,

    /// An integer literal, stored as its raw source text (e.g. `"-1"`, `"0xFF"`, `"0b101"`).
    Integer(String),
    /// A string literal with the surrounding quotes removed and escape sequences resolved.
    StringLit(String),
    /// A `//` line comment; the inner text is trimmed of leading/trailing whitespace.
    Comment(String),
    /// Any non-keyword identifier.
    Ident(String),
}

impl TokenKind {
    /// Returns a string representation of this token kind suitable for display purposes.
    pub fn display(&self) -> String {
        match self {
            Self::Include => "include".to_string(),
            Self::Enum => "enum".to_string(),
            Self::Struct => "struct".to_string(),
            Self::I8 => "i8".to_string(),
            Self::U8 => "u8".to_string(),
            Self::I16 => "i16".to_string(),
            Self::U16 => "u16".to_string(),
            Self::I32 => "i32".to_string(),
            Self::U32 => "u32".to_string(),
            Self::I64 => "i64".to_string(),
            Self::U64 => "u64".to_string(),
            Self::F32 => "f32".to_string(),
            Self::F64 => "f64".to_string(),
            Self::String => "string".to_string(),
            Self::Bool => "bool".to_string(),
            Self::SchemaAttrOpen => "#![".to_string(),
            Self::AttrOpen => "#[".to_string(),
            Self::BraceOpen => "{".to_string(),
            Self::BraceClose => "}".to_string(),
            Self::BracketOpen => "[".to_string(),
            Self::BracketClose => "]".to_string(),
            Self::Comma => ",".to_string(),
            Self::Colon => ":".to_string(),
            Self::Equals => "=".to_string(),
            Self::Semicolon => ";".to_string(),
            Self::Question => "?".to_string(),
            Self::Integer(s) => format!("{}", s),
            Self::StringLit(s) => format!("{}", s),
            Self::Comment(s) => format!("{}", s),
            Self::Ident(s) => s.to_string(),
        }
    }
}

/// A token together with its position in the source.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What kind of token this is.
    pub kind: TokenKind,
    /// Where in the source it was found.
    pub location: Location,
}

/// Lexical tokenizer for the Geno schema language.
///
/// Implements [`Iterator`]`<Item = Result<`[`Token`]`, `[`TokenizeError`]`>>`.
/// Whitespace is silently consumed; comments are yielded as [`TokenKind::Comment`].
///
/// # Example
/// ```
/// use geno::{Tokenizer, TokenKind, Token};
/// use fallible_iterator::FallibleIterator;
///
/// let src = r#"#![ format = 1 ] struct Foo { x: i32 }"#;
/// let tokens: Vec<Token> = Tokenizer::new(src).collect().unwrap();
/// ```
pub struct Tokenizer<'a> {
    input: &'a str,
    /// Current byte offset into `input`.
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer for `input`.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    // Cursor helpers

    fn location(&self) -> Location {
        Location {
            line: self.line,
            column: self.col,
        }
    }

    /// Peek at the character at the current position without advancing.
    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Peek at the nth character from the current position (0 == [`peek`]).
    fn peek_nth(&self, n: usize) -> Option<char> {
        self.input[self.pos..].chars().nth(n)
    }

    /// Consume one character, updating line/column tracking.
    fn advance(&mut self) -> Option<char> {
        let ch = self.input[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    /// Consume characters while they are ASCII whitespace.
    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\r' | '\n')) {
            self.advance();
        }
    }

    /// Tokenize a `//` comment. The `//` has already been consumed.
    fn lex_comment(&mut self, start: Location) -> Token {
        let mut content = String::new();
        while !matches!(self.peek(), Some('\n') | None) {
            content.push(self.advance().unwrap());
        }
        Token {
            kind: TokenKind::Comment(content.trim().to_string()),
            location: start,
        }
    }

    /// Tokenize a `"…"` string literal. The opening `"` has already been consumed.
    fn lex_string(&mut self, start: Location) -> Result<Token, TokenizeError> {
        let mut content = String::new();
        loop {
            match self.peek() {
                None => return Err(TokenizeError::UnterminatedString { location: start }),
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('"') => content.push('"'),
                        Some('\\') => content.push('\\'),
                        Some('n') => content.push('\n'),
                        Some('t') => content.push('\t'),
                        Some('r') => content.push('\r'),
                        Some(ch) => {
                            // Preserve unrecognised escapes verbatim.
                            content.push('\\');
                            content.push(ch);
                        }
                        None => return Err(TokenizeError::UnterminatedString { location: start }),
                    }
                }
                Some('"') => {
                    self.advance();
                    break;
                }
                Some(ch) => {
                    content.push(ch);
                    self.advance();
                }
            }
        }
        Ok(Token {
            kind: TokenKind::StringLit(content),
            location: start,
        })
    }

    /// Tokenize an integer literal.
    ///
    /// Grammar: `("0b" BIN+) | ("0x" HEX+) | (("+"|"-")? DEC+)`
    fn lex_integer(&mut self, start: Location) -> Result<Token, TokenizeError> {
        let mut raw = String::new();

        // Binary or hex prefix — only valid without a leading sign.
        if self.peek() == Some('0') {
            match self.peek_nth(1) {
                Some('b') => {
                    raw.push_str("0b");
                    self.advance();
                    self.advance();
                    if !matches!(self.peek(), Some('0' | '1')) {
                        return Err(TokenizeError::InvalidNumber { location: start });
                    }
                    while matches!(self.peek(), Some('0' | '1')) {
                        raw.push(self.advance().unwrap());
                    }
                    return Ok(Token {
                        kind: TokenKind::Integer(raw),
                        location: start,
                    });
                }
                Some('x') => {
                    raw.push_str("0x");
                    self.advance();
                    self.advance();
                    if !matches!(self.peek(), Some('0'..='9' | 'a'..='f' | 'A'..='F')) {
                        return Err(TokenizeError::InvalidNumber { location: start });
                    }
                    while matches!(self.peek(), Some('0'..='9' | 'a'..='f' | 'A'..='F')) {
                        raw.push(self.advance().unwrap());
                    }
                    return Ok(Token {
                        kind: TokenKind::Integer(raw),
                        location: start,
                    });
                }
                _ => {}
            }
        }

        // Decimal — optional leading sign.
        if matches!(self.peek(), Some('+' | '-')) {
            raw.push(self.advance().unwrap());
        }
        if !matches!(self.peek(), Some('0'..='9')) {
            return Err(TokenizeError::InvalidNumber { location: start });
        }
        while matches!(self.peek(), Some('0'..='9')) {
            raw.push(self.advance().unwrap());
        }

        Ok(Token {
            kind: TokenKind::Integer(raw),
            location: start,
        })
    }

    /// Tokenize an identifier or keyword. The first character (which must be
    /// `ASCII_ALPHA`) has not yet been consumed.
    fn lex_ident(&mut self, start: Location) -> Token {
        let mut name = String::new();
        name.push(self.advance().unwrap()); // first alpha char
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            name.push(self.advance().unwrap());
        }

        let kind = match name.as_str() {
            "include" => TokenKind::Include,
            "enum" => TokenKind::Enum,
            "struct" => TokenKind::Struct,
            "i8" => TokenKind::I8,
            "u8" => TokenKind::U8,
            "i16" => TokenKind::I16,
            "u16" => TokenKind::U16,
            "i32" => TokenKind::I32,
            "u32" => TokenKind::U32,
            "i64" => TokenKind::I64,
            "u64" => TokenKind::U64,
            "f32" => TokenKind::F32,
            "f64" => TokenKind::F64,
            "string" => TokenKind::String,
            "bool" => TokenKind::Bool,
            _ => TokenKind::Ident(name),
        };

        Token {
            kind,
            location: start,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, TokenizeError> {
        self.skip_whitespace();

        let start = self.location();
        let ch = match self.peek() {
            Some(c) => c,
            None => return Ok(None),
        };

        // Line comment
        if ch == '/' && self.peek_nth(1) == Some('/') {
            self.advance();
            self.advance();
            return Ok(Some(self.lex_comment(start)));
        }

        // Attribute openers: `#![` and `#[`
        if ch == '#' {
            self.advance(); // consume '#'
            return match self.peek() {
                Some('!') if self.peek_nth(1) == Some('[') => {
                    self.advance(); // consume '!'
                    self.advance(); // consume '['
                    Ok(Some(Token {
                        kind: TokenKind::SchemaAttrOpen,
                        location: start,
                    }))
                }
                Some('[') => {
                    self.advance(); // consume '['
                    Ok(Some(Token {
                        kind: TokenKind::AttrOpen,
                        location: start,
                    }))
                }
                Some(bad) => Err(TokenizeError::UnexpectedChar {
                    ch: bad,
                    location: self.location(),
                }),
                None => Err(TokenizeError::UnexpectedChar {
                    ch: '#',
                    location: start,
                }),
            };
        }

        // String literal
        if ch == '"' {
            self.advance();
            return self.lex_string(start).map(|t| Some(t));
        }

        // Integer literal starting with a sign
        if matches!(ch, '+' | '-') && matches!(self.peek_nth(1), Some('0'..='9')) {
            return self.lex_integer(start).map(|t| Some(t));
        }

        // Integer literal starting with a digit
        if ch.is_ascii_digit() {
            return self.lex_integer(start).map(|t| Some(t));
        }

        // Identifier or keyword
        if ch.is_ascii_alphabetic() {
            return Ok(Some(self.lex_ident(start)));
        }

        // Single-character punctuation
        self.advance();
        let kind = match ch {
            '{' => TokenKind::BraceOpen,
            '}' => TokenKind::BraceClose,
            '[' => TokenKind::BracketOpen,
            ']' => TokenKind::BracketClose,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            '=' => TokenKind::Equals,
            ';' => TokenKind::Semicolon,
            '?' => TokenKind::Question,
            bad => {
                return Err(TokenizeError::UnexpectedChar {
                    ch: bad,
                    location: start,
                });
            }
        };

        Ok(Some(Token {
            kind,
            location: start,
        }))
    }
}

impl<'a> FallibleIterator for Tokenizer<'a> {
    type Item = Token;
    type Error = TokenizeError;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        self.next_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(src: &str) -> Vec<TokenKind> {
        Tokenizer::new(src).map(|t| Ok(t.kind)).collect().unwrap()
    }

    #[test]
    fn schema_attr() {
        assert_eq!(
            tokenize("#![ format = 1 ]"),
            vec![
                TokenKind::SchemaAttrOpen,
                TokenKind::Ident("format".into()),
                TokenKind::Equals,
                TokenKind::Integer("1".into()),
                TokenKind::BracketClose,
            ]
        );
    }

    #[test]
    fn attr() {
        assert_eq!(
            tokenize(r#"#[ lang = "rust", ]"#),
            vec![
                TokenKind::AttrOpen,
                TokenKind::Ident("lang".into()),
                TokenKind::Equals,
                TokenKind::StringLit("rust".into()),
                TokenKind::Comma,
                TokenKind::BracketClose,
            ]
        );
    }

    #[test]
    fn include_directive() {
        assert_eq!(
            tokenize(r#"include "include.geno""#),
            vec![
                TokenKind::Include,
                TokenKind::StringLit("include.geno".into()),
            ]
        );
    }

    #[test]
    fn enum_decl() {
        let src = "enum Fruit: i16 { apple = 1, orange = 2, }";
        assert_eq!(
            tokenize(src),
            vec![
                TokenKind::Enum,
                TokenKind::Ident("Fruit".into()),
                TokenKind::Colon,
                TokenKind::I16,
                TokenKind::BraceOpen,
                TokenKind::Ident("apple".into()),
                TokenKind::Equals,
                TokenKind::Integer("1".into()),
                TokenKind::Comma,
                TokenKind::Ident("orange".into()),
                TokenKind::Equals,
                TokenKind::Integer("2".into()),
                TokenKind::Comma,
                TokenKind::BraceClose,
            ]
        );
    }

    #[test]
    fn struct_decl() {
        let src = "struct Foo { x: i32, y: f64?, }";
        assert_eq!(
            tokenize(src),
            vec![
                TokenKind::Struct,
                TokenKind::Ident("Foo".into()),
                TokenKind::BraceOpen,
                TokenKind::Ident("x".into()),
                TokenKind::Colon,
                TokenKind::I32,
                TokenKind::Comma,
                TokenKind::Ident("y".into()),
                TokenKind::Colon,
                TokenKind::F64,
                TokenKind::Question,
                TokenKind::Comma,
                TokenKind::BraceClose,
            ]
        );
    }

    #[test]
    fn array_and_map_types() {
        let src = "r1: [ string; 10 ], m1: { i32 : f64 }";
        assert_eq!(
            tokenize(src),
            vec![
                TokenKind::Ident("r1".into()),
                TokenKind::Colon,
                TokenKind::BracketOpen,
                TokenKind::String,
                TokenKind::Semicolon,
                TokenKind::Integer("10".into()),
                TokenKind::BracketClose,
                TokenKind::Comma,
                TokenKind::Ident("m1".into()),
                TokenKind::Colon,
                TokenKind::BraceOpen,
                TokenKind::I32,
                TokenKind::Colon,
                TokenKind::F64,
                TokenKind::BraceClose,
            ]
        );
    }

    #[test]
    fn integer_formats() {
        assert_eq!(
            tokenize("0 -1 +42 0xFF 0b101"),
            vec![
                TokenKind::Integer("0".into()),
                TokenKind::Integer("-1".into()),
                TokenKind::Integer("+42".into()),
                TokenKind::Integer("0xFF".into()),
                TokenKind::Integer("0b101".into()),
            ]
        );
    }

    #[test]
    fn comment_skipped() {
        let kinds = tokenize("// This is a comment\nstruct Foo {}");
        assert!(kinds.contains(&TokenKind::Comment("This is a comment".into())));
        assert!(kinds.contains(&TokenKind::Struct));
    }

    #[test]
    fn identifier_with_hyphen() {
        assert_eq!(
            tokenize("kiwi-fruit"),
            vec![TokenKind::Ident("kiwi-fruit".into())]
        );
    }

    #[test]
    fn string_escape() {
        assert_eq!(
            tokenize(r#""hello \"world\"""#),
            vec![TokenKind::StringLit(r#"hello "world""#.into())]
        );
    }

    #[test]
    fn error_unexpected_char() {
        let result = Tokenizer::new("@").next_token();
        assert!(matches!(
            result,
            Err(TokenizeError::UnexpectedChar { ch: '@', .. })
        ));
    }

    #[test]
    fn error_unterminated_string() {
        let result = Tokenizer::new(r#""hello"#).next();
        assert!(matches!(
            result,
            Err(TokenizeError::UnterminatedString { .. })
        ));
    }

    #[test]
    fn span_line_col() {
        // "enum" starts at line 1, col 1.
        let token = Tokenizer::new("enum").next().unwrap().unwrap();
        assert_eq!(token.location, Location { line: 1, column: 1 });
    }

    #[test]
    fn full_example_file() {
        let src = include_str!("../examples/example.geno");
        let result = Tokenizer::new(src).for_each(|_| Ok(()));
        assert!(result.is_ok(), "unexpected errors: {result:?}");
    }

    #[test]
    fn full_include_file() {
        let src = include_str!("../examples/include.geno");
        let result = Tokenizer::new(src).for_each(|_| Ok(()));
        assert!(result.is_ok(), "unexpected error: {result:?}");
    }
}
