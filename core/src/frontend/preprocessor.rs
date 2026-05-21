use std::collections::HashMap;

use crate::frontend::ast::OptionCompare;
use crate::runtime::{Diagnostic, FileId, SourcePos, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConstValue {
    Boolean(bool),
    Integer(i64),
    String(String),
}

#[derive(Debug)]
struct ConditionalFrame {
    parent_active: bool,
    active: bool,
    branch_taken: bool,
    saw_else: bool,
}

pub fn preprocess(source: &str) -> Result<String, Diagnostic> {
    let mut constants = builtin_constants();
    let mut stack: Vec<ConditionalFrame> = Vec::new();
    let mut output = String::new();
    let mut option_compare = OptionCompare::Binary;
    let mut saw_declaration = false;

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim_start();
        let active = stack.last().is_none_or(|frame| frame.active);

        if let Some(rest) = line.strip_prefix('#') {
            handle_directive(
                rest.trim_start(),
                line_number,
                active,
                &mut constants,
                &mut stack,
                option_compare,
            )?;
            output.push('\n');
            continue;
        }

        if active {
            if !saw_declaration {
                let trimmed = strip_comment(raw_line).trim();
                if !trimmed.is_empty() {
                    if let Some(compare) = parse_option_compare_line(trimmed) {
                        option_compare = compare?;
                    } else if !is_option_line(trimmed) {
                        saw_declaration = true;
                    }
                }
            }
            output.push_str(raw_line);
        }
        output.push('\n');
    }

    if let Some(frame) = stack.last() {
        let span = Span::new(
            FileId::default(),
            SourcePos::new(source.lines().count(), 1),
            SourcePos::new(source.lines().count(), 1),
        );
        let message = if frame.saw_else {
            "Missing '#End If' after '#Else'"
        } else {
            "Missing '#End If' for conditional compilation block"
        };
        return Err(
            Diagnostic::new(crate::runtime::DiagnosticCode::GENERIC, message, Some(span))
                .with_primary_label("conditional compilation block is not closed")
                .with_help("add '#End If' for this conditional block"),
        );
    }

    Ok(join_line_continuations(&output))
}

fn join_line_continuations(source: &str) -> String {
    let mut output = String::new();
    let mut in_string = false;
    let mut lines = source.lines().peekable();

    while let Some(line) = lines.next() {
        let mut code = String::new();
        let mut line_in_string = in_string;
        for ch in line.chars() {
            if ch == '"' {
                line_in_string = !line_in_string;
            }
            code.push(ch);
        }

        let stripped = strip_comment(&code);
        let trimmed = stripped.trim_end();

        // VBA rule: _ must be preceded by at least one space and be the last non-comment char
        if !line_in_string && trimmed.ends_with(" _") {
            let underscore_pos = trimmed.rfind(" _").unwrap();
            output.push_str(&line[..underscore_pos]);
            output.push(' ');
            in_string = line_in_string;
        } else {
            output.push_str(line);
            output.push('\n');
            in_string = line_in_string;
        }
    }
    output
}

