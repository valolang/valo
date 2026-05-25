//! Valo Builtins
//!
//! Standard library functions and procedures.
//!
//! Value-level builtins are kept backend-neutral where practical. A small
//! dispatch layer still handles lazy/special forms that need AST or frame access
//! (`IIf`, `CallByName`, pointer builtins, and host-owned services).

use super::{ControlFlow, Frame, Interpreter};
use crate::Expr;
use crate::runtime::{Diagnostic, Value};

pub(crate) fn dispatch_stmt(
    interpreter: &mut Interpreter,
    object_name: &str,
    method: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<ControlFlow>, Diagnostic> {
    // Handle VBA namespace fallback: VBA.MsgBox(...) -> MsgBox(...)
    let effective_object_name = if object_name.eq_ignore_ascii_case("VBA") {
        "VBA"
    } else {
        object_name
    };

    if effective_object_name.eq_ignore_ascii_case("Console")
        || effective_object_name.eq_ignore_ascii_case("Debug")
    {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            let val = interpreter.eval_expr(arg, frame)?;
            let resolved = interpreter.resolve_default_value(val, frame, arg.span)?;
            values.push(resolved);
        }

        if effective_object_name.eq_ignore_ascii_case("Console") {
            if let Some(line) = console::exec_console(method, &values, span)? {
                interpreter.output.push(line);
                return Ok(Some(ControlFlow::Continue));
            }
        } else {
            if let Some(line) = debug::exec_debug(method, &values, span)? {
                interpreter.output.push(line);
                return Ok(Some(ControlFlow::Continue));
            }
        }
    }

    if effective_object_name.eq_ignore_ascii_case("Err") {
        return err::exec_err(interpreter, method, args, frame, span);
    }

    if object_name.eq_ignore_ascii_case("VBA") {
        // VBA.Randomize 123
        if let Some(val) = dispatch_function(interpreter, method, args, frame, span)? {
            // If it returns a value but was called as a stmt, we just ignore the value
            // (or maybe check if it's a valid stmt builtin)
            if matches!(val, Value::Empty) || method.eq_ignore_ascii_case("Randomize") {
                return Ok(Some(ControlFlow::Continue));
            }
        }
    }

    Ok(None)
}

