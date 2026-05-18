use std::fmt;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: SourcePos,
    pub end: SourcePos,
}

impl Span {
    pub fn new(start: SourcePos, end: SourcePos) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Option<Span>,
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<String>,
    pub helps: Vec<String>,
    pub related: Vec<Diagnostic>,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        let message = message.into();
        Self {
            code: DiagnosticCode::for_message(&message),
            message,
            span,
            severity: Severity::Error,
            labels: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            related: Vec::new(),
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
        if let Some(span) = self.span {
            self.labels.push(DiagnosticLabel::primary(span, message));
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

    pub fn render(&self, source_name: &str, source: &str) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{}[{}]: {}\n",
            self.severity, self.code, self.message
        ));

        if let Some(span) = self.span {
            out.push_str(&format!(
                "  --> {}:{}:{}\n",
                source_name, span.start.line, span.start.column
            ));
            out.push_str("   |\n");
            render_span_lines(&mut out, source, span, &self.labels);
        }

        for note in &self.notes {
            out.push_str(&format!("note: {note}\n"));
        }
        for help in &self.helps {
            out.push_str(&format!("help: {help}\n"));
        }
        for related in &self.related {
            out.push_str(&format!(
                "{}[{}]: {}\n",
                related.severity, related.code, related.message
            ));
            if let Some(span) = related.span {
                out.push_str(&format!(
                    "  --> {}:{}:{}\n",
                    source_name, span.start.line, span.start.column
                ));
            }
        }

        out.trim_end().to_string()
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.span {
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
/// V1200 arrays, V1300 control flow, and V9000 runtime execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticCode(&'static str);

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
    pub const RUNTIME: Self = Self("V9000");

    fn for_message(message: &str) -> Self {
        if message.starts_with("Expected")
            || message.starts_with("Unexpected character")
            || message.starts_with("Unterminated")
            || message.starts_with("Only Option")
        {
            Self::PARSE
        } else if message.contains("Option Base") || message.contains("Option Compare") {
            Self::OPTION
        } else if message.contains("#End If")
            || message.contains("#Else")
            || message.contains("#ElseIf")
            || message.contains("#If")
            || message.contains("#Const")
            || message.contains("preprocessor")
        {
            Self::PREPROCESSOR
        } else if message.contains("not declared")
            || message.contains("not defined")
            || message.contains("Unknown")
        {
            Self::UNKNOWN_NAME
        } else if message.contains("already declared")
            || message.contains("already defined")
            || message.contains("conflicts with existing")
        {
            Self::DUPLICATE_DECLARATION
        } else if message.contains("Array")
            || message.contains("array")
            || message.contains("ReDim")
            || message.contains("LBound")
            || message.contains("UBound")
        {
            Self::ARRAY
        } else if message.contains("Exit") || message.contains("Return is only allowed") {
            Self::CONTROL_FLOW
        } else if message.contains("Case") || message.contains("Select") {
            Self::SELECT_CASE
        } else if message.contains("Private") || message.contains("private") {
            Self::PRIVATE_ACCESS
        } else if message.contains("not assignable") || message.contains("cannot be assigned") {
            Self::INVALID_ASSIGNMENT
        } else if message.contains("Cannot assign")
            || message.contains("requires")
            || message.contains("must be")
            || message.contains("type mismatch")
        {
            Self::TYPE_MISMATCH
        } else if message.contains("member")
            || message.contains("field")
            || message.contains("Object reference is Nothing")
            || message.contains("method")
            || message.contains("property")
        {
            Self::MEMBER_ACCESS
        } else {
            Self::GENERIC
        }
    }
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

fn render_span_lines(out: &mut String, source: &str, primary: Span, labels: &[DiagnosticLabel]) {
    let line_number = primary.start.line;
    let source_line = source
        .lines()
        .nth(line_number.saturating_sub(1))
        .unwrap_or("");
    out.push_str(&format!("{line_number:>3} | {source_line}\n"));

    let primary_label = labels
        .iter()
        .find(|label| label.style == LabelStyle::Primary && label.span == primary)
        .map(|label| label.message.as_str())
        .unwrap_or("");
    out.push_str(&format!(
        "    | {}{} {}\n",
        " ".repeat(primary.start.column.saturating_sub(1)),
        "^".repeat(span_width(primary)),
        primary_label
    ));

    for label in labels
        .iter()
        .filter(|label| label.style == LabelStyle::Secondary)
    {
        if label.span.start.line == line_number {
            out.push_str(&format!(
                "    | {}{} {}\n",
                " ".repeat(label.span.start.column.saturating_sub(1)),
                "-".repeat(span_width(label.span)),
                label.message
            ));
        } else {
            let source_line = source
                .lines()
                .nth(label.span.start.line.saturating_sub(1))
                .unwrap_or("");
            out.push_str("   |\n");
            out.push_str(&format!("{:>3} | {source_line}\n", label.span.start.line));
            out.push_str(&format!(
                "    | {}{} {}\n",
                " ".repeat(label.span.start.column.saturating_sub(1)),
                "-".repeat(span_width(label.span)),
                label.message
            ));
        }
    }

    out.push_str("   |\n");
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
        let span = Span::new(SourcePos::new(2, 5), SourcePos::new(2, 8));
        let other = Span::new(SourcePos::new(1, 1), SourcePos::new(1, 4));
        let diagnostic = Diagnostic::new("cannot assign String to Integer", Some(span))
            .with_code(DiagnosticCode::TYPE_MISMATCH)
            .with_primary_label("expected Integer, found String")
            .with_secondary_label(other, "variable declared here")
            .with_note("assignment types must match")
            .with_help("change the variable type or assign an Integer value");

        let rendered = diagnostic.render("test.valo", "Dim age As Integer\n    age = \"Valo\"");

        assert!(rendered.contains("error[V1100]: cannot assign String to Integer"));
        assert!(rendered.contains("--> test.valo:2:5"));
        assert!(rendered.contains("expected Integer, found String"));
        assert!(rendered.contains("variable declared here"));
        assert!(rendered.contains("note: assignment types must match"));
        assert!(rendered.contains("help: change the variable type"));
    }
}
