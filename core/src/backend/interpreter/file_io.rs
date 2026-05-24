use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::runtime::{Diagnostic, Span, Value};
use crate::{AssignTarget, Expr, FileAccess, FileLock, OpenMode, PrintItem, PrintSeparator};

use super::{Frame, Interpreter};

#[derive(Debug, Default)]
pub(crate) struct FileIoState {
    files: HashMap<i32, OpenFile>,
    dir_state: Option<DirState>,
}

#[derive(Debug)]
struct OpenFile {
    mode: OpenMode,
    access: FileAccess,
    _lock: Option<FileLock>,
    path: PathBuf,
    content: Vec<u8>,
    position: usize,
    record_len: Option<usize>,
    writer: Option<File>,
}

#[derive(Debug)]
struct DirState {
    entries: Vec<String>,
    index: usize,
}

pub(crate) struct OpenFileRequest<'a> {
    pub(crate) path: &'a Expr,
    pub(crate) mode: OpenMode,
    pub(crate) access: Option<FileAccess>,
    pub(crate) lock: Option<FileLock>,
    pub(crate) shared: bool,
    pub(crate) number: &'a Expr,
    pub(crate) record_len: Option<&'a Expr>,
    pub(crate) span: Span,
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
        request: OpenFileRequest<'_>,
        frame: &mut Frame,
    ) -> Result<(), Diagnostic> {
        let OpenFileRequest {
            path,
            mode,
            access,
            lock,
            shared: _shared,
            number,
            record_len,
            span,
        } = request;
        let number = self.eval_file_number(number, frame)?;
        if self.file_io.files.contains_key(&number) {
            return Err(file_error(
                format!("File number #{} is already open", number),
                span,
            ));
        }
        let path = PathBuf::from(self.eval_expr(path, frame)?.to_output_string());
        let access = access.unwrap_or_else(|| default_access(mode));
        let record_len = if let Some(record_len) = record_len {
            let len = self.eval_integer_expr(record_len, frame, "Open Len must be Integer")?;
            if len <= 0 {
                return Err(file_error(
                    "Open Len must be greater than 0",
                    record_len.span,
                ));
            }
            Some(len as usize)
        } else {
            None
        };
        if record_len.is_some() && mode != OpenMode::Random {
            return Err(file_error(
                "Open Len is only supported with Random mode",
                span,
            ));
        }
        if mode == OpenMode::Random && record_len.is_none() {
            return Err(file_error("Open For Random requires Len =", span));
        }
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
                    access,
                    _lock: lock,
                    path,
                    content,
                    position: 0,
                    record_len,
                    writer: None,
                }
            }
            OpenMode::Output => {
                let writer = if can_write(access) {
                    Some(
                        OpenOptions::new()
                            .create(true)
                            .truncate(true)
                            .write(true)
                            .open(&path)
                            .map_err(|error| {
                                file_error(
                                    format!(
                                        "Unable to open '{}' For Output: {}",
                                        path.display(),
                                        error
                                    ),
                                    span,
                                )
                            })?,
                    )
                } else {
                    None
                };
                OpenFile {
                    mode,
                    access,
                    _lock: lock,
                    path,
                    content: Vec::new(),
                    position: 0,
                    record_len,
                    writer,
                }
            }
            OpenMode::Append => {
                let writer = if can_write(access) {
                    Some(
                        OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&path)
                            .map_err(|error| {
                                file_error(
                                    format!(
                                        "Unable to open '{}' For Append: {}",
                                        path.display(),
                                        error
                                    ),
                                    span,
                                )
                            })?,
                    )
                } else {
                    None
                };
                let position = fs::metadata(&path).map(|m| m.len() as usize).unwrap_or(0);
                OpenFile {
                    mode,
                    access,
                    _lock: lock,
                    path,
                    content: Vec::new(),
                    position,
                    record_len,
                    writer,
                }
            }
            OpenMode::Binary | OpenMode::Random => {
                let writer = OpenOptions::new()
                    .create(can_write(access))
                    .truncate(false)
                    .read(can_read(access))
                    .write(can_write(access))
                    .open(&path)
                    .map_err(|error| {
                        file_error(
                            format!(
                                "Unable to open '{}' For {:?}: {}",
                                path.display(),
                                mode,
                                error
                            ),
                            span,
                        )
                    })?;
                let content = if can_read(access) {
                    fs::read(&path).unwrap_or_default()
                } else {
                    Vec::new()
                };
                OpenFile {
                    mode,
                    access,
                    _lock: lock,
                    path,
                    content,
                    position: 0,
                    record_len,
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
            if !can_read(file.access)
                || file.mode == OpenMode::Output
                || file.mode == OpenMode::Append
            {
                return Err(file_error(
                    format!(
                        "Line Input requires file #{} to be open for reading",
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
                if !can_read(file.access)
                    || file.mode == OpenMode::Output
                    || file.mode == OpenMode::Append
                {
                    return Err(file_error(
                        format!("Input requires file #{} to be open for reading", number),
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
        trailing: Option<PrintSeparator>,
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
        match trailing {
            Some(PrintSeparator::Comma) => text.push('\t'),
            Some(PrintSeparator::Semicolon) => {}
            Some(PrintSeparator::None) | None => text.push('\n'),
        }
        self.write_file_text(number, &text, span)
    }

    pub(crate) fn get_file(
        &mut self,
        number: &Expr,
        position: Option<&Expr>,
        target: &AssignTarget,
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let number = self.eval_file_number(number, frame)?;
        let position = match position {
            Some(position) => {
                Some(self.eval_integer_expr(position, frame, "Get position must be Integer")?)
            }
            None => None,
        };
        let current = self.read_assign_target_value(target, frame, span)?;
        let value = {
            let file = self.open_file_mut(number, span)?;
            ensure_can_read_file(file, number, "Get", span)?;
            let offset = file_offset(file, position, "Get", span)?;
            let (value, consumed) =
                deserialize_value(&file.content, offset, &current, file.record_len, span)?;
            file.position = offset + consumed;
            value
        };
        self.assign_target(target, value, frame, span)
    }

    pub(crate) fn put_file(
        &mut self,
        number: &Expr,
        position: Option<&Expr>,
        expr: &Expr,
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let number = self.eval_file_number(number, frame)?;
        let position = match position {
            Some(position) => {
                Some(self.eval_integer_expr(position, frame, "Put position must be Integer")?)
            }
            None => None,
        };
        let value = self.eval_expr(expr, frame)?;
        let value = self.resolve_default_value(value, frame, expr.span)?;
        let mut bytes = serialize_value(&value, span)?;
        let file = self.open_file_mut(number, span)?;
        ensure_can_write_file(file, number, "Put", span)?;
        let offset = file_offset(file, position, "Put", span)?;
        if let Some(record_len) = file.record_len {
            if bytes.len() > record_len {
                bytes.truncate(record_len);
            } else if bytes.len() < record_len {
                bytes.resize(record_len, 0);
            }
        }
        let writer = file
            .writer
            .as_mut()
            .ok_or_else(|| file_error(format!("File #{} is not open for writing", number), span))?;
        writer
            .seek(SeekFrom::Start(offset as u64))
            .map_err(|error| file_error(format!("Seek failed for #{}: {}", number, error), span))?;
        writer
            .write_all(&bytes)
            .map_err(|error| file_error(format!("Put failed for #{}: {}", number, error), span))?;
        file.position = offset + bytes.len();
        if file.content.len() < file.position {
            file.content.resize(file.position, 0);
        }
        file.content[offset..offset + bytes.len()].copy_from_slice(&bytes);
        Ok(())
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
        if !can_seek(file.mode) {
            return Err(file_error(
                format!(
                    "Seek is not supported for file #{} in {:?} mode",
                    number, file.mode
                ),
                span,
            ));
        }
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
                let entries = collect_dir_entries(&pattern, attrs, span)?;
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
        ensure_can_write_file(file, number, "Write", span)?;
        if !matches!(
            file.mode,
            OpenMode::Output | OpenMode::Append | OpenMode::Binary | OpenMode::Random
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

    fn read_assign_target_value(
        &mut self,
        target: &AssignTarget,
        frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        match target {
            AssignTarget::Variable { name, .. } => frame.get(name, span),
            AssignTarget::ArrayElement { name, indices, .. } => {
                let mut args = Vec::with_capacity(indices.len());
                for index in indices {
                    args.push(self.eval_expr(index, frame)?);
                }
                self.eval_index_expr(frame.get(name, span)?, indices, frame, span)
            }
            AssignTarget::Member { object, field, .. } => {
                let object = self.eval_expr(object, frame)?;
                self.read_member(&object, field, frame, span)
            }
            AssignTarget::MemberArrayElement { object, field, .. } => {
                let object = self.eval_expr(object, frame)?;
                self.read_member(&object, field, frame, span)
            }
        }
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

fn default_access(mode: OpenMode) -> FileAccess {
    match mode {
        OpenMode::Input => FileAccess::Read,
        OpenMode::Output | OpenMode::Append => FileAccess::Write,
        OpenMode::Binary | OpenMode::Random => FileAccess::ReadWrite,
    }
}

fn can_read(access: FileAccess) -> bool {
    matches!(access, FileAccess::Read | FileAccess::ReadWrite)
}

fn can_write(access: FileAccess) -> bool {
    matches!(access, FileAccess::Write | FileAccess::ReadWrite)
}

fn can_seek(mode: OpenMode) -> bool {
    matches!(mode, OpenMode::Binary | OpenMode::Random)
}

fn ensure_can_read_file(
    file: &OpenFile,
    number: i32,
    operation: &str,
    span: Span,
) -> Result<(), Diagnostic> {
    if !can_read(file.access) {
        return Err(file_error(
            format!("{operation} cannot read from write-only file #{}", number),
            span,
        ));
    }
    if !matches!(file.mode, OpenMode::Binary | OpenMode::Random) {
        return Err(file_error(
            format!(
                "{operation} requires file #{} to be opened For Binary or Random",
                number
            ),
            span,
        ));
    }
    Ok(())
}

fn ensure_can_write_file(
    file: &OpenFile,
    number: i32,
    operation: &str,
    span: Span,
) -> Result<(), Diagnostic> {
    if !can_write(file.access) {
        return Err(file_error(
            format!("{operation} cannot write to read-only file #{}", number),
            span,
        ));
    }
    Ok(())
}

fn file_offset(
    file: &OpenFile,
    position: Option<i64>,
    operation: &str,
    span: Span,
) -> Result<usize, Diagnostic> {
    match file.mode {
        OpenMode::Random => {
            let record_len = file
                .record_len
                .ok_or_else(|| file_error("Random mode requires Len =", span))?;
            let record = position.unwrap_or((file.position / record_len) as i64 + 1);
            if record < 1 {
                return Err(file_error(
                    format!("{operation} record number must be at least 1"),
                    span,
                ));
            }
            Ok((record as usize - 1) * record_len)
        }
        _ => {
            let offset = position.unwrap_or(file.position as i64 + 1);
            if offset < 1 {
                return Err(file_error(
                    format!("{operation} position must be at least 1"),
                    span,
                ));
            }
            Ok(offset as usize - 1)
        }
    }
}

fn serialize_value(value: &Value, span: Span) -> Result<Vec<u8>, Diagnostic> {
    match value {
        Value::Byte(value) => Ok(vec![*value]),
        Value::Int16(value) => Ok(value.to_le_bytes().to_vec()),
        Value::Int32(value) => Ok(value.to_le_bytes().to_vec()),
        Value::Int64(value) => Ok(value.to_le_bytes().to_vec()),
        Value::Single(value) => Ok(value.to_le_bytes().to_vec()),
        Value::Double(value) => Ok(value.to_le_bytes().to_vec()),
        Value::Boolean(value) => {
            let raw: i16 = if *value { -1 } else { 0 };
            Ok(raw.to_le_bytes().to_vec())
        }
        Value::String(value) => Ok(value.as_bytes().to_vec()),
        Value::Array(array)
            if array
                .element_type
                .same_type(&crate::runtime::TypeName::Byte) =>
        {
            let mut bytes = Vec::with_capacity(array.elements.len());
            for value in &array.elements {
                let Value::Byte(byte) = value else {
                    return Err(file_error("Put only supports Byte arrays", span));
                };
                bytes.push(*byte);
            }
            Ok(bytes)
        }
        _ => Err(file_error(
            format!(
                "Binary serialization is not supported for {}",
                value.type_name().display_name()
            ),
            span,
        )),
    }
}

fn deserialize_value(
    content: &[u8],
    offset: usize,
    target: &Value,
    record_len: Option<usize>,
    span: Span,
) -> Result<(Value, usize), Diagnostic> {
    match target {
        Value::Byte(_) => Ok((Value::Byte(read_fixed::<1>(content, offset, span)?[0]), 1)),
        Value::Int16(_) => {
            let bytes = read_fixed::<2>(content, offset, span)?;
            Ok((Value::Int16(i16::from_le_bytes(bytes)), 2))
        }
        Value::Int32(_) => {
            let bytes = read_fixed::<4>(content, offset, span)?;
            Ok((Value::Int32(i32::from_le_bytes(bytes)), 4))
        }
        Value::Int64(_) => {
            let bytes = read_fixed::<8>(content, offset, span)?;
            Ok((Value::Int64(i64::from_le_bytes(bytes)), 8))
        }
        Value::Single(_) => {
            let bytes = read_fixed::<4>(content, offset, span)?;
            Ok((Value::Single(f32::from_le_bytes(bytes)), 4))
        }
        Value::Double(_) => {
            let bytes = read_fixed::<8>(content, offset, span)?;
            Ok((Value::Double(f64::from_le_bytes(bytes)), 8))
        }
        Value::Boolean(_) => {
            let bytes = read_fixed::<2>(content, offset, span)?;
            Ok((Value::Boolean(i16::from_le_bytes(bytes) != 0), 2))
        }
        Value::String(_) => {
            let len = record_len.unwrap_or_else(|| content.len().saturating_sub(offset));
            ensure_available(content, offset, len, span)?;
            let bytes = &content[offset..offset + len];
            let text = String::from_utf8_lossy(bytes)
                .trim_end_matches('\0')
                .to_string();
            Ok((Value::String(text), len))
        }
        _ => Err(file_error(
            format!(
                "Binary deserialization is not supported for {}",
                target.type_name().display_name()
            ),
            span,
        )),
    }
}

fn read_fixed<const N: usize>(
    content: &[u8],
    offset: usize,
    span: Span,
) -> Result<[u8; N], Diagnostic> {
    ensure_available(content, offset, N, span)?;
    let mut bytes = [0; N];
    bytes.copy_from_slice(&content[offset..offset + N]);
    Ok(bytes)
}

fn ensure_available(
    content: &[u8],
    offset: usize,
    len: usize,
    span: Span,
) -> Result<(), Diagnostic> {
    if offset
        .checked_add(len)
        .is_some_and(|end| end <= content.len())
    {
        Ok(())
    } else {
        Err(file_error("Get attempted to read past end of file", span))
    }
}

fn collect_dir_entries(pattern: &str, attrs: i64, span: Span) -> Result<Vec<String>, Diagnostic> {
    let include_dirs = attrs & 16 != 0;
    let include_hidden = attrs & 2 != 0;
    let require_readonly = attrs & 1 != 0;
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
        if require_readonly && !metadata.permissions().readonly() {
            return Ok(Vec::new());
        }
        if is_hidden_path(path) && !include_hidden {
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
        let metadata = entry
            .metadata()
            .map_err(|error| file_error(format!("Dir failed: {}", error), span))?;
        if require_readonly && !metadata.permissions().readonly() {
            continue;
        }
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if is_hidden_path(&entry_path) && !include_hidden {
            continue;
        }
        if wildcard_match(file_pattern, &name) {
            entries.push(name);
        }
    }
    entries.sort();
    Ok(entries)
}

fn is_hidden_path(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        if std::fs::metadata(path)
            .map(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
            .unwrap_or(false)
        {
            return true;
        }
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
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