pub(crate) fn dispatch_function(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    // Handle VBA namespace fallback: VBA.Join(...) -> Join(...)
    let effective_name = if let Some(stripped) = name.strip_prefix("VBA.") {
        stripped
    } else {
        name
    };

    // Special forms that require lazy evaluation or direct Expr access
    if effective_name.eq_ignore_ascii_case("IIf") {
        expect_arg_count(effective_name, args, 3, span)?;
        let condition = interpreter.eval_expr(&args[0], frame)?.is_truthy();
        let value_expr = if condition { &args[1] } else { &args[2] };
        return Ok(Some(interpreter.eval_expr(value_expr, frame)?));
    }

    if effective_name.eq_ignore_ascii_case("CallByName") {
        return dispatch_callbyname(interpreter, effective_name, args, frame, span);
    }

    if effective_name.eq_ignore_ascii_case("VarPtr") {
        expect_arg_count(effective_name, args, 1, span)?;
        let arg = &args[0];
        let ptr = match &arg.kind {
            crate::ExprKind::Variable(name) => {
                let variable = frame.variable(name, arg.span)?;
                if let super::frame::VariableCell::Direct(cell) = &variable.cell {
                    std::rc::Rc::as_ptr(cell) as usize
                } else {
                    0
                }
            }
            _ => {
                let _ = interpreter.eval_expr(arg, frame)?;
                0
            }
        };
        return Ok(Some(Value::Ptr(ptr)));
    }

    if effective_name.eq_ignore_ascii_case("StrPtr") {
        expect_arg_count(effective_name, args, 1, span)?;
        let arg = &args[0];
        if let crate::ExprKind::Variable(name) = &arg.kind {
            let variable = frame.variable(name, arg.span)?;
            let value = variable.borrow();
            if let Value::String(s) = &*value {
                return Ok(Some(Value::Ptr(s.as_ptr() as usize)));
            }
            return Ok(Some(Value::Ptr(0)));
        }
        let value = interpreter.eval_expr(arg, frame)?;
        let text = match value {
            Value::String(text) => text,
            Value::Empty => String::new(),
            Value::Null | Value::Nothing | Value::Missing => return Ok(Some(Value::Ptr(0))),
            other => other.to_output_string(),
        };
        interpreter.temporary_strings.push(text);
        let ptr = interpreter
            .temporary_strings
            .last()
            .map(|text| text.as_ptr() as usize)
            .unwrap_or(0);
        return Ok(Some(Value::Ptr(ptr)));
    }

    if effective_name.eq_ignore_ascii_case("ObjPtr") {
        expect_arg_count(effective_name, args, 1, span)?;
        let value = interpreter.eval_expr(&args[0], frame)?;
        match value {
            Value::Object(obj) => {
                let ptr = std::rc::Rc::as_ptr(&obj) as usize;
                return Ok(Some(Value::Ptr(ptr)));
            }
            Value::Nothing => {
                return Ok(Some(Value::Ptr(0)));
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "ObjPtr requires an object",
                    Some(span),
                ));
            }
        }
    }

    if effective_name.eq_ignore_ascii_case("CreateObject") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "CreateObject expects 1 to 2 arguments",
                Some(span),
            ));
        }
        let prog_id = interpreter.eval_expr(&args[0], frame)?.to_output_string();
        if args.len() == 2 {
            let server = interpreter.eval_expr(&args[1], frame)?.to_output_string();
            if !server.is_empty() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "CreateObject remote server activation is not supported yet",
                    Some(args[1].span),
                ));
            }
        }
        return Ok(Some(crate::runtime::com::create_object(&prog_id, span)?));
    }

    if matches!(
        effective_name.to_ascii_lowercase().as_str(),
        "freefile" | "eof" | "lof" | "seek" | "dir" | "filelen" | "filedatetime" | "curdir"
    ) {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(interpreter.eval_expr(arg, frame)?);
        }
        return dispatch_file_function(interpreter, effective_name, &values, span);
    }

    if matches!(
        effective_name.to_ascii_lowercase().as_str(),
        "timer"
            | "now"
            | "date"
            | "time"
            | "dateserial"
            | "timeserial"
            | "datevalue"
            | "timevalue"
            | "year"
            | "month"
            | "day"
            | "hour"
            | "minute"
            | "second"
            | "weekday"
            | "monthname"
            | "weekdayname"
    ) {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(interpreter.eval_expr(arg, frame)?);
        }
        return dispatch_datetime_function(effective_name, &values, span);
    }

    if matches!(
        effective_name.to_ascii_lowercase().as_str(),
        "kill" | "mkdir" | "rmdir" | "chdir"
    ) {
        expect_arg_count(effective_name, args, 1, span)?;
        let path = interpreter.eval_expr(&args[0], frame)?.to_output_string();
        match effective_name.to_ascii_lowercase().as_str() {
            "kill" => interpreter.kill_path(&path, span)?,
            "mkdir" => interpreter.mkdir_path(&path, span)?,
            "rmdir" => interpreter.rmdir_path(&path, span)?,
            "chdir" => interpreter.chdir_path(&path, span)?,
            _ => unreachable!(),
        }
        return Ok(Some(Value::Empty));
    }

    if !is_builtin_function(effective_name) {
        return Ok(None);
    }

    // Normal functions: evaluate all arguments first
    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        values.push(interpreter.eval_expr(arg, frame)?);
    }

    if effective_name.eq_ignore_ascii_case("IsMissing") {
        if values.len() != 1 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "IsMissing expects exactly 1 argument",
                Some(span),
            ));
        }
        return Ok(Some(Value::Boolean(matches!(values[0], Value::Missing))));
    }

    if let Some(val) = types::eval_types(effective_name, &values, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = arrays::eval_arrays(effective_name, &values, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = strings::eval_strings(interpreter, effective_name, &values, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = math::eval_math(interpreter, effective_name, &values, span)? {
        return Ok(Some(val));
    }

    Ok(None)
}

fn is_builtin_function(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "sgn"
            | "int"
            | "randomize"
            | "rnd"
            | "split"
            | "join"
            | "filter"
            | "cstr"
            | "strcomp"
            | "isobject"
            | "isarray"
            | "isnull"
            | "isempty"
            | "iserror"
            | "vartype"
            | "typename"
            | "createobject"
            | "cbyte"
            | "cint"
            | "clng"
            | "clnglng"
            | "cint64"
            | "csng"
            | "cdbl"
            | "cdec"
            | "ccur"
            | "cdate"
            | "cbool"
            | "array"
            | "lbound"
            | "ubound"
            | "len"
            | "lenb"
            | "left"
            | "right"
            | "mid"
            | "trim"
            | "ltrim"
            | "rtrim"
            | "ucase"
            | "lcase"
            | "replace"
            | "instr"
            | "instrrev"
            | "space"
            | "string"
            | "chr"
            | "chrw"
            | "asc"
            | "ascw"
            | "val"
            | "str"
            | "hex"
            | "oct"
            | "freefile"
            | "eof"
            | "lof"
            | "seek"
            | "dir"
            | "filelen"
            | "filedatetime"
            | "curdir"
            | "timer"
            | "now"
            | "date"
            | "time"
            | "dateserial"
            | "timeserial"
            | "datevalue"
            | "timevalue"
            | "year"
            | "month"
            | "day"
            | "hour"
            | "minute"
            | "second"
            | "weekday"
            | "monthname"
            | "weekdayname"
            | "kill"
            | "mkdir"
            | "rmdir"
            | "chdir"
            | "ismissing"
    )
}

fn dispatch_file_function(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    match name.to_ascii_lowercase().as_str() {
        "freefile" => {
            if !args.is_empty() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "FreeFile expects no arguments",
                    Some(span),
                ));
            }
            Ok(Some(Value::Int64(i64::from(
                interpreter.free_file_number(),
            ))))
        }
        "eof" => {
            expect_value_count(name, args, 1, span)?;
            let number = file_number_arg(name, &args[0], span)?;
            Ok(Some(Value::Boolean(interpreter.eof_file(number, span)?)))
        }
        "lof" => {
            expect_value_count(name, args, 1, span)?;
            let number = file_number_arg(name, &args[0], span)?;
            Ok(Some(Value::Int64(interpreter.lof_file(number, span)?)))
        }
        "seek" => {
            expect_value_count(name, args, 1, span)?;
            let number = file_number_arg(name, &args[0], span)?;
            Ok(Some(Value::Int64(
                interpreter.seek_file_position(number, span)?,
            )))
        }
        "dir" => Ok(Some(Value::String(interpreter.dir(args, span)?))),
        "filelen" => {
            expect_value_count(name, args, 1, span)?;
            let path = args[0].to_output_string();
            let len = std::fs::metadata(&path).map_err(|error| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!("Unable to get FileLen for '{}': {}", path, error),
                    Some(span),
                )
            })?;
            Ok(Some(Value::Int64(len.len() as i64)))
        }
        "filedatetime" => {
            expect_value_count(name, args, 1, span)?;
            let path = args[0].to_output_string();
            let modified = std::fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .map_err(|error| {
                    Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!("Unable to get FileDateTime for '{}': {}", path, error),
                        Some(span),
                    )
                })?;
            Ok(Some(Value::Date(system_time_to_vba_date(modified)?)))
        }
        "curdir" => {
            if args.len() > 1 {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "CurDir expects 0 to 1 arguments",
                    Some(span),
                ));
            }
            let cwd = std::env::current_dir().map_err(|error| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!("Unable to get current directory: {}", error),
                    Some(span),
                )
            })?;
            Ok(Some(Value::String(cwd.display().to_string())))
        }
        _ => Ok(None),
    }
}

