use crate::runtime::{Diagnostic, FileId, SourcePos, Span};

use super::{Token, TokenKind};

pub struct Lexer<'a> {
    file_id: FileId,
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    _source: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            file_id: FileId::default(),
            chars: source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            _source: source,
        }
    }

    pub fn with_id(mut self, file_id: FileId) -> Self {
        self.file_id = file_id;
        self
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek() {
            match ch {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => tokens.push(self.single_char(TokenKind::Newline)),
                '\'' => self.skip_comment(),
                '"' => tokens.push(self.string()?),
                '0'..='9' => tokens.push(self.number()?),
                'A'..='Z' | 'a'..='z' => tokens.push(self.identifier()),
                '_' => {
                    if let Some('\n') = self.peek_next() {
                        self.advance();
                        self.advance();
                    } else if let Some('\r') = self.peek_next() {
                        if let Some('\n') = self.peek_at(2) {
                            self.advance();
                            self.advance();
                            self.advance();
                        } else {
                            tokens.push(self.identifier());
                        }
                    } else {
                        tokens.push(self.identifier());
                    }
                }
                '[' => tokens.push(self.bracketed_identifier()?),
                '.' => {
                    if self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
                        tokens.push(self.number()?)
                    } else {
                        tokens.push(self.single_char(TokenKind::Dot))
                    }
                }
                ',' => tokens.push(self.single_char(TokenKind::Comma)),
                ':' => tokens.push(self.single_char(TokenKind::Colon)),
                '(' => tokens.push(self.single_char(TokenKind::LeftParen)),
                ')' => tokens.push(self.single_char(TokenKind::RightParen)),
                '{' => tokens.push(self.single_char(TokenKind::LeftBrace)),
                '}' => tokens.push(self.single_char(TokenKind::RightBrace)),
                '+' => tokens.push(self.single_char(TokenKind::Plus)),
                '-' => tokens.push(self.single_char(TokenKind::Minus)),
                '*' => tokens.push(self.single_char(TokenKind::Star)),
                '^' => tokens.push(self.single_char(TokenKind::Caret)),
                '/' => tokens.push(self.single_char(TokenKind::Slash)),
                '\\' => tokens.push(self.single_char(TokenKind::Backslash)),
                '&' => {
                    if let Some('H') | Some('h') = self.peek_next() {
                        tokens.push(self.hex_number()?)
                    } else if let Some('O') | Some('o') = self.peek_next() {
                        tokens.push(self.octal_number()?)
                    } else {
                        tokens.push(self.single_char(TokenKind::Ampersand))
                    }
                }
                '%' => tokens.push(self.single_char(TokenKind::Percent)),
                '!' => tokens.push(self.single_char(TokenKind::Exclamation)),
                '#' => tokens.push(self.single_char(TokenKind::Hash)),
                '@' => tokens.push(self.single_char(TokenKind::At)),
                '$' => tokens.push(self.single_char(TokenKind::Dollar)),
                ';' => tokens.push(self.single_char(TokenKind::Semicolon)),
                '=' => tokens.push(self.single_char(TokenKind::Equal)),
                '<' => tokens.push(self.less_or_not_equal()),
                '>' => tokens.push(self.greater_or_equal()),
                _ => {
                    let span = self.current_span();
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::PARSE,
                        format!("Unexpected character '{}'", ch),
                        Some(span),
                    ));
                }
            }
        }

        let pos = self.pos();
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.file_id, pos, pos),
        });
        Ok(tokens)
    }

    fn identifier(&mut self) -> Token {
        let start = self.pos();
        let mut text = String::new();

        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                text.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let mut hint = None;
        if let Some(ch) = self.peek() {
            hint = match ch {
                '%' => Some(crate::runtime::TypeName::Integer),
                '&' => Some(crate::runtime::TypeName::Long),
                '!' => Some(crate::runtime::TypeName::Single),
                '#' => Some(crate::runtime::TypeName::Double),
                '@' => Some(crate::runtime::TypeName::Currency),
                '$' => Some(crate::runtime::TypeName::String),
                _ => None,
            };

            if hint.is_some() {
                self.advance();
            }
        }

        let lower = text.to_ascii_lowercase();
        let kind = match lower.as_str() {
            "sub" if hint.is_none() => TokenKind::Sub,
            "function" if hint.is_none() => TokenKind::Function,
            "iterator" if hint.is_none() => TokenKind::Iterator,
            "import" if hint.is_none() => TokenKind::Import,
            "namespace" if hint.is_none() => TokenKind::Namespace,
            "module" if hint.is_none() => TokenKind::Module,
            "const" if hint.is_none() => TokenKind::Const,
            "option" if hint.is_none() => TokenKind::Option,
            "explicit" if hint.is_none() => TokenKind::Explicit,
            "base" if hint.is_none() => TokenKind::Base,
            "compare" if hint.is_none() => TokenKind::Compare,
            "binary" if hint.is_none() => TokenKind::Binary,
            "text" if hint.is_none() => TokenKind::Text,
            "version" if hint.is_none() => TokenKind::Version,
            "begin" if hint.is_none() => TokenKind::Begin,
            "type" if hint.is_none() => TokenKind::Type,
            "structure" if hint.is_none() => TokenKind::Structure,
            "interface" if hint.is_none() => TokenKind::Interface,
            "enum" if hint.is_none() => TokenKind::Enum,
            "class" if hint.is_none() => TokenKind::Class,
            "inherits" if hint.is_none() => TokenKind::Inherits,
            "mustinherit" if hint.is_none() => TokenKind::MustInherit,
            "notinheritable" if hint.is_none() => TokenKind::NotInheritable,
            "implements" if hint.is_none() => TokenKind::Implements,
            "shared" if hint.is_none() => TokenKind::Shared,
            "readonly" if hint.is_none() => TokenKind::ReadOnly,
            "writeonly" if hint.is_none() => TokenKind::WriteOnly,
            "overridable" if hint.is_none() => TokenKind::Overridable,
            "overrides" if hint.is_none() => TokenKind::Overrides,
            "mustoverride" if hint.is_none() => TokenKind::MustOverride,
            "shadows" if hint.is_none() => TokenKind::Shadows,
            "event" if hint.is_none() => TokenKind::Event,
            "declare" if hint.is_none() => TokenKind::Declare,
            "ptrsafe" if hint.is_none() => TokenKind::PtrSafe,
            "lib" if hint.is_none() => TokenKind::Lib,
            "alias" if hint.is_none() => TokenKind::Alias,
            "any" if hint.is_none() => TokenKind::Any,
            "raiseevent" if hint.is_none() => TokenKind::RaiseEvent,
            "withevents" if hint.is_none() => TokenKind::WithEvents,
            "property" if hint.is_none() => TokenKind::Property,
            "default" if hint.is_none() => TokenKind::Default,
            "get" if hint.is_none() => TokenKind::Get,
            "let" if hint.is_none() => TokenKind::Let,
            "call" if hint.is_none() => TokenKind::Call,
            "set" if hint.is_none() => TokenKind::Set,
            "nothing" if hint.is_none() => TokenKind::Nothing,
            "addressof" if hint.is_none() => TokenKind::AddressOf,
            "empty" if hint.is_none() => TokenKind::Empty,
            "null" if hint.is_none() => TokenKind::Null,
            "is" if hint.is_none() => TokenKind::Is,
            "like" if hint.is_none() => TokenKind::Like,
            "typeof" if hint.is_none() => TokenKind::TypeOf,
            "new" if hint.is_none() => TokenKind::New,
            "me" if hint.is_none() => TokenKind::Me,
            "public" if hint.is_none() => TokenKind::Public,
            "private" if hint.is_none() => TokenKind::Private,
            "friend" if hint.is_none() => TokenKind::Friend,
            "protected" if hint.is_none() => TokenKind::Protected,
            "end" if hint.is_none() => TokenKind::End,
            "dim" if hint.is_none() => TokenKind::Dim,
            "as" if hint.is_none() => TokenKind::As,
            "byval" if hint.is_none() => TokenKind::ByVal,
            "byref" if hint.is_none() => TokenKind::ByRef,
            "optional" if hint.is_none() => TokenKind::Optional,
            "paramarray" if hint.is_none() => TokenKind::ParamArray,
            "static" if hint.is_none() => TokenKind::Static,
            "return" if hint.is_none() => TokenKind::Return,
            "string" if hint.is_none() => TokenKind::StringType,
            "integer" if hint.is_none() => TokenKind::IntegerType,
            "boolean" if hint.is_none() => TokenKind::BooleanType,
            "variant" if hint.is_none() => TokenKind::VariantType,
            "true" if hint.is_none() => TokenKind::True,
            "false" if hint.is_none() => TokenKind::False,
            "if" if hint.is_none() => TokenKind::If,
            "then" if hint.is_none() => TokenKind::Then,
            "else" if hint.is_none() => TokenKind::Else,
            "elseif" if hint.is_none() => TokenKind::ElseIf,
            "select" if hint.is_none() => TokenKind::Select,
            "case" if hint.is_none() => TokenKind::Case,
            "while" if hint.is_none() => TokenKind::While,
            "wend" if hint.is_none() => TokenKind::Wend,
            "with" if hint.is_none() => TokenKind::With,
            "using" if hint.is_none() => TokenKind::Using,
            "do" if hint.is_none() => TokenKind::Do,
            "loop" if hint.is_none() => TokenKind::Loop,
            "until" if hint.is_none() => TokenKind::Until,
            "for" if hint.is_none() => TokenKind::For,
            "each" if hint.is_none() => TokenKind::Each,
            "in" if hint.is_none() => TokenKind::In,
            "of" if hint.is_none() => TokenKind::Of,
            "goto" if hint.is_none() => TokenKind::GoTo,
            "to" if hint.is_none() => TokenKind::To,
            "step" if hint.is_none() => TokenKind::Step,
            "next" if hint.is_none() => TokenKind::Next,
            "exit" if hint.is_none() => TokenKind::Exit,
            "on" if hint.is_none() => TokenKind::On,
            "error" if hint.is_none() => TokenKind::Error,
            "resume" if hint.is_none() => TokenKind::Resume,
            "try" if hint.is_none() => TokenKind::Try,
            "catch" if hint.is_none() => TokenKind::Catch,
            "finally" if hint.is_none() => TokenKind::Finally,
            "throw" if hint.is_none() => TokenKind::Throw,
            "yield" if hint.is_none() => TokenKind::Yield,
            "and" if hint.is_none() => TokenKind::And,
            "or" if hint.is_none() => TokenKind::Or,
            "xor" if hint.is_none() => TokenKind::Xor,
            "eqv" if hint.is_none() => TokenKind::Eqv,
            "imp" if hint.is_none() => TokenKind::Imp,
            "not" if hint.is_none() => TokenKind::Not,
            "mod" if hint.is_none() => TokenKind::Mod,
            "redim" if hint.is_none() => TokenKind::ReDim,
            "preserve" if hint.is_none() => TokenKind::Preserve,
            "erase" if hint.is_none() => TokenKind::Erase,
            "console" if hint.is_none() => TokenKind::Console,
            "writeline" if hint.is_none() => TokenKind::WriteLine,
            _ => TokenKind::Identifier(text, hint),
        };

        Token {
            kind,
            span: Span::new(self.file_id, start, self.pos()),
        }
    }

    fn bracketed_identifier(&mut self) -> Result<Token, Diagnostic> {
        let start = self.pos();
        self.advance();
        let mut text = String::new();

        while let Some(ch) = self.peek() {
            if ch == ']' {
                self.advance();
                if text.is_empty() {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        "Square-bracket array type syntax is not supported; use 'Dim name() As Type'",
                        Some(Span::new(self.file_id, start, self.pos())),
                    ));
                }

                let mut hint = None;
                if let Some(ch) = self.peek() {
                    hint = match ch {
                        '%' => Some(crate::runtime::TypeName::Integer),
                        '&' => Some(crate::runtime::TypeName::Long),
                        '!' => Some(crate::runtime::TypeName::Single),
                        '#' => Some(crate::runtime::TypeName::Double),
                        '@' => Some(crate::runtime::TypeName::Currency),
                        '$' => Some(crate::runtime::TypeName::String),
                        _ => None,
                    };

                    if hint.is_some() {
                        self.advance();
                    }
                }

                return Ok(Token {
                    kind: TokenKind::Identifier(text, hint),
                    span: Span::new(self.file_id, start, self.pos()),
                });
            }
            if ch == '\n' {
                break;
            }
            text.push(ch);
            self.advance();
        }

        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::PARSE,
            "Unterminated bracketed identifier",
            Some(Span::new(self.file_id, start, self.pos())),
        ))
    }

    fn number(&mut self) -> Result<Token, Diagnostic> {
        let start = self.pos();
        let mut text = String::new();
        let mut is_float = false;
        let mut has_exponent = false;

        if let Some('.') = self.peek() {
            is_float = true;
            text.push('.');
            self.advance();
        }

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                text.push(ch);
                self.advance();
            } else if ch == '.' && !is_float && !has_exponent {
                is_float = true;
                text.push(ch);
                self.advance();
            } else if (ch == 'E' || ch == 'e') && !has_exponent {
                let valid = self.peek_next().is_some_and(|next| {
                    next.is_ascii_digit()
                        || ((next == '+' || next == '-')
                            && self.peek_at(2).is_some_and(|c| c.is_ascii_digit()))
                });

                if valid {
                    has_exponent = true;
                    is_float = true;
                    text.push(ch);
                    self.advance();
                    if let Some(next) = self.peek()
                        && (next == '+' || next == '-')
                    {
                        text.push(next);
                        self.advance();
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(ch) = self.peek() {
            match ch {
                '%' | '&' | '^' | '!' | '#' | '@' => {
                    text.push(ch);
                    self.advance();
                    is_float = true;
                }
                _ => {}
            }
        }

        if is_float {
            Ok(Token {
                kind: TokenKind::Float(text),
                span: Span::new(self.file_id, start, self.pos()),
            })
        } else {
            let value = text.parse::<i64>().map_err(|_| {
                if text.parse::<u64>().is_ok() {
                    return Diagnostic::new(
                        crate::runtime::DiagnosticCode::PARSE,
                        format!(
                            "Integer literal '{}' is too large (use Int64 or Double)",
                            text
                        ),
                        Some(Span::new(self.file_id, start, self.pos())),
                    );
                }
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!("Integer literal '{}' is out of range", text),
                    Some(Span::new(self.file_id, start, self.pos())),
                )
            });

            match value {
                Ok(v) => Ok(Token {
                    kind: TokenKind::Integer(v),
                    span: Span::new(self.file_id, start, self.pos()),
                }),
                Err(e) => {
                    if text.parse::<f64>().is_ok() {
                        Ok(Token {
                            kind: TokenKind::Float(text),
                            span: Span::new(self.file_id, start, self.pos()),
                        })
                    } else {
                        Err(e)
                    }
                }
            }
        }
    }

    fn hex_number(&mut self) -> Result<Token, Diagnostic> {
        let start = self.pos();
        self.advance(); // &
        self.advance(); // H or h
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_hexdigit() {
                text.push(ch);
                self.advance();
            } else if ch == '&' {
                text.push(ch);
                self.advance();
                break;
            } else {
                break;
            }
        }
        Ok(Token {
            kind: TokenKind::Hex(text),
            span: Span::new(self.file_id, start, self.pos()),
        })
    }

    fn octal_number(&mut self) -> Result<Token, Diagnostic> {
        let start = self.pos();
        self.advance(); // &
        self.advance(); // O or o
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            if ('0'..='7').contains(&ch) {
                text.push(ch);
                self.advance();
            } else if ch == '&' {
                text.push(ch);
                self.advance();
                break;
            } else {
                break;
            }
        }
        Ok(Token {
            kind: TokenKind::Octal(text),
            span: Span::new(self.file_id, start, self.pos()),
        })
    }

    fn string(&mut self) -> Result<Token, Diagnostic> {
        let start = self.pos();
        self.advance();
        let mut value = String::new();

        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.advance();
                if let Some('"') = self.peek() {
                    value.push('"');
                    self.advance();
                    continue;
                }
                return Ok(Token {
                    kind: TokenKind::String(value),
                    span: Span::new(self.file_id, start, self.pos()),
                });
            }

            if ch == '\n' {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    "Unterminated string literal",
                    Some(Span::new(self.file_id, start, self.pos())),
                ));
            }

            value.push(ch);
            self.advance();
        }

        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::PARSE,
            "Unterminated string literal",
            Some(Span::new(self.file_id, start, self.pos())),
        ))
    }

    fn less_or_not_equal(&mut self) -> Token {
        let start = self.pos();
        self.advance();
        let kind = match self.peek() {
            Some('=') => {
                self.advance();
                TokenKind::LessEqual
            }
            Some('>') => {
                self.advance();
                TokenKind::NotEqual
            }
            _ => TokenKind::Less,
        };
        Token {
            kind,
            span: Span::new(self.file_id, start, self.pos()),
        }
    }

    fn greater_or_equal(&mut self) -> Token {
        let start = self.pos();
        self.advance();
        let kind = match self.peek() {
            Some('=') => {
                self.advance();
                TokenKind::GreaterEqual
            }
            _ => TokenKind::Greater,
        };
        Token {
            kind,
            span: Span::new(self.file_id, start, self.pos()),
        }
    }

    fn skip_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn single_char(&mut self, kind: TokenKind) -> Token {
        let start = self.pos();
        self.advance();
        Token {
            kind,
            span: Span::new(self.file_id, start, self.pos()),
        }
    }

    fn current_span(&self) -> Span {
        let pos = self.pos();
        Span::new(self.file_id, pos, pos)
    }

    fn pos(&self) -> SourcePos {
        SourcePos::new(self.line, self.column)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.index + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.index += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_console_writeline_and_comments() {
        let tokens = Lexer::new("Console.WriteLine(\"hi\") ' comment")
            .tokenize()
            .unwrap();
        let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();

        assert_eq!(
            kinds,
            vec![
                TokenKind::Console,
                TokenKind::Dot,
                TokenKind::WriteLine,
                TokenKind::LeftParen,
                TokenKind::String("hi".to_string()),
                TokenKind::RightParen,
                TokenKind::Eof,
            ]
        );
    }
}

#[cfg(test)]
mod scanner_tests {
    use super::*;

    #[test]
    fn test_string_escaping() {
        let tokens = Lexer::new("\"He said \"\"Hello\"\" to me\"")
            .tokenize()
            .unwrap();
        if let TokenKind::String(value) = &tokens[0].kind {
            assert_eq!(value, "He said \"Hello\" to me");
        } else {
            panic!("Expected string token");
        }
    }

    #[test]
    fn backslashes_are_literal_inside_strings() {
        let tokens = Lexer::new("\"\\t\\nC:\\\\Temp\\\\file.txt\"")
            .tokenize()
            .unwrap();
        if let TokenKind::String(value) = &tokens[0].kind {
            assert_eq!(value, "\\t\\nC:\\\\Temp\\\\file.txt");
        } else {
            panic!("Expected string token");
        }
    }
}
