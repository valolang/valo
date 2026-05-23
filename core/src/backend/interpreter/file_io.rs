use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::runtime::{Diagnostic, Span, Value};
use crate::{AssignTarget, Expr, OpenMode, PrintItem, PrintSeparator};

use super::{Frame, Interpreter};

#[derive(Debug, Default)]
pub(crate) struct FileIoState {
    files: HashMap<i32, OpenFile>,
    dir_state: Option<DirState>,
}

#[derive(Debug)]
struct OpenFile {
    mode: OpenMode,
    path: PathBuf,
    content: Vec<u8>,
    position: usize,
    writer: Option<File>,
}

#[derive(Debug)]
struct DirState {
    entries: Vec<String>,
    index: usize,
}

impl FileIoState {
    fn free_file(&self) -> i32 {
        (1..=255)
            .find(|number| !self.files.contains_key(number))
            .unwrap_or(256)
    }
}

impl Interpreter {
    pub(crate) fn free_file_number(&self) -> i32 {
        self.file_io.free_file()
    }

    pub(crate) fn open_file(
        &mut self,
        path: &Expr,
        mode: OpenMode,
        number: &Expr,
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let number = self.eval_file_number(number, frame)?;
        if self.file_io.files.contains_key(&number) {
            return Err(file_error(
                format!("File number #{} is already open", number),
                span,
            ));
        }
        let path = PathBuf::from(self.eval_expr(path, frame)?.to_output_string());
        let opened = match mode {
            OpenMode::Input => {
                let content = fs::read(&path).map_err(|error| {
                    file_error(
                        format!("Unable to open '{}' For Input: {}", path.display(), error),
                        span,
                    )
                })?;
                OpenFile {
                    mode,
                    path,
                    content,
                    position: 0,
                    writer: None,
                }
            }
            OpenMode::Output => {
                let writer = OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(&path)
                    .map_err(|error| {
                        file_error(
                            format!("Unable to open '{}' For Output: {}", path.display(), error),
                            span,
                        )
                    })?;
                OpenFile {
                    mode,
                    path,
                    content: Vec::new(),
                    position: 0,
                    writer: Some(writer),
                }
            }
            OpenMode::Append => {
                let writer = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .map_err(|error| {
                        file_error(
                            format!("Unable to open '{}' For Append: {}", path.display(), error),
                            span,
                        )
                    })?;
                let position = fs::metadata(&path).map(|m| m.len() as usize).unwrap_or(0);
                OpenFile {
                    mode,
                    path,
                    content: Vec::new(),
                    position,
                    writer: Some(writer),
                }
            }
            OpenMode::Binary => {
                let writer = OpenOptions::new()
                    .create(true)
                    .truncate(false)
                    .read(true)
                    .write(true)
                    .open(&path)
                    .map_err(|error| {
                        file_error(
                            format!("Unable to open '{}' For Binary: {}", path.display(), error),
                            span,
                        )
                    })?;
                let content = fs::read(&path).unwrap_or_default();
                OpenFile {
                    mode,
                    path,
                    content,
                    position: 0,
                    writer: Some(writer),
                }
            }
        };
        self.file_io.files.insert(number, opened);
        Ok(())
    }

    pub(crate) fn close_files(
        &mut self,
        numbers: &[Expr],
        frame: &mut Frame,
    ) -> Result<(), Diagnostic> {
        if numbers.is_empty() {
            self.file_io.files.clear();
            return Ok(());
        }
        for number in numbers {
            let number = self.eval_file_number(number, frame)?;
            self.file_io.files.remove(&number);
        }
        Ok(())
    }

    pub(crate) fn line_input_file(
        &mut self,
        number: &Expr,
        target: &AssignTarget,
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let number = self.eval_file_number(number, frame)?;
        let line = {
            let file = self.open_file_mut(number, span)?;
            if file.mode != OpenMode::Input {
                return Err(file_error(
                    format!(
                        "Line Input requires file #{} to be opened For Input",
                        number
                    ),
                    span,
                ));
            }
            read_line(file, number, span)?
        };
        self.assign_target(target, Value::String(line), frame, span)
    }