fn dispatch_datetime_function(
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    match name.to_ascii_lowercase().as_str() {
        "timer" => {
            expect_value_count(name, args, 0, span)?;
            Ok(Some(Value::Double(timer_seconds()?)))
        }
        "now" => {
            expect_value_count(name, args, 0, span)?;
            Ok(Some(Value::Date(system_time_to_vba_date(
                std::time::SystemTime::now(),
            )?)))
        }
        "date" => {
            expect_value_count(name, args, 0, span)?;
            let now = system_time_to_vba_date(std::time::SystemTime::now())?;
            Ok(Some(Value::Date(now.floor())))
        }
        "time" => {
            expect_value_count(name, args, 0, span)?;
            let now = system_time_to_vba_date(std::time::SystemTime::now())?;
            Ok(Some(Value::Date(now.fract())))
        }
        "dateserial" => {
            expect_value_count(name, args, 3, span)?;
            let year = integer_arg(name, &args[0], span)?;
            let month = integer_arg(name, &args[1], span)?;
            let day = integer_arg(name, &args[2], span)?;
            Ok(Some(Value::Date(date_serial(year, month, day))))
        }
        "timeserial" => {
            expect_value_count(name, args, 3, span)?;
            let hour = integer_arg(name, &args[0], span)?;
            let minute = integer_arg(name, &args[1], span)?;
            let second = integer_arg(name, &args[2], span)?;
            Ok(Some(Value::Date(time_serial(hour, minute, second))))
        }
        "datevalue" => {
            expect_value_count(name, args, 1, span)?;
            Ok(Some(Value::Date(parse_date_value(
                &args[0].to_output_string(),
                span,
            )?)))
        }
        "timevalue" => {
            expect_value_count(name, args, 1, span)?;
            Ok(Some(Value::Date(parse_time_value(
                &args[0].to_output_string(),
                span,
            )?)))
        }
        "year" | "month" | "day" => {
            expect_value_count(name, args, 1, span)?;
            let serial = date_arg(name, &args[0], span)?;
            let (year, month, day) = civil_from_days(serial.floor() as i64 - UNIX_EPOCH_AS_VBA);
            let value = match name.to_ascii_lowercase().as_str() {
                "year" => year,
                "month" => i64::from(month),
                "day" => i64::from(day),
                _ => unreachable!(),
            };
            Ok(Some(Value::Int64(value)))
        }
        "hour" | "minute" | "second" => {
            expect_value_count(name, args, 1, span)?;
            let serial = date_arg(name, &args[0], span)?;
            let total = seconds_since_midnight(serial);
            let value = match name.to_ascii_lowercase().as_str() {
                "hour" => total / 3600,
                "minute" => (total % 3600) / 60,
                "second" => total % 60,
                _ => unreachable!(),
            };
            Ok(Some(Value::Int64(value)))
        }
        "weekday" => {
            if args.is_empty() || args.len() > 2 {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Weekday expects 1 to 2 arguments",
                    Some(span),
                ));
            }
            let serial = date_arg(name, &args[0], span)?;
            let first_day = args
                .get(1)
                .and_then(crate::runtime::numeric::value_to_i64)
                .unwrap_or(1);
            Ok(Some(Value::Int64(weekday_value(serial, first_day))))
        }
        "monthname" => {
            if args.is_empty() || args.len() > 2 {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "MonthName expects 1 to 2 arguments",
                    Some(span),
                ));
            }
            let month = integer_arg(name, &args[0], span)?;
            let abbreviate = args.get(1).is_some_and(Value::is_truthy);
            Ok(Some(Value::String(month_name(month, abbreviate, span)?)))
        }
        "weekdayname" => {
            if args.is_empty() || args.len() > 3 {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "WeekdayName expects 1 to 3 arguments",
                    Some(span),
                ));
            }
            let weekday = integer_arg(name, &args[0], span)?;
            let abbreviate = args.get(1).is_some_and(Value::is_truthy);
            Ok(Some(Value::String(weekday_name(
                weekday, abbreviate, span,
            )?)))
        }
        _ => Ok(None),
    }
}