fn builtin_constants() -> HashMap<String, ConstValue> {
    let mut constants = HashMap::new();
    constants.insert(key("VALO"), ConstValue::Boolean(true));
    constants.insert(key("Valo"), ConstValue::Boolean(true));
    constants.insert(key("ValoRuntime"), ConstValue::Boolean(true));
    constants.insert(key("Debug"), ConstValue::Boolean(cfg!(debug_assertions)));
    constants.insert(key("Release"), ConstValue::Boolean(!cfg!(debug_assertions)));
    constants.insert(
        key("Windows"),
        ConstValue::Boolean(cfg!(target_os = "windows")),
    );
    constants.insert(key("Linux"), ConstValue::Boolean(cfg!(target_os = "linux")));
    constants.insert(key("MacOS"), ConstValue::Boolean(cfg!(target_os = "macos")));
    constants.insert(
        key("Android"),
        ConstValue::Boolean(cfg!(target_os = "android")),
    );
    constants.insert(key("IOS"), ConstValue::Boolean(cfg!(target_os = "ios")));
    constants.insert(
        key("FreeBSD"),
        ConstValue::Boolean(cfg!(target_os = "freebsd")),
    );
    constants.insert(
        key("OpenBSD"),
        ConstValue::Boolean(cfg!(target_os = "openbsd")),
    );
    constants.insert(
        key("NetBSD"),
        ConstValue::Boolean(cfg!(target_os = "netbsd")),
    );
    constants.insert(
        key("DragonFly"),
        ConstValue::Boolean(cfg!(target_os = "dragonfly")),
    );
    constants.insert(
        key("Solaris"),
        ConstValue::Boolean(cfg!(target_os = "solaris")),
    );
    constants.insert(
        key("Illumos"),
        ConstValue::Boolean(cfg!(target_os = "illumos")),
    );
    constants.insert(key("Haiku"), ConstValue::Boolean(cfg!(target_os = "haiku")));
    constants.insert(
        key("Wasm"),
        ConstValue::Boolean(cfg!(target_arch = "wasm32") || cfg!(target_arch = "wasm64")),
    );
    constants.insert(key("Unix"), ConstValue::Boolean(cfg!(unix)));
    constants.insert(key("X86"), ConstValue::Boolean(cfg!(target_arch = "x86")));
    constants.insert(
        key("X64"),
        ConstValue::Boolean(cfg!(target_arch = "x86_64")),
    );
    constants.insert(key("Arm"), ConstValue::Boolean(cfg!(target_arch = "arm")));
    constants.insert(
        key("Arm64"),
        ConstValue::Boolean(cfg!(target_arch = "aarch64")),
    );
    constants.insert(key("Armv7"), ConstValue::Boolean(false));
    constants.insert(
        key("RiscV32"),
        ConstValue::Boolean(cfg!(target_arch = "riscv32")),
    );
    constants.insert(
        key("RiscV64"),
        ConstValue::Boolean(cfg!(target_arch = "riscv64")),
    );
    constants.insert(
        key("Wasm32"),
        ConstValue::Boolean(cfg!(target_arch = "wasm32")),
    );
    constants.insert(
        key("Wasm64"),
        ConstValue::Boolean(cfg!(target_arch = "wasm64")),
    );
    constants.insert(
        key("S390x"),
        ConstValue::Boolean(cfg!(target_arch = "s390x")),
    );
    constants.insert(
        key("PowerPC"),
        ConstValue::Boolean(cfg!(target_arch = "powerpc")),
    );
    constants.insert(
        key("PowerPC64"),
        ConstValue::Boolean(cfg!(target_arch = "powerpc64")),
    );
    constants.insert(
        key("Mips"),
        ConstValue::Boolean(cfg!(target_arch = "mips") || cfg!(target_arch = "mips32r6")),
    );
    constants.insert(
        key("Mips64"),
        ConstValue::Boolean(cfg!(target_arch = "mips64") || cfg!(target_arch = "mips64r6")),
    );
    constants.insert(
        key("LoongArch64"),
        ConstValue::Boolean(cfg!(target_arch = "loongarch64")),
    );
    constants.insert(key("VBA7"), ConstValue::Boolean(true));
    constants.insert(key("VBA6"), ConstValue::Boolean(false));
    constants.insert(
        key("Win32"),
        ConstValue::Boolean(cfg!(target_os = "windows") && cfg!(target_arch = "x86")),
    );
    constants.insert(
        key("Win64"),
        ConstValue::Boolean(cfg!(target_os = "windows") && cfg!(target_arch = "x86_64")),
    );
    constants.insert(key("Mac"), ConstValue::Boolean(cfg!(target_os = "macos")));
    constants.insert(
        key("Mac64"),
        ConstValue::Boolean(
            cfg!(target_os = "macos")
                && (cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64")),
        ),
    );
    constants
}

fn handle_directive(
    directive: &str,
    line: usize,
    active: bool,
    constants: &mut HashMap<String, ConstValue>,
    stack: &mut Vec<ConditionalFrame>,
    compare: OptionCompare,
) -> Result<(), Diagnostic> {
    let directive = directive.trim();
    let lower = directive.to_ascii_lowercase();
    if lower.starts_with("const") && keyword_boundary(directive, 5) {
        if active {
            let rest = directive[5..].trim_start();
            let (name, expr) = rest
                .split_once('=')
                .ok_or_else(|| diagnostic(line, 1, "#Const requires '='"))?;
            let name = name.trim();
            if !is_identifier(name) {
                return Err(diagnostic(line, 1, "#Const requires an identifier name"));
            }
            let value = ConstExprParser::new(expr.trim(), line, constants, compare).parse()?;
            constants.insert(key(name), value);
        }
        return Ok(());
    }
    if lower.starts_with("if") && keyword_boundary(directive, 2) {
        let rest = directive[2..].trim();
        let Some(expr_text) = strip_then(rest) else {
            return Err(diagnostic(line, 1, "#If requires 'Then'"));
        };
        let parent_active = active;
        let condition = if parent_active {
            ConstExprParser::new(expr_text, line, constants, compare)
                .parse()?
                .truthy()
        } else {
            false
        };
        stack.push(ConditionalFrame {
            parent_active,
            active: parent_active && condition,
            branch_taken: parent_active && condition,
            saw_else: false,
        });
        return Ok(());
    }
    if lower.starts_with("elseif") && keyword_boundary(directive, 6) {
        let frame = stack
            .last_mut()
            .ok_or_else(|| diagnostic(line, 1, "Unexpected '#ElseIf'"))?;
        if frame.saw_else {
            return Err(diagnostic(line, 1, "#ElseIf cannot appear after #Else"));
        }
        let rest = directive[6..].trim();
        let Some(expr_text) = strip_then(rest) else {
            return Err(diagnostic(line, 1, "#ElseIf requires 'Then'"));
        };
        if frame.parent_active && !frame.branch_taken {
            let condition = ConstExprParser::new(expr_text, line, constants, compare)
                .parse()?
                .truthy();
            frame.active = condition;
            frame.branch_taken = condition;
        } else {
            frame.active = false;
        }
        return Ok(());
    }
    if lower == "else" {
        let frame = stack
            .last_mut()
            .ok_or_else(|| diagnostic(line, 1, "Unexpected '#Else'"))?;
        if frame.saw_else {
            return Err(diagnostic(line, 1, "#Else is already declared"));
        }
        frame.saw_else = true;
        frame.active = frame.parent_active && !frame.branch_taken;
        frame.branch_taken = frame.branch_taken || frame.active;
        return Ok(());
    }
    if lower == "end if" || lower == "endif" {
        stack
            .pop()
            .ok_or_else(|| diagnostic(line, 1, "Unexpected '#End If'"))?;
        return Ok(());
    }
    Err(diagnostic(line, 1, "Unknown preprocessor directive"))
}

fn strip_then(text: &str) -> Option<&str> {
    let trimmed = text.trim_end();
    let lower = trimmed.to_ascii_lowercase();
    if lower.ends_with(" then") {
        Some(trimmed[..trimmed.len() - 5].trim_end())
    } else {
        None
    }
}

fn parse_option_compare_line(line: &str) -> Option<Result<OptionCompare, Diagnostic>> {
    let mut parts = line.split_ascii_whitespace();
    if !parts
        .next()
        .is_some_and(|part| part.eq_ignore_ascii_case("Option"))
    {
        return None;
    }
    if !parts
        .next()
        .is_some_and(|part| part.eq_ignore_ascii_case("Compare"))
    {
        return None;
    }
    let mode = parts.next();
    if parts.next().is_some() {
        return Some(Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::OPTION,
            "Option Compare must be Binary or Text",
            None,
        )));
    }
    Some(match mode {
        Some(mode) if mode.eq_ignore_ascii_case("Binary") => Ok(OptionCompare::Binary),
        Some(mode) if mode.eq_ignore_ascii_case("Text") => Ok(OptionCompare::Text),
        _ => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::OPTION,
            "Option Compare must be Binary or Text",
            None,
        )),
    })
}