    pub(crate) fn input_file(
        &mut self,
        number: &Expr,
        targets: &[AssignTarget],
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let number = self.eval_file_number(number, frame)?;
        for target in targets {
            let token = {
                let file = self.open_file_mut(number, span)?;
                if file.mode != OpenMode::Input {
                    return Err(file_error(
                        format!("Input requires file #{} to be opened For Input", number),
                        span,
                    ));
                }
                read_input_token(file, number, span)?
            };
            self.assign_target(target, parse_input_value(&token), frame, span)?;
        }
        Ok(())
    }

    pub(crate) fn print_file(
        &mut self,
        number: &Expr,
        items: &[PrintItem],
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let number = self.eval_file_number(number, frame)?;
        let mut text = String::new();
        for item in items {
            match item.separator {
                PrintSeparator::None => {}
                PrintSeparator::Comma => text.push('\t'),
                PrintSeparator::Semicolon => {}
            }
            let value = self.eval_expr(&item.expr, frame)?;
            let value = self.resolve_default_value(value, frame, item.expr.span)?;
            text.push_str(&value.to_output_string());
        }
        text.push('\n');
        self.write_file_text(number, &text, span)
    }

    pub(crate) fn write_file(
        &mut self,
        number: &Expr,
        args: &[Expr],
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let number = self.eval_file_number(number, frame)?;
        let mut parts = Vec::new();
        for arg in args {
            let value = self.eval_expr(arg, frame)?;
            let value = self.resolve_default_value(value, frame, arg.span)?;
            parts.push(write_field(&value));
        }
        let mut text = parts.join(",");
        text.push('\n');
        self.write_file_text(number, &text, span)
    }

    pub(crate) fn seek_file_statement(
        &mut self,
        number: &Expr,
        position: &Expr,
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let number = self.eval_file_number(number, frame)?;
        let position = self.eval_integer_expr(position, frame, "Seek position must be Integer")?;
        self.seek_file(number, position, span)
    }

    pub(crate) fn eof_file(&self, number: i32, span: Span) -> Result<bool, Diagnostic> {
        let file = self.opened_file(number, span)?;
        Ok(file.position >= file.content.len())
    }

    pub(crate) fn lof_file(&self, number: i32, span: Span) -> Result<i64, Diagnostic> {
        let file = self.opened_file(number, span)?;
        let len = if file.content.is_empty() {
            fs::metadata(&file.path).map(|m| m.len()).unwrap_or(0)
        } else {
            file.content.len() as u64
        };
        Ok(len as i64)
    }

    pub(crate) fn seek_file_position(&self, number: i32, span: Span) -> Result<i64, Diagnostic> {
        let file = self.opened_file(number, span)?;
        Ok(file.position as i64 + 1)
    }

    pub(crate) fn seek_file(
        &mut self,
        number: i32,
        position: i64,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if position < 1 {
            return Err(file_error("Seek position must be at least 1", span));
        }
        let file = self.open_file_mut(number, span)?;
        file.position = (position - 1) as usize;
        if let Some(writer) = &mut file.writer {
            writer
                .seek(SeekFrom::Start((position - 1) as u64))
                .map_err(|error| {
                    file_error(format!("Seek failed for #{}: {}", number, error), span)
                })?;
        }
        Ok(())
    }

    pub(crate) fn dir(&mut self, args: &[Value], span: Span) -> Result<String, Diagnostic> {
        match args.len() {
            0 => {
                let Some(state) = &mut self.file_io.dir_state else {
                    return Ok(String::new());
                };
                Ok(next_dir_entry(state))
            }
            1 | 2 => {
                let pattern = args[0].to_output_string();
                let attrs = args
                    .get(1)
                    .and_then(crate::runtime::numeric::value_to_i64)
                    .unwrap_or(0);
                let include_dirs = attrs & 16 != 0;
                let entries = collect_dir_entries(&pattern, include_dirs, span)?;
                self.file_io.dir_state = Some(DirState { entries, index: 0 });
                let state = self.file_io.dir_state.as_mut().expect("just initialized");
                Ok(next_dir_entry(state))
            }
            _ => Err(file_error("Dir expects 0 to 2 arguments", span)),
        }
    }

