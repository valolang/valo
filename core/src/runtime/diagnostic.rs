use std::fmt;
use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePos {
    pub line: usize,
    pub column: usize,
}

impl SourcePos {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct FileId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub file_id: FileId,
    pub start: SourcePos,
    pub end: SourcePos,
}

impl Span {
    pub fn new(file_id: FileId, start: SourcePos, end: SourcePos) -> Self {
        Self {
            file_id,
            start,
            end,
        }
    }

    pub fn empty(file_id: FileId) -> Self {
        Self::new(file_id, SourcePos::new(1, 1), SourcePos::new(1, 1))
    }
}

#[derive(Debug, Clone)]
pub struct SourceMap {
    sources: Vec<Source>,
}

#[derive(Debug, Clone)]
struct Source {
    name: String,
    content: String,
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    pub fn add(&mut self, name: String, content: String) -> FileId {
        let id = FileId(self.sources.len() as u32);
        self.sources.push(Source { name, content });
        id
    }

    pub fn get_name(&self, id: FileId) -> Option<&str> {
        self.sources.get(id.0 as usize).map(|s| s.name.as_str())
    }

    pub fn get_content(&self, id: FileId) -> Option<&str> {
        self.sources.get(id.0 as usize).map(|s| s.content.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: Box<str>,
    pub span: Option<Box<Span>>,
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub runtime_error: Option<Box<RuntimeErrorInfo>>,
    pub labels: Box<Vec<DiagnosticLabel>>,
    pub notes: Box<Vec<String>>,
    pub helps: Box<Vec<String>>,
    pub related: Box<Vec<Diagnostic>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeErrorInfo {
    pub number: i64,
    pub source: String,
    pub description: String,
    pub help_file: String,
    pub help_context: i64,
}

impl Diagnostic {
    pub fn new(code: DiagnosticCode, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            code,
            message: message.into().into_boxed_str(),
            span: span.map(Box::new),
            severity: Severity::Error,
            labels: Box::default(),
            notes: Box::default(),
            helps: Box::default(),
            related: Box::default(),
            runtime_error: None,
        }
    }

    pub fn with_code(mut self, code: DiagnosticCode) -> Self {
        self.code = code;
        self
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_primary_label(mut self, message: impl Into<String>) -> Self {
        if let Some(span) = &self.span {
            self.labels.push(DiagnosticLabel::primary(**span, message));
        }
        self
    }

    pub fn with_secondary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel::secondary(span, message));
        self
    }

    pub fn with_note(mut self, message: impl Into<String>) -> Self {
        self.notes.push(message.into());
        self
    }

    pub fn with_help(mut self, message: impl Into<String>) -> Self {
        self.helps.push(message.into());
        self
    }

    pub fn with_related(mut self, diagnostic: Diagnostic) -> Self {
        self.related.push(diagnostic);
        self
    }

    pub fn with_runtime_error(mut self, info: RuntimeErrorInfo) -> Self {
        self.runtime_error = Some(Box::new(info));
        self
    }

    pub fn render(&self, source_map: &SourceMap) -> String {
        self.render_colored(source_map, terminal_supports_color())
    }

    pub fn render_colored(&self, source_map: &SourceMap, use_color: bool) -> String {
        let mut out = String::new();
        let color = ColorSupport::new(use_color);

        out.push_str(&format!(
            "{}{}[{}]{}: {}{}{}\n",
            color.bold(self.severity_color()),
            self.severity,
            self.code,
            color.reset(),
            color.bold(""),
            self.message,
            color.reset()
        ));

        if let Some(span) = &self.span {
            let source_name = source_map.get_name(span.file_id).unwrap_or("<unknown>");
            out.push_str(&format!(
                "  {}-->{}{} {}:{}:{}\n",
                color.blue(""),
                color.reset(),
                color.bold(""),
                source_name,
                span.start.line,
                span.start.column
            ));
            out.push_str(&format!("   {}|{}\n", color.blue(""), color.reset()));

            if let Some(source) = source_map.get_content(span.file_id) {
                render_span_lines(&mut out, source, **span, &self.labels, &color);
            }
        }

        for note in self.notes.iter() {
            out.push_str(&format!(
                "   {}={} {}note{}: {}\n",
                color.blue(""),
                color.reset(),
                color.cyan(""),
                color.reset(),
                note
            ));
        }
        for help in self.helps.iter() {
            out.push_str(&format!(
                "   {}={} {}help{}: {}\n",
                color.blue(""),
                color.reset(),
                color.cyan(""),
                color.reset(),
                help
            ));
        }
        for related in self.related.iter() {
            out.push_str(&related.render_colored(source_map, use_color));
            out.push('\n');
        }

        out.trim_end().to_string()
    }

    fn severity_color(&self) -> &'static str {
        match self.severity {
            Severity::Error => "\x1b[31;1m",
            Severity::Warning => "\x1b[33;1m",
            Severity::Note => "\x1b[36;1m",
            Severity::Help => "\x1b[32;1m",
        }
    }
}

