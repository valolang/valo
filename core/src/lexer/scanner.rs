use crate::runtime::{Diagnostic, SourcePos, Span};

use super::{Token, TokenKind};

pub struct Lexer<'a> {
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    _source: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            _source: source,
        }
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
                '0'..='9' => tokens.push(self.integer()?),
                'A'..='Z' | 'a'..='z' | '_' => tokens.push(self.identifier()),
                '.' => tokens.push(self.single_char(TokenKind::Dot)),
                ',' => tokens.push(self.single_char(TokenKind::Comma)),
                '(' => tokens.push(self.single_char(TokenKind::LeftParen)),
                ')' => tokens.push(self.single_char(TokenKind::RightParen)),
                '+' => tokens.push(self.single_char(TokenKind::Plus)),
                '-' => tokens.push(self.single_char(TokenKind::Minus)),
                '*' => tokens.push(self.single_char(TokenKind::Star)),
                '/' => tokens.push(self.single_char(TokenKind::Slash)),
                '&' => tokens.push(self.single_char(TokenKind::Ampersand)),
                '=' => tokens.push(self.single_char(TokenKind::Equal)),
                '<' => tokens.push(self.less_or_not_equal()),
                '>' => tokens.push(self.greater_or_equal()),
                _ => {
                    let span = self.current_span();
                    return Err(Diagnostic::new(
                        format!("Unexpected character '{}'", ch),
                        Some(span),
                    ));
                }
            }
        }

        let pos = self.pos();
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(pos, pos),
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
            "type" => TokenKind::Type,
            "class" => TokenKind::Class,
            "property" => TokenKind::Property,
            "get" => TokenKind::Get,
            "let" => TokenKind::Let,
            "set" => TokenKind::Set,
            "nothing" => TokenKind::Nothing,
            "is" => TokenKind::Is,
            "new" => TokenKind::New,
            "me" => TokenKind::Me,
            "public" => TokenKind::Public,
            "private" => TokenKind::Private,
            "end" => TokenKind::End,
            "dim" => TokenKind::Dim,
            "as" => TokenKind::As,
            "byval" => TokenKind::ByVal,
            "byref" => TokenKind::ByRef,
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
            "while" => TokenKind::While,
            "wend" => TokenKind::Wend,
            "for" => TokenKind::For,
            "to" => TokenKind::To,
            "step" => TokenKind::Step,
            "next" => TokenKind::Next,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "mod" => TokenKind::Mod,
            "console" => TokenKind::Console,
            "writeline" => TokenKind::WriteLine,
            _ => TokenKind::Identifier(text),
        };

        Token {
            kind,
            span: Span::new(start, self.pos()),
        }
    }

    fn integer(&mut self) -> Result<Token, Diagnostic> {
        let start = self.pos();
        let mut text = String::new();

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                text.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let value = text.parse::<i64>().map_err(|_| {
            Diagnostic::new(
                format!("Integer literal '{}' is out of range", text),
                Some(Span::new(start, self.pos())),
            )
        })?;

        Ok(Token {
            kind: TokenKind::Integer(value),
            span: Span::new(start, self.pos()),
        })
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
                    span: Span::new(start, self.pos()),
                });
            }

            if ch == '\n' {
                return Err(Diagnostic::new(
                    "Unterminated string literal",
                    Some(Span::new(start, self.pos())),
                ));
            }

            value.push(ch);
            self.advance();
        }

        Err(Diagnostic::new(
            "Unterminated string literal",
            Some(Span::new(start, self.pos())),
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
            span: Span::new(start, self.pos()),
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
            span: Span::new(start, self.pos()),
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
            span: Span::new(start, self.pos()),
        }
    }

    fn current_span(&self) -> Span {
        let pos = self.pos();
        Span::new(pos, pos)
    }

    fn pos(&self) -> SourcePos {
        SourcePos::new(self.line, self.column)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
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
