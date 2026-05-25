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

    pub fn with_name_suggestion<'a>(
        self,
        misspelled: &str,
        candidates: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        if let Some(candidate) = suggest_name(misspelled, candidates) {
            self.with_help(format!("did you mean '{candidate}'?"))
        } else {
            self
        }
    }

    pub fn with_available_items<'a>(
        mut self,
        label: &str,
        items: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let mut items: Vec<_> = items.into_iter().collect();
        items.sort_unstable_by_key(|item| item.to_ascii_lowercase());
        items.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        if !items.is_empty() {
            let mut note = String::from(label);
            note.push(':');
            for item in items {
                note.push_str("\n  - ");
                note.push_str(item);
            }
            self.notes.push(note);
        }
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
        let gutter_width = diagnostic_gutter_width(self);

        out.push_str(&format!(
            "{}[{}]: {}{}{}\n",
            color.severity(self.severity),
            self.code,
            color.bold(""),
            self.message,
            color.reset()
        ));

        if let Some(span) = &self.span {
            let source_name = source_map.get_name(span.file_id).unwrap_or("<unknown>");
            out.push_str(&format!(
                "{}{}--> {}{}:{}:{}{}\n",
                " ".repeat(gutter_width),
                color.gutter(),
                color.bold(""),
                source_name,
                span.start.line,
                span.start.column,
                color.reset()
            ));
            render_empty_gutter(&mut out, gutter_width, &color);

            if let Some(source) = source_map.get_content(span.file_id) {
                render_span_lines(&mut out, source, **span, &self.labels, &color, gutter_width);
            }
        }

        for note in self.notes.iter() {
            out.push_str(&format!(
                "{}{}= {}note{}: {}\n",
                " ".repeat(gutter_width),
                color.gutter(),
                color.note(),
                color.reset(),
                note
            ));
        }
        for help in self.helps.iter() {
            out.push_str(&format!(
                "{}{}= {}help{}: {}\n",
                " ".repeat(gutter_width),
                color.gutter(),
                color.help(),
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
}

pub fn suggest_name<'a>(
    misspelled: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let needle = misspelled.to_ascii_lowercase();
    let max_distance = (needle.chars().count() / 3).clamp(2, 3);
    candidates
        .into_iter()
        .filter(|candidate| !candidate.is_empty())
        .map(|candidate| {
            (
                candidate,
                edit_distance(&needle, &candidate.to_ascii_lowercase()),
            )
        })
        .filter(|(_, distance)| *distance <= max_distance)
        .min_by_key(|(candidate, distance)| (*distance, candidate.len()))
        .map(|(candidate, _)| candidate.to_string())
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_ch) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_ch) in right_chars.iter().enumerate() {
            let substitution_cost = usize::from(left_ch != *right_ch);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution_cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_chars.len()]
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

    fn gutter(&self) -> &str {
        if self.enabled { "\x1b[34;1m" } else { "" }
    }

    fn note(&self) -> &str {
        if self.enabled { "\x1b[36;1m" } else { "" }
    }

    fn help(&self) -> &str {
        if self.enabled { "\x1b[32;1m" } else { "" }
    }

    fn primary(&self) -> &str {
        if self.enabled { "\x1b[31;1m" } else { "" }
    }

    fn secondary(&self) -> &str {
        if self.enabled { "\x1b[33;1m" } else { "" }
    }

    fn severity(&self, severity: Severity) -> String {
        if !self.enabled {
            return severity.to_string();
        }
        let code = match severity {
            Severity::Error => "\x1b[31;1m",
            Severity::Warning => "\x1b[33;1m",
            Severity::Note => "\x1b[36;1m",
            Severity::Help => "\x1b[32;1m",
        };
        format!("{code}{severity}\x1b[0m")
    }

    fn reset(&self) -> &str {
        if self.enabled { "\x1b[0m" } else { "" }
    }
}

fn diagnostic_gutter_width(diagnostic: &Diagnostic) -> usize {
    max_diagnostic_line(diagnostic).to_string().len().max(1)
}

fn max_diagnostic_line(diagnostic: &Diagnostic) -> usize {
    let mut max_line = diagnostic
        .span
        .as_ref()
        .map(|span| span.start.line.max(span.end.line))
        .unwrap_or(0);

    for label in diagnostic.labels.iter() {
        max_line = max_line.max(label.span.start.line).max(label.span.end.line);
    }
    for related in diagnostic.related.iter() {
        max_line = max_line.max(max_diagnostic_line(related));
    }

    max_line
}

fn render_empty_gutter(out: &mut String, gutter_width: usize, color: &ColorSupport) {
    out.push_str(&format!(
        "{}{} |{}\n",
        " ".repeat(gutter_width),
        color.gutter(),
        color.reset()
    ));
}