    pub(crate) fn kill_path(&mut self, path: &str, span: Span) -> Result<(), Diagnostic> {
        fs::remove_file(path)
            .map_err(|error| file_error(format!("Unable to Kill '{}': {}", path, error), span))
    }

    pub(crate) fn mkdir_path(&mut self, path: &str, span: Span) -> Result<(), Diagnostic> {
        fs::create_dir(path)
            .map_err(|error| file_error(format!("Unable to MkDir '{}': {}", path, error), span))
    }

    pub(crate) fn rmdir_path(&mut self, path: &str, span: Span) -> Result<(), Diagnostic> {
        fs::remove_dir(path)
            .map_err(|error| file_error(format!("Unable to RmDir '{}': {}", path, error), span))
    }

    pub(crate) fn chdir_path(&mut self, path: &str, span: Span) -> Result<(), Diagnostic> {
        std::env::set_current_dir(path)
            .map_err(|error| file_error(format!("Unable to ChDir '{}': {}", path, error), span))
    }

    pub(crate) fn name_file(
        &mut self,
        old_path: &Expr,
        new_path: &Expr,
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let old_path = self.eval_expr(old_path, frame)?.to_output_string();
        let new_path = self.eval_expr(new_path, frame)?.to_output_string();
        fs::rename(&old_path, &new_path).map_err(|error| {
            file_error(
                format!(
                    "Unable to rename '{}' As '{}': {}",
                    old_path, new_path, error
                ),
                span,
            )
        })
    }

    fn write_file_text(&mut self, number: i32, text: &str, span: Span) -> Result<(), Diagnostic> {
        let file = self.open_file_mut(number, span)?;
        if !matches!(
            file.mode,
            OpenMode::Output | OpenMode::Append | OpenMode::Binary
        ) {
            return Err(file_error(
                format!("File #{} is not open for writing", number),
                span,
            ));
        }
        let writer = file
            .writer
            .as_mut()
            .ok_or_else(|| file_error(format!("File #{} is not open for writing", number), span))?;
        writer.write_all(text.as_bytes()).map_err(|error| {
            file_error(format!("Write failed for #{}: {}", number, error), span)
        })?;
        file.position += text.len();
        Ok(())
    }

    fn eval_file_number(&mut self, expr: &Expr, frame: &mut Frame) -> Result<i32, Diagnostic> {
        let number = self.eval_integer_expr(expr, frame, "File number must be Integer")?;
        if !(1..=511).contains(&number) {
            return Err(file_error(
                "File number must be between 1 and 511",
                expr.span,
            ));
        }
        Ok(number as i32)
    }

    fn opened_file(&self, number: i32, span: Span) -> Result<&OpenFile, Diagnostic> {
        self.file_io
            .files
            .get(&number)
            .ok_or_else(|| file_error(format!("File number #{} is not open", number), span))
    }

    fn open_file_mut(&mut self, number: i32, span: Span) -> Result<&mut OpenFile, Diagnostic> {
        self.file_io
            .files
            .get_mut(&number)
            .ok_or_else(|| file_error(format!("File number #{} is not open", number), span))
    }
}

fn read_line(file: &mut OpenFile, number: i32, span: Span) -> Result<String, Diagnostic> {
    if file.position >= file.content.len() {
        return Err(file_error(
            format!("Input past end of file #{}", number),
            span,
        ));
    }
    let rest = &file.content[file.position..];
    let newline = rest.iter().position(|byte| *byte == b'\n');
    let (line_bytes, consumed) = if let Some(index) = newline {
        (&rest[..index], index + 1)
    } else {
        (rest, rest.len())
    };
    file.position += consumed;
    let mut line = String::from_utf8_lossy(line_bytes).to_string();
    if line.ends_with('\r') {
        line.pop();
    }
    Ok(line)
}