fn is_option_line(line: &str) -> bool {
    line.split_ascii_whitespace()
        .next()
        .is_some_and(|part| part.eq_ignore_ascii_case("Option"))
}

fn strip_comment(line: &str) -> &str {
    line.split_once('\'').map_or(line, |(before, _)| before)
}

fn keyword_boundary(text: &str, len: usize) -> bool {
    text.len() == len
        || text
            .as_bytes()
            .get(len)
            .is_some_and(|ch| ch.is_ascii_whitespace())
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}

fn diagnostic(line: usize, column: usize, message: &str) -> Diagnostic {
    let pos = SourcePos::new(line, column);
    Diagnostic::new(
        crate::runtime::DiagnosticCode::GENERIC,
        message,
        Some(Span::new(FileId::default(), pos, pos)),
    )
    .with_primary_label(message)
}

impl ConstValue {
    fn truthy(&self) -> bool {
        match self {
            ConstValue::Boolean(value) => *value,
            ConstValue::Integer(value) => *value != 0,
            ConstValue::String(value) => !value.is_empty(),
        }
    }
}

struct ConstExprParser<'a> {
    tokens: Vec<ConstToken>,
    current: usize,
    constants: &'a HashMap<String, ConstValue>,
    compare: OptionCompare,
    line: usize,
}

