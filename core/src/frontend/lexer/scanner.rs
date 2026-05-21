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
                'A'..='Z' | 'a'..='z' | '_' => tokens.push(self.identifier()),
                '[' => tokens.push(self.bracketed_identifier()?),
                '.' => tokens.push(self.single_char(TokenKind::Dot)),
                ',' => tokens.push(self.single_char(TokenKind::Comma)),
                ':' => tokens.push(self.single_char(TokenKind::Colon)),
                '(' => tokens.push(self.single_char(TokenKind::LeftParen)),
                ')' => tokens.push(self.single_char(TokenKind::RightParen)),
                '+' => tokens.push(self.single_char(TokenKind::Plus)),
                '-' => tokens.push(self.single_char(TokenKind::Minus)),
                '*' => tokens.push(self.single_char(TokenKind::Star)),
                '^' => tokens.push(self.single_char(TokenKind::Caret)),
                '/' => tokens.push(self.single_char(TokenKind::Slash)),
                '\\' => tokens.push(self.single_char(TokenKind::Backslash)),
                '&' => tokens.push(self.single_char(TokenKind::Ampersand)),
                '%' => tokens.push(self.single_char(TokenKind::Percent)),
                '!' => tokens.push(self.single_char(TokenKind::Exclamation)),
                '#' => tokens.push(self.single_char(TokenKind::Hash)),
                '@' => tokens.push(self.single_char(TokenKind::At)),
                '$' => tokens.push(self.single_char(TokenKind::Dollar)),
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

        let lower = text.to_ascii_lowercase();
        let kind = match lower.as_str() {
            "sub" => TokenKind::Sub,
            "function" => TokenKind::Function,
            "iterator" => TokenKind::Iterator,
            "import" => TokenKind::Import,
            "const" => TokenKind::Const,
            "option" => TokenKind::Option,
            "explicit" => TokenKind::Explicit,
            "base" => TokenKind::Base,
            "compare" => TokenKind::Compare,
            "binary" => TokenKind::Binary,
            "text" => TokenKind::Text,
            "version" => TokenKind::Version,
            "begin" => TokenKind::Begin,
            "type" => TokenKind::Type,
            "structure" => TokenKind::Structure,
            "enum" => TokenKind::Enum,
            "class" => TokenKind::Class,
            "event" => TokenKind::Event,
            "declare" => TokenKind::Declare,
            "ptrsafe" => TokenKind::PtrSafe,
            "lib" => TokenKind::Lib,
            "alias" => TokenKind::Alias,
            "any" => TokenKind::Any,
            "raiseevent" => TokenKind::RaiseEvent,
            "withevents" => TokenKind::WithEvents,
            "property" => TokenKind::Property,
            "default" => TokenKind::Default,
            "get" => TokenKind::Get,
            "let" => TokenKind::Let,
            "call" => TokenKind::Call,
            "set" => TokenKind::Set,
            "nothing" => TokenKind::Nothing,
            "empty" => TokenKind::Empty,
            "null" => TokenKind::Null,
            "is" => TokenKind::Is,
            "like" => TokenKind::Like,
            "typeof" => TokenKind::TypeOf,
            "new" => TokenKind::New,
            "me" => TokenKind::Me,
            "public" => TokenKind::Public,
            "private" => TokenKind::Private,
            "end" => TokenKind::End,
            "dim" => TokenKind::Dim,
            "as" => TokenKind::As,
            "byval" => TokenKind::ByVal,
            "byref" => TokenKind::ByRef,
            "optional" => TokenKind::Optional,
            "paramarray" => TokenKind::ParamArray,
            "static" => TokenKind::Static,
            "return" => TokenKind::Return,
            "string" => TokenKind::StringType,
            "integer" => TokenKind::IntegerType,
            "boolean" => TokenKind::BooleanType,
            "variant" => TokenKind::VariantType,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "if" => TokenKind::If,
            "then" => TokenKind::Then,
            "else" => TokenKind::Else,
            "elseif" => TokenKind::ElseIf,
            "select" => TokenKind::Select,
            "case" => TokenKind::Case,
            "while" => TokenKind::While,
            "wend" => TokenKind::Wend,
            "with" => TokenKind::With,
            "using" => TokenKind::Using,
            "do" => TokenKind::Do,
            "loop" => TokenKind::Loop,
            "until" => TokenKind::Until,
            "for" => TokenKind::For,
            "each" => TokenKind::Each,
            "in" => TokenKind::In,
            "goto" => TokenKind::GoTo,
            "to" => TokenKind::To,
            "step" => TokenKind::Step,
            "next" => TokenKind::Next,
            "exit" => TokenKind::Exit,
            "on" => TokenKind::On,
            "error" => TokenKind::Error,
            "resume" => TokenKind::Resume,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "yield" => TokenKind::Yield,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "mod" => TokenKind::Mod,
            "redim" => TokenKind::ReDim,
            "preserve" => TokenKind::Preserve,
            "erase" => TokenKind::Erase,
            "console" => TokenKind::Console,
            "writeline" => TokenKind::WriteLine,
            _ => TokenKind::Identifier(text),
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
                return Ok(Token {
                    kind: TokenKind::Identifier(text),
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

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                text.push(ch);
                self.advance();
            } else if ch == '.' && !is_float {
                if self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
                    is_float = true;
                    text.push(ch);
                    self.advance();
                } else {
                    break;
                }
            } else {
                break;
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

    fn string(&mut self) -> Result<Token, Diagnostic> {
        let start = self.pos();
        self.advance();
        let mut value = String::new();

        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.advance();
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