fn expect_value_count(
    name: &str,
    args: &[Value],
    expected: usize,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            format!("{name} expects exactly {expected} argument(s)"),
            Some(span),
        ))
    }
}

fn file_number_arg(
    name: &str,
    value: &Value,
    span: crate::runtime::Span,
) -> Result<i32, Diagnostic> {
    let number = crate::runtime::numeric::value_to_i64(value).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!("{name} file number must be Integer"),
            Some(span),
        )
    })?;
    if !(1..=511).contains(&number) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "File number must be between 1 and 511",
            Some(span),
        ));
    }
    Ok(number as i32)
}

const UNIX_EPOCH_AS_VBA: i64 = 25_569;

fn system_time_to_vba_date(time: std::time::SystemTime) -> Result<f64, Diagnostic> {
    let duration = time
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                format!("Date before Unix epoch is not supported: {}", error),
                None,
            )
        })?;
    Ok(UNIX_EPOCH_AS_VBA as f64 + duration.as_secs_f64() / 86_400.0)
}

fn timer_seconds() -> Result<f64, Diagnostic> {
    let now = system_time_to_vba_date(std::time::SystemTime::now())?;
    Ok((now.fract() * 86_400.0).rem_euclid(86_400.0))
}

fn integer_arg(name: &str, value: &Value, span: crate::runtime::Span) -> Result<i64, Diagnostic> {
    crate::runtime::numeric::value_to_i64(value).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!("{name} argument must be Integer"),
            Some(span),
        )
    })
}