pub fn terminal_supports_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some() || !std::io::stderr().is_terminal() {
        return false;
    }
    #[cfg(windows)]
    {
        if std::env::var_os("WT_SESSION").is_some()
            || std::env::var_os("ANSICON").is_some()
            || std::env::var_os("ConEmuANSI").is_some()
            || std::env::var("TERM")
                .map(|term| term != "dumb")
                .unwrap_or(false)
        {
            return true;
        }
        false
    }
    #[cfg(not(windows))]
    {
        std::env::var("TERM")
            .map(|term| term != "dumb")
            .unwrap_or(false)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.span {
            Some(span) => write!(
                f,
                "{}[{}]: {} at line {}, column {}",
                self.severity, self.code, self.message, span.start.line, span.start.column
            ),
            None => write!(f, "{}[{}]: {}", self.severity, self.code, self.message),
        }
    }
}

impl std::error::Error for Diagnostic {}

struct ColorSupport {
    enabled: bool,
}

impl ColorSupport {
    fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    fn bold(&self, code: &str) -> String {
        if self.enabled {
            format!("{}{}", code, "\x1b[1m")
        } else {
            String::new()
        }
    }

    fn blue(&self, _text: &str) -> &str {
        if self.enabled { "\x1b[34;1m" } else { "" }
    }

    fn cyan(&self, _text: &str) -> &str {
        if self.enabled { "\x1b[36;1m" } else { "" }
    }

    fn reset(&self) -> &str {
        if self.enabled { "\x1b[0m" } else { "" }
    }
}

fn render_span_lines(
    out: &mut String,
    source: &str,
    primary: Span,
    labels: &[DiagnosticLabel],
    color: &ColorSupport,
) {
    let line_number = primary.start.line;
    let source_line = source
        .lines()
        .nth(line_number.saturating_sub(1))
        .unwrap_or("");

    out.push_str(&format!(
        "{}{:3} |{} {}\n",
        color.blue(""),
        line_number,
        color.reset(),
        source_line
    ));

    let primary_label = labels
        .iter()
        .find(|label| label.style == LabelStyle::Primary && label.span == primary)
        .map(|label| label.message.as_str())
        .unwrap_or("");

    out.push_str(&format!(
        "    {}|{} {}{}{} {}\n",
        color.blue(""),
        color.reset(),
        " ".repeat(primary.start.column.saturating_sub(1)),
        color.bold("\x1b[31m"),
        "^".repeat(span_width(primary)),
        primary_label
    ));
    out.push_str(color.reset());

    for label in labels
        .iter()
        .filter(|label| label.style == LabelStyle::Secondary)
    {
        if label.span.start.line == line_number {
            out.push_str(&format!(
                "    {}|{} {}{} {}\n",
                color.blue(""),
                color.reset(),
                " ".repeat(label.span.start.column.saturating_sub(1)),
                "-".repeat(span_width(label.span)),
                label.message
            ));
        } else {
            let source_line = source
                .lines()
                .nth(label.span.start.line.saturating_sub(1))
                .unwrap_or("");
            out.push_str(&format!("    {}|{}\n", color.blue(""), color.reset()));
            out.push_str(&format!(
                "{}{:3} |{} {}\n",
                color.blue(""),
                label.span.start.line,
                color.reset(),
                source_line
            ));
            out.push_str(&format!(
                "    {}|{} {}{} {}\n",
                color.blue(""),
                color.reset(),
                " ".repeat(label.span.start.column.saturating_sub(1)),
                "-".repeat(span_width(label.span)),
                label.message
            ));
        }
    }

    out.push_str(&format!("    {}|{}\n", color.blue(""), color.reset()));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        };
        write!(f, "{text}")
    }
}