fn read_input_token(file: &mut OpenFile, number: i32, span: Span) -> Result<String, Diagnostic> {
    skip_input_separators(file);
    if file.position >= file.content.len() {
        return Err(file_error(
            format!("Input past end of file #{}", number),
            span,
        ));
    }
    if file.content[file.position] == b'"' {
        file.position += 1;
        let start = file.position;
        while file.position < file.content.len() && file.content[file.position] != b'"' {
            file.position += 1;
        }
        let token = String::from_utf8_lossy(&file.content[start..file.position]).to_string();
        if file.position < file.content.len() {
            file.position += 1;
        }
        Ok(token)
    } else {
        let start = file.position;
        while file.position < file.content.len()
            && !matches!(file.content[file.position], b',' | b'\n' | b'\r')
        {
            file.position += 1;
        }
        Ok(String::from_utf8_lossy(&file.content[start..file.position])
            .trim()
            .to_string())
    }
}

fn skip_input_separators(file: &mut OpenFile) {
    while file.position < file.content.len()
        && matches!(
            file.content[file.position],
            b',' | b'\n' | b'\r' | b' ' | b'\t'
        )
    {
        file.position += 1;
    }
}

fn parse_input_value(token: &str) -> Value {
    if token.eq_ignore_ascii_case("true") || token.eq_ignore_ascii_case("#TRUE#") {
        Value::Boolean(true)
    } else if token.eq_ignore_ascii_case("false") || token.eq_ignore_ascii_case("#FALSE#") {
        Value::Boolean(false)
    } else if token.eq_ignore_ascii_case("null") {
        Value::Null
    } else if token.is_empty() {
        Value::Empty
    } else if let Ok(value) = token.parse::<i64>() {
        Value::Int64(value)
    } else if let Ok(value) = token.parse::<f64>() {
        Value::Double(value)
    } else {
        Value::String(token.to_string())
    }
}

fn write_field(value: &Value) -> String {
    match value {
        Value::String(value) => format!("\"{}\"", value.replace('"', "\"\"")),
        Value::Boolean(true) => "#TRUE#".to_string(),
        Value::Boolean(false) => "#FALSE#".to_string(),
        Value::Null => "#NULL#".to_string(),
        Value::Empty => String::new(),
        value => value.to_output_string(),
    }
}

fn collect_dir_entries(
    pattern: &str,
    include_dirs: bool,
    span: Span,
) -> Result<Vec<String>, Diagnostic> {
    let path = Path::new(pattern);
    let (dir, file_pattern) = if pattern.contains('*') || pattern.contains('?') {
        (
            path.parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("*"),
        )
    } else {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => return Ok(Vec::new()),
        };
        if metadata.is_dir() && !include_dirs {
            return Ok(Vec::new());
        }
        return Ok(path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| vec![name.to_string()])
            .unwrap_or_default());
    };
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).map_err(|error| {
        file_error(
            format!("Unable to read directory '{}': {}", dir.display(), error),
            span,
        )
    })? {
        let entry = entry.map_err(|error| file_error(format!("Dir failed: {}", error), span))?;
        let file_type = entry
            .file_type()
            .map_err(|error| file_error(format!("Dir failed: {}", error), span))?;
        if file_type.is_dir() && !include_dirs {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if wildcard_match(file_pattern, &name) {
            entries.push(name);
        }
    }
    entries.sort();
    Ok(entries)
}

fn next_dir_entry(state: &mut DirState) -> String {
    let Some(entry) = state.entries.get(state.index) else {
        return String::new();
    };
    state.index += 1;
    entry.clone()
}

fn wildcard_match(pattern: &str, name: &str) -> bool {
    wildcard_match_bytes(pattern.as_bytes(), name.as_bytes())
}

fn wildcard_match_bytes(pattern: &[u8], name: &[u8]) -> bool {
    if pattern.is_empty() {
        return name.is_empty();
    }
    match pattern[0] {
        b'*' => {
            wildcard_match_bytes(&pattern[1..], name)
                || (!name.is_empty() && wildcard_match_bytes(pattern, &name[1..]))
        }
        b'?' => !name.is_empty() && wildcard_match_bytes(&pattern[1..], &name[1..]),
        ch => {
            !name.is_empty()
                && ch.eq_ignore_ascii_case(&name[0])
                && wildcard_match_bytes(&pattern[1..], &name[1..])
        }
    }
}

fn file_error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(crate::runtime::DiagnosticCode::GENERIC, message, Some(span))
}