impl<'a> ConstExprParser<'a> {
    fn new(
        source: &str,
        line: usize,
        constants: &'a HashMap<String, ConstValue>,
        compare: OptionCompare,
    ) -> Self {
        Self {
            tokens: ConstLexer::new(source, line).tokenize(),
            current: 0,
            constants,
            compare,
            line,
        }
    }

    fn parse(mut self) -> Result<ConstValue, Diagnostic> {
        let value = self.parse_or()?;
        if !matches!(self.peek().kind, ConstTokenKind::Eof) {
            return Err(diagnostic(
                self.line,
                self.peek().column,
                "Expected end of #If expression",
            ));
        }
        Ok(value)
    }

    fn parse_or(&mut self) -> Result<ConstValue, Diagnostic> {
        let mut value = self.parse_and()?;
        while self.match_keyword("or") {
            let right = self.parse_and()?;
            value = ConstValue::Boolean(value.truthy() || right.truthy());
        }
        Ok(value)
    }

    fn parse_and(&mut self) -> Result<ConstValue, Diagnostic> {
        let mut value = self.parse_not()?;
        while self.match_keyword("and") {
            let right = self.parse_not()?;
            value = ConstValue::Boolean(value.truthy() && right.truthy());
        }
        Ok(value)
    }

    fn parse_not(&mut self) -> Result<ConstValue, Diagnostic> {
        if self.match_keyword("not") {
            return Ok(ConstValue::Boolean(!self.parse_not()?.truthy()));
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<ConstValue, Diagnostic> {
        let mut value = self.parse_term()?;
        while let Some(op) = self.match_compare_op() {
            let right = self.parse_term()?;
            value = ConstValue::Boolean(compare_const_values(&value, op, &right, self.compare)?);
        }
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<ConstValue, Diagnostic> {
        let mut value = self.parse_factor()?;
        loop {
            if self.match_token(ConstTokenKind::Plus) {
                value = ConstValue::Integer(
                    expect_integer(value, self.line)?
                        + expect_integer(self.parse_factor()?, self.line)?,
                );
            } else if self.match_token(ConstTokenKind::Minus) {
                value = ConstValue::Integer(
                    expect_integer(value, self.line)?
                        - expect_integer(self.parse_factor()?, self.line)?,
                );
            } else {
                break;
            }
        }
        Ok(value)
    }

    fn parse_factor(&mut self) -> Result<ConstValue, Diagnostic> {
        let mut value = self.parse_unary()?;
        loop {
            if self.match_token(ConstTokenKind::Star) {
                value = ConstValue::Integer(
                    expect_integer(value, self.line)?
                        * expect_integer(self.parse_unary()?, self.line)?,
                );
            } else if self.match_token(ConstTokenKind::Slash) {
                let right = expect_integer(self.parse_unary()?, self.line)?;
                if right == 0 {
                    return Err(diagnostic(
                        self.line,
                        self.previous().column,
                        "Division by zero",
                    ));
                }
                value = ConstValue::Integer(expect_integer(value, self.line)? / right);
            } else {
                break;
            }
        }
        Ok(value)
    }

    fn parse_unary(&mut self) -> Result<ConstValue, Diagnostic> {
        if self.match_token(ConstTokenKind::Minus) {
            return Ok(ConstValue::Integer(-expect_integer(
                self.parse_unary()?,
                self.line,
            )?));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<ConstValue, Diagnostic> {
        let token = self.advance().clone();
        match token.kind {
            ConstTokenKind::Integer(value) => Ok(ConstValue::Integer(value)),
            ConstTokenKind::String(value) => Ok(ConstValue::String(value)),
            ConstTokenKind::Identifier(name) if name.eq_ignore_ascii_case("true") => {
                Ok(ConstValue::Boolean(true))
            }
            ConstTokenKind::Identifier(name) if name.eq_ignore_ascii_case("false") => {
                Ok(ConstValue::Boolean(false))
            }
            ConstTokenKind::Identifier(name) => {
                self.constants.get(&key(&name)).cloned().ok_or_else(|| {
                    diagnostic(
                        self.line,
                        token.column,
                        "Compile-time constant is not defined",
                    )
                })
            }
            ConstTokenKind::LeftParen => {
                let value = self.parse_or()?;
                if !self.match_token(ConstTokenKind::RightParen) {
                    return Err(diagnostic(self.line, self.peek().column, "Expected ')'"));
                }
                Ok(value)
            }
            _ => Err(diagnostic(
                self.line,
                token.column,
                "Expected #If expression",
            )),
        }
    }

    fn match_keyword(&mut self, keyword: &str) -> bool {
        if matches!(&self.peek().kind, ConstTokenKind::Identifier(name) if name.eq_ignore_ascii_case(keyword))
        {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_compare_op(&mut self) -> Option<CompareOp> {
        let op = match self.peek().kind {
            ConstTokenKind::Equal => CompareOp::Equal,
            ConstTokenKind::NotEqual => CompareOp::NotEqual,
            ConstTokenKind::Less => CompareOp::Less,
            ConstTokenKind::Greater => CompareOp::Greater,
            ConstTokenKind::LessEqual => CompareOp::LessEqual,
            ConstTokenKind::GreaterEqual => CompareOp::GreaterEqual,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    fn match_token(&mut self, kind: ConstTokenKind) -> bool {
        if std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(&kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> &ConstToken {
        if !matches!(self.peek().kind, ConstTokenKind::Eof) {
            self.current += 1;
        }
        self.previous()
    }

    fn previous(&self) -> &ConstToken {
        &self.tokens[self.current - 1]
    }

    fn peek(&self) -> &ConstToken {
        &self.tokens[self.current]
    }
}

#[derive(Debug, Clone, Copy)]
enum CompareOp {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
}

fn compare_const_values(
    left: &ConstValue,
    op: CompareOp,
    right: &ConstValue,
    compare: OptionCompare,
) -> Result<bool, Diagnostic> {
    let ordering = match (left, right) {
        (ConstValue::Integer(left), ConstValue::Integer(right)) => left.cmp(right),
        (ConstValue::String(left), ConstValue::String(right)) => {
            if compare == OptionCompare::Text {
                left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase())
            } else {
                left.cmp(right)
            }
        }
        (ConstValue::Boolean(left), ConstValue::Boolean(right)) => left.cmp(right),
        _ => return Ok(false),
    };
    Ok(match op {
        CompareOp::Equal => ordering.is_eq(),
        CompareOp::NotEqual => !ordering.is_eq(),
        CompareOp::Less => ordering.is_lt(),
        CompareOp::Greater => ordering.is_gt(),
        CompareOp::LessEqual => ordering.is_le(),
        CompareOp::GreaterEqual => ordering.is_ge(),
    })
}

fn expect_integer(value: ConstValue, line: usize) -> Result<i64, Diagnostic> {
    match value {
        ConstValue::Integer(value) => Ok(value),
        _ => Err(diagnostic(
            line,
            1,
            "Compile-time arithmetic requires Integer operands",
        )),
    }
}

#[derive(Debug, Clone)]
struct ConstToken {
    kind: ConstTokenKind,
    column: usize,
}

#[derive(Debug, Clone)]
enum ConstTokenKind {
    Identifier(String),
    Integer(i64),
    String(String),
    LeftParen,
    RightParen,
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Eof,
}

struct ConstLexer<'a> {
    chars: Vec<char>,
    current: usize,
    _source: &'a str,
}

impl<'a> ConstLexer<'a> {
    fn new(source: &'a str, _line: usize) -> Self {
        Self {
            chars: source.chars().collect(),
            current: 0,
            _source: source,
        }
    }

    fn tokenize(mut self) -> Vec<ConstToken> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek() {
            let column = self.current + 1;
            match ch {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '0'..='9' => tokens.push(self.integer(column)),
                '"' => tokens.push(self.string(column)),
                'A'..='Z' | 'a'..='z' | '_' => tokens.push(self.identifier(column)),
                '(' => tokens.push(self.single(column, ConstTokenKind::LeftParen)),
                ')' => tokens.push(self.single(column, ConstTokenKind::RightParen)),
                '+' => tokens.push(self.single(column, ConstTokenKind::Plus)),
                '-' => tokens.push(self.single(column, ConstTokenKind::Minus)),
                '*' => tokens.push(self.single(column, ConstTokenKind::Star)),
                '/' => tokens.push(self.single(column, ConstTokenKind::Slash)),
                '=' => tokens.push(self.single(column, ConstTokenKind::Equal)),
                '<' => tokens.push(self.less(column)),
                '>' => tokens.push(self.greater(column)),
                _ => {
                    self.advance();
                }
            }
        }
        tokens.push(ConstToken {
            kind: ConstTokenKind::Eof,
            column: self.current + 1,
        });
        tokens
    }

    fn identifier(&mut self, column: usize) -> ConstToken {
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                text.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        ConstToken {
            kind: ConstTokenKind::Identifier(text),
            column,
        }
    }

    fn integer(&mut self, column: usize) -> ConstToken {
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                text.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        ConstToken {
            kind: ConstTokenKind::Integer(text.parse().unwrap_or(0)),
            column,
        }
    }

    fn string(&mut self, column: usize) -> ConstToken {
        self.advance();
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            self.advance();
            if ch == '"' {
                break;
            }
            text.push(ch);
        }
        ConstToken {
            kind: ConstTokenKind::String(text),
            column,
        }
    }

    fn less(&mut self, column: usize) -> ConstToken {
        self.advance();
        let kind = match self.peek() {
            Some('=') => {
                self.advance();
                ConstTokenKind::LessEqual
            }
            Some('>') => {
                self.advance();
                ConstTokenKind::NotEqual
            }
            _ => ConstTokenKind::Less,
        };
        ConstToken { kind, column }
    }

    fn greater(&mut self, column: usize) -> ConstToken {
        self.advance();
        let kind = match self.peek() {
            Some('=') => {
                self.advance();
                ConstTokenKind::GreaterEqual
            }
            _ => ConstTokenKind::Greater,
        };
        ConstToken { kind, column }
    }

    fn single(&mut self, column: usize, kind: ConstTokenKind) -> ConstToken {
        self.advance();
        ConstToken { kind, column }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.current).copied()
    }

    fn advance(&mut self) {
        self.current += 1;
    }
}