/// Diagnostic code scheme:
/// V0000 generic diagnostics, V0100 syntax/options/preprocessor,
/// V1000 name/declaration/member lookup, V1100 typing/assignment,
/// V1200 arrays, V1300 control flow, V3000 native FFI, and V9000 runtime execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticCode(pub &'static str);

impl DiagnosticCode {
    pub const GENERIC: Self = Self("V0001");
    pub const PARSE: Self = Self("V0100");
    pub const OPTION: Self = Self("V0101");
    pub const PREPROCESSOR: Self = Self("V0102");
    pub const UNKNOWN_NAME: Self = Self("V1001");
    pub const DUPLICATE_DECLARATION: Self = Self("V1002");
    pub const PRIVATE_ACCESS: Self = Self("V1003");
    pub const TYPE_MISMATCH: Self = Self("V1100");
    pub const INVALID_ASSIGNMENT: Self = Self("V1101");
    pub const ARRAY: Self = Self("V1200");
    pub const CONTROL_FLOW: Self = Self("V1300");
    pub const MEMBER_ACCESS: Self = Self("V1400");
    pub const SELECT_CASE: Self = Self("V1500");
    pub const MODULE_NOT_FOUND: Self = Self("V1600");
    pub const DUPLICATE_IMPORT: Self = Self("V1601");
    pub const IMPORT_CYCLE: Self = Self("V1602");
    pub const AMBIGUOUS_IMPORT: Self = Self("V1603");
    pub const CASE_COLLISION: Self = Self("V1604");
    pub const UNKNOWN_QUALIFIED_SYMBOL: Self = Self("V1605");
    pub const INVALID_QUALIFIED_ACCESS: Self = Self("V1606");
    pub const FFI_LIBRARY_NOT_FOUND: Self = Self("V3001");
    pub const FFI_SYMBOL_NOT_FOUND: Self = Self("V3002");
    pub const FFI_UNSUPPORTED_MARSHALING: Self = Self("V3003");
    pub const FFI_CALL: Self = Self("V3004");
    pub const RUNTIME: Self = Self("V9000");
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub message: String,
    pub style: LabelStyle,
}

impl DiagnosticLabel {
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Primary,
        }
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Secondary,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelStyle {
    Primary,
    Secondary,
}

fn span_width(span: Span) -> usize {
    if span.start.line == span.end.line {
        span.end.column.saturating_sub(span.start.column).max(1)
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_code_labels_notes_and_help() {
        let mut source_map = SourceMap::new();
        let file_id = source_map.add(
            "test.valo".to_string(),
            "Dim age As Integer\n    age = \"Valo\"".to_string(),
        );

        let span = Span::new(file_id, SourcePos::new(2, 5), SourcePos::new(2, 8));
        let other = Span::new(file_id, SourcePos::new(1, 1), SourcePos::new(1, 4));
        let diagnostic = Diagnostic::new(
            DiagnosticCode::GENERIC,
            "cannot assign String to Integer",
            Some(span),
        )
        .with_code(DiagnosticCode::TYPE_MISMATCH)
        .with_primary_label("expected Integer, found String")
        .with_secondary_label(other, "variable declared here")
        .with_note("assignment types match")
        .with_help("change the variable type or assign an Integer value");

        let rendered = diagnostic.render_colored(&source_map, false);

        assert!(rendered.contains("error[V1100]: cannot assign String to Integer"));
        assert!(rendered.contains("--> test.valo:2:5"));
        assert!(rendered.contains("expected Integer, found String"));
        assert!(rendered.contains("variable declared here"));
        assert!(rendered.contains("note: assignment types match"));
        assert!(rendered.contains("help: change the variable type"));
    }
}