fn render_span_lines(
    out: &mut String,
    source: &str,
    primary: Span,
    labels: &[DiagnosticLabel],
    color: &ColorSupport,
    gutter_width: usize,
) {
    let primary_label = labels
        .iter()
        .find(|label| label.style == LabelStyle::Primary && label.span == primary)
        .map(|label| label.message.as_str())
        .unwrap_or("");

    render_labeled_span(
        out,
        source,
        primary,
        primary_label,
        LabelStyle::Primary,
        color,
        gutter_width,
    );

    for label in labels
        .iter()
        .filter(|label| label.style == LabelStyle::Secondary)
    {
        render_empty_gutter(out, gutter_width, color);
        render_labeled_span(
            out,
            source,
            label.span,
            &label.message,
            LabelStyle::Secondary,
            color,
            gutter_width,
        );
    }

    render_empty_gutter(out, gutter_width, color);
}

fn render_labeled_span(
    out: &mut String,
    source: &str,
    span: Span,
    label: &str,
    style: LabelStyle,
    color: &ColorSupport,
    gutter_width: usize,
) {
    let source_line = source
        .lines()
        .nth(span.start.line.saturating_sub(1))
        .unwrap_or("");
    let displayed_line = expand_tabs(source_line);
    let marker_offset = visual_offset_for_column(source_line, span.start.column);
    let marker_width = visual_span_width(source_line, span).max(1);
    let marker = match style {
        LabelStyle::Primary => "^",
        LabelStyle::Secondary => "-",
    };
    let marker_color = match style {
        LabelStyle::Primary => color.primary(),
        LabelStyle::Secondary => color.secondary(),
    };
    let label_suffix = if label.is_empty() {
        String::new()
    } else {
        format!(" {label}")
    };

    out.push_str(&format!(
        "{}{:>width$} |{} {}\n",
        color.gutter(),
        span.start.line,
        color.reset(),
        displayed_line,
        width = gutter_width
    ));
    out.push_str(&format!(
        "{}{} |{} {}{}{}{}{}\n",
        " ".repeat(gutter_width),
        color.gutter(),
        color.reset(),
        " ".repeat(marker_offset),
        marker_color,
        marker.repeat(marker_width),
        color.reset(),
        label_suffix
    ));
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
    pub const MEMBER_IS_PRIVATE: Self = Self("V1003");
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
    pub const RUNTIME_ERROR: Self = Self("V9001");
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

fn visual_span_width(source_line: &str, span: Span) -> usize {
    if span.start.line == span.end.line {
        let start = visual_offset_for_column(source_line, span.start.column);
        let end = visual_offset_for_column(source_line, span.end.column);
        end.saturating_sub(start).max(1)
    } else {
        1
    }
}

fn visual_offset_for_column(source_line: &str, column: usize) -> usize {
    source_line
        .chars()
        .take(column.saturating_sub(1))
        .fold(0, |offset, ch| offset + char_width(ch, offset))
}

fn expand_tabs(source_line: &str) -> String {
    let mut expanded = String::with_capacity(source_line.len());
    let mut offset = 0;
    for ch in source_line.chars() {
        if ch == '\t' {
            let width = char_width(ch, offset);
            expanded.push_str(&" ".repeat(width));
            offset += width;
        } else {
            expanded.push(ch);
            offset += char_width(ch, offset);
        }
    }
    expanded
}

fn char_width(ch: char, offset: usize) -> usize {
    if ch == '\t' { 4 - (offset % 4) } else { 1 }
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

    #[test]
    fn renders_wide_line_gutters_without_shifting_bars() {
        let mut source_map = SourceMap::new();
        let mut source = "\n".repeat(999);
        source.push_str("Dim answer As Integer\n");
        let file_id = source_map.add("large.valo".to_string(), source);
        let span = Span::new(file_id, SourcePos::new(1000, 5), SourcePos::new(1000, 11));
        let diagnostic = Diagnostic::new(DiagnosticCode::TYPE_MISMATCH, "bad value", Some(span))
            .with_primary_label("expected Integer");

        let rendered = diagnostic.render_colored(&source_map, false);

        assert!(rendered.contains("    |"));
        assert!(rendered.contains("1000 | Dim answer As Integer"));
        assert!(rendered.contains("    |     ^^^^^^ expected Integer"));
    }

    #[test]
    fn expands_tabs_before_rendering_caret_markers() {
        let mut source_map = SourceMap::new();
        let file_id = source_map.add("tabs.valo".to_string(), "\tvalue = \"x\"".to_string());
        let span = Span::new(file_id, SourcePos::new(1, 2), SourcePos::new(1, 7));
        let diagnostic = Diagnostic::new(DiagnosticCode::TYPE_MISMATCH, "bad value", Some(span))
            .with_primary_label("here");

        let rendered = diagnostic.render_colored(&source_map, false);

        assert!(rendered.contains("1 |     value = \"x\""));
        assert!(rendered.contains("  |     ^^^^^ here"));
    }
}