fn date_arg(name: &str, value: &Value, span: crate::runtime::Span) -> Result<f64, Diagnostic> {
    match value {
        Value::Date(value) | Value::Double(value) => Ok(*value),
        Value::Single(value) => Ok(f64::from(*value)),
        Value::Int16(value) => Ok(f64::from(*value)),
        Value::Int32(value) => Ok(f64::from(*value)),
        Value::Int64(value) => Ok(*value as f64),
        _ => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!("{name} argument must be Date"),
            Some(span),
        )),
    }
}

fn date_serial(year: i64, month: i64, day: i64) -> f64 {
    let month_index = month - 1;
    let normalized_year = year + month_index.div_euclid(12);
    let normalized_month = month_index.rem_euclid(12) + 1;
    let days = days_from_civil(normalized_year, normalized_month as u32, 1) + day - 1;
    (days + UNIX_EPOCH_AS_VBA) as f64
}

fn time_serial(hour: i64, minute: i64, second: i64) -> f64 {
    let total = hour * 3600 + minute * 60 + second;
    total as f64 / 86_400.0
}

fn parse_date_value(value: &str, span: crate::runtime::Span) -> Result<f64, Diagnostic> {
    let parts: Vec<_> = value
        .split(['-', '/'])
        .filter_map(|part| part.trim().parse::<i64>().ok())
        .collect();
    if parts.len() == 3 {
        let (year, month, day) = if parts[0] > 31 {
            (parts[0], parts[1], parts[2])
        } else {
            (parts[2], parts[0], parts[1])
        };
        Ok(date_serial(year, month, day))
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "DateValue expects a date like yyyy-mm-dd or mm/dd/yyyy",
            Some(span),
        ))
    }
}

fn parse_time_value(value: &str, span: crate::runtime::Span) -> Result<f64, Diagnostic> {
    let parts: Vec<_> = value
        .split(':')
        .map(str::trim)
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "TimeValue expects a time like hh:mm:ss",
                Some(span),
            )
        })?;
    if !(2..=3).contains(&parts.len()) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "TimeValue expects a time like hh:mm:ss",
            Some(span),
        ));
    }
    Ok(time_serial(
        parts[0],
        parts[1],
        parts.get(2).copied().unwrap_or(0),
    ))
}

fn seconds_since_midnight(serial: f64) -> i64 {
    ((serial.fract().rem_euclid(1.0) * 86_400.0).round() as i64).rem_euclid(86_400)
}

fn weekday_value(serial: f64, first_day: i64) -> i64 {
    let days_since_unix = serial.floor() as i64 - UNIX_EPOCH_AS_VBA;
    let sunday_based = (days_since_unix + 4).rem_euclid(7) + 1;
    let first_day = if (1..=7).contains(&first_day) {
        first_day
    } else {
        1
    };
    (sunday_based - first_day).rem_euclid(7) + 1
}

fn month_name(
    month: i64,
    abbreviate: bool,
    span: crate::runtime::Span,
) -> Result<String, Diagnostic> {
    const FULL: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const SHORT: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    if !(1..=12).contains(&month) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "MonthName month must be between 1 and 12",
            Some(span),
        ));
    }
    let names = if abbreviate { SHORT } else { FULL };
    Ok(names[month as usize - 1].to_string())
}

fn weekday_name(
    weekday: i64,
    abbreviate: bool,
    span: crate::runtime::Span,
) -> Result<String, Diagnostic> {
    const FULL: [&str; 7] = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    const SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    if !(1..=7).contains(&weekday) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "WeekdayName weekday must be between 1 and 7",
            Some(span),
        ));
    }
    let names = if abbreviate { SHORT } else { FULL };
    Ok(names[weekday as usize - 1].to_string())
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let month = month as i64;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096).div_euclid(365);
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2).div_euclid(153);
    let day = doy - (153 * mp + 2).div_euclid(5) + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month as u32, day as u32)
}

fn dispatch_callbyname(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("CallByName") {
        if args.len() < 3 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "CallByName expects at least 3 arguments",
                Some(span),
            ));
        }
        let obj = interpreter.eval_expr(&args[0], frame)?;
        let member = interpreter.eval_expr(&args[1], frame)?.to_output_string();
        let call_type =
            interpreter.eval_integer_expr(&args[2], frame, "Call type must be Integer")?;

        let remaining_args = &args[3..];

        match call_type {
            1 => {
                // VbMethod
                // Try Function first to get a return value
                if let Ok(val) = interpreter.call_method_function(
                    obj.clone(),
                    &member,
                    remaining_args,
                    frame,
                    span,
                ) {
                    return Ok(Some(val));
                }

                // If function fails, try Sub
                interpreter.call_method_sub(obj, &member, remaining_args, frame, span)?;
                return Ok(Some(Value::Empty));
            }
            2 => {
                // VbGet
                return Ok(Some(interpreter.read_member(&obj, &member, frame, span)?));
            }
            4 | 8 => {
                // VbLet (4) or VbSet (8)
                if remaining_args.len() != 1 {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "CallByName for Let/Set expects exactly one value argument",
                        Some(span),
                    ));
                }
                let value = interpreter.eval_expr(&remaining_args[0], frame)?;
                interpreter.assign_member_to_value(obj, &member, value, span)?;
                return Ok(Some(Value::Empty));
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!("Invalid CallByName call type: {}", call_type),
                    Some(span),
                ));
            }
        }
    }
    Ok(None)
}

pub(crate) mod arrays;
pub(crate) mod console;
pub(crate) mod debug;
pub(crate) mod err;
pub(crate) mod math;
pub(crate) mod strings;
pub(crate) mod types;

pub(crate) fn expect_arg_count(
    name: &str,
    args: &[Expr],
    expected: usize,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            format!("{name} expects exactly {expected} argument(s)"),
            Some(span),
        ))
    }
}
