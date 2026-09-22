//! Valo Builtins
//!
//! Standard library functions and procedures.
//!
//! Value-level builtins are kept backend-neutral where practical. A small
//! dispatch layer still handles lazy/special forms that need AST or frame access
//! (`IIf`, `CallByName`, pointer builtins, and host-owned services).

use super::{ControlFlow, Frame, Interpreter};
use crate::runtime::builtins::{
    BuiltinGroup, is_builtin_function, is_in_group, strip_vba_namespace,
};
use crate::runtime::well_known;
use crate::runtime::{Diagnostic, Value};
use crate::{Expr, ExprKind};
use std::rc::Rc;

const DEFAULT_DIALOG_TITLE: &str = "Valo";

pub(crate) fn dispatch_stmt(
    interpreter: &mut Interpreter,
    object_name: &str,
    method: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<ControlFlow>, Diagnostic> {
    if object_name.is_empty() && is_in_group(method, BuiltinGroup::Dialog) {
        dispatch_function(interpreter, method, args, frame, span)?;
        return Ok(Some(ControlFlow::Continue));
    }
    // Handle VBA namespace fallback: VBA.MsgBox(...) -> MsgBox(...)
    let effective_object_name = if object_name.eq_ignore_ascii_case(well_known::VBA) {
        "VBA"
    } else {
        object_name
    };

    if effective_object_name.eq_ignore_ascii_case(well_known::CONSOLE)
        || effective_object_name.eq_ignore_ascii_case(well_known::DEBUG)
    {
        if effective_object_name.eq_ignore_ascii_case(well_known::DEBUG)
            && method.eq_ignore_ascii_case("Assert")
        {
            dispatch_function(interpreter, "Debug.Assert", args, frame, span)?;
            return Ok(Some(ControlFlow::Continue));
        }

        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            let val = interpreter.eval_expr(arg, frame)?;
            let resolved = interpreter.resolve_default_value(val, frame, arg.span)?;
            values.push(resolved);
        }

        if effective_object_name.eq_ignore_ascii_case(well_known::CONSOLE) {
            if method.eq_ignore_ascii_case("ReadLine") {
                let result = console::exec_console(method, &values, span)?;
                if let Some(_line) = result {
                    // This is a bit tricky: ReadLine as a statement? Usually it's a function.
                    // If called as a statement, we just ignore the result.
                    return Ok(Some(ControlFlow::Continue));
                }
            }
            if let Some(line) = console::exec_console(method, &values, span)? {
                interpreter.emit_output(line);
                return Ok(Some(ControlFlow::Continue));
            } else if method.eq_ignore_ascii_case("Write") {
                return Ok(Some(ControlFlow::Continue));
            }
        } else {
            if let Some(line) = debug::exec_debug(method, &values, span)? {
                interpreter.emit_output(line);
                return Ok(Some(ControlFlow::Continue));
            }
        }
    }

    if effective_object_name.eq_ignore_ascii_case(well_known::ERR) {
        return err::exec_err(interpreter, method, args, frame, span);
    }

    if effective_object_name.eq_ignore_ascii_case(well_known::VBA) {
        if is_in_group(method, BuiltinGroup::Dialog) {
            dispatch_function(interpreter, method, args, frame, span)?;
            return Ok(Some(ControlFlow::Continue));
        }

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

/// A builtin implemented over already-evaluated arguments.
///
/// The dispatcher checks the argument count against the registry before calling
/// one of these, so an implementation may index the arguments its own registry
/// entry guarantees without re-checking.
/// The name is passed through so one implementation can back several
/// registered names, as the financial functions and the numeric conversions do.
pub(super) type ValueFn =
    fn(&mut Interpreter, &str, &[Value], crate::runtime::Span) -> Result<Value, Diagnostic>;

/// Finds the implementation a module provides for `name`.
pub(super) fn find_handler(handlers: &[(&str, ValueFn)], name: &str) -> Option<ValueFn> {
    // The registry has already said whether the name is a builtin at all, so
    // this scan only runs for names that are one, over a table of a few dozen.
    handlers
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, handler)| *handler)
}

#[cfg(test)]
mod handler_tests {
    use super::*;

    /// Every module that dispatches through a handler table.
    ///
    /// As modules are converted from name-comparison chains to tables they are
    /// added here, and the checks below start covering them.
    const TABLES: &[(&str, &[(&str, ValueFn)])] = &[
        ("types", types::HANDLERS),
        ("math", math::HANDLERS),
        ("strings", strings::HANDLERS),
        ("arrays", arrays::HANDLERS),
        ("misc", MISC_HANDLERS),
        ("file system", FILE_SYSTEM_HANDLERS),
        ("files", FILE_HANDLERS),
        ("date and time", DATETIME_HANDLERS),
    ];

    /// The lazy table and the value tables must not claim the same builtin.
    ///
    /// A name in both would be answered by the lazy one, since it is tried
    /// first, leaving the value implementation silently unreachable.
    #[test]
    fn no_builtin_is_both_lazy_and_value_dispatched() {
        for (lazy_name, _) in EXPR_HANDLERS {
            for (module, handlers) in TABLES {
                assert!(
                    find_handler(handlers, lazy_name).is_none(),
                    "'{lazy_name}' is dispatched before evaluation and also implemented by {module}"
                );
            }
        }
    }

    /// Every builtin the registry declares must have an implementation.
    ///
    /// This is the half of the registry's promise that only became checkable
    /// once dispatch was table-driven: before, an implementation could only be
    /// found by executing the chain of name comparisons.
    #[test]
    fn every_declared_builtin_has_an_implementation() {
        let missing: Vec<&str> = crate::runtime::builtins::BUILTINS
            .iter()
            .filter(|builtin| {
                find_expr_handler(builtin.name).is_none()
                    && TABLES
                        .iter()
                        .all(|(_, handlers)| find_handler(handlers, builtin.name).is_none())
                    && !UNCONVERTED
                        .iter()
                        .any(|name| name.eq_ignore_ascii_case(builtin.name))
            })
            .map(|builtin| builtin.name)
            .collect();

        assert!(
            missing.is_empty(),
            "the registry declares builtins with no implementation and no exemption: {missing:?}"
        );
    }

    /// A group routed to a table must list exactly the same builtins.
    ///
    /// Dispatch selects a table by group, so a disagreement would silently send
    /// a builtin somewhere with no handler for it.
    #[test]
    fn every_routed_group_matches_its_table() {
        use crate::runtime::builtins::{BUILTINS, BuiltinGroup};

        /// A dispatch group paired with the table it routes to.
        type RoutedGroup = (
            BuiltinGroup,
            &'static str,
            &'static [(&'static str, ValueFn)],
        );

        let routed: &[RoutedGroup] = &[
            (BuiltinGroup::FileSystem, "FileSystem", FILE_SYSTEM_HANDLERS),
            (BuiltinGroup::File, "File", FILE_HANDLERS),
            (BuiltinGroup::DateTime, "DateTime", DATETIME_HANDLERS),
        ];

        for (group, label, handlers) in routed {
            for builtin in BUILTINS {
                let in_group = builtin.group == *group;
                let in_table = find_handler(handlers, builtin.name).is_some();
                assert_eq!(
                    in_group, in_table,
                    "'{}' is in the {label} group ({in_group}) but its handler table says {in_table}",
                    builtin.name
                );
            }
        }
    }

    /// An exemption must name a builtin that really has no table entry.
    ///
    /// Without this, converting a module would leave stale exemptions behind,
    /// and the list would stop reflecting what is actually left to do.
    #[test]
    fn no_exemption_is_stale() {
        for name in UNCONVERTED {
            let implemented = find_expr_handler(name).is_some()
                || TABLES
                    .iter()
                    .any(|(_, handlers)| find_handler(handlers, name).is_some());
            assert!(
                !implemented,
                "'{name}' is implemented by a handler table and should be removed from UNCONVERTED"
            );
        }
    }

    /// Builtins still reached through the remaining name-comparison chains.
    ///
    /// Each entry is a module not yet converted to a handler table. The list
    /// only shrinks: adding a name to it hides a builtin from the check above,
    /// so a new builtin belongs in a table instead.
    const UNCONVERTED: &[&str] = &[];

    /// A handler table may only name builtins the registry declares.
    ///
    /// Implementing something the registry has never heard of means the
    /// analyzer will reject every call to it, so the implementation is dead
    /// code that looks alive.
    #[test]
    fn every_handler_is_a_declared_builtin() {
        for (module, handlers) in TABLES {
            for (name, _) in *handlers {
                assert!(
                    crate::runtime::builtins::lookup(name).is_some(),
                    "{module} implements '{name}', which is not in the builtin registry"
                );
            }
        }
    }

    /// A name may not be registered twice within one table.
    #[test]
    fn no_handler_table_registers_a_name_twice() {
        for (module, handlers) in TABLES {
            let mut seen = std::collections::HashSet::new();
            for (name, _) in *handlers {
                assert!(
                    seen.insert(name.to_lowercase()),
                    "{module} registers '{name}' more than once"
                );
            }
        }
    }

    /// Two modules must not claim the same builtin.
    ///
    /// Dispatch tries the tables in order, so a name in two of them would be
    /// answered by whichever happens to be tried first, which is a silent
    /// choice between two implementations.
    #[test]
    fn no_builtin_is_implemented_by_two_modules() {
        let mut owner = std::collections::HashMap::new();
        for (module, handlers) in TABLES {
            for (name, _) in *handlers {
                if let Some(previous) = owner.insert(name.to_lowercase(), *module)
                    && previous != *module
                {
                    panic!("'{name}' is implemented by both {previous} and {module}");
                }
            }
        }
    }

    /// A converted module must implement every builtin routed to it.
    ///
    /// `types` owns the type-inspection and conversion builtins; if the
    /// registry gains one and the table does not, calls would fall through to
    /// the remaining name-comparison chains and be reported as undefined.
    #[test]
    fn the_types_module_implements_every_conversion_builtin() {
        for builtin in crate::runtime::builtins::BUILTINS {
            let looks_like_a_conversion = builtin.name.starts_with('C')
                && builtin.name.len() > 1
                && builtin.name.as_bytes()[1].is_ascii_uppercase();
            let looks_like_an_inspection = builtin.name.starts_with("Is");
            if !looks_like_a_conversion && !looks_like_an_inspection {
                continue;
            }
            // These are checked by the analyzer against the call site rather
            // than evaluated from a value, so they live elsewhere.
            if matches!(builtin.name, "IsMissing" | "CallByName" | "CStr") {
                continue;
            }
            assert!(
                find_handler(types::HANDLERS, builtin.name).is_some(),
                "the registry declares '{}' but types has no implementation for it",
                builtin.name
            );
        }
    }
}

/// A builtin that needs its arguments before they are evaluated.
///
/// Lazy forms such as `IIf` must not evaluate the branch they do not take, and
/// the pointer builtins need the storage an argument names rather than its
/// value. Everything else is reached through a module's value table instead.
pub(super) type ExprFn = fn(
    &mut Interpreter,
    &str,
    &[Expr],
    &mut Frame,
    crate::runtime::Span,
) -> Result<Value, Diagnostic>;

/// Builtins dispatched before their arguments are evaluated.
pub(super) const EXPR_HANDLERS: &[(&str, ExprFn)] = &[
    ("IIf", i_if),
    ("Choose", choose),
    ("Switch", switch),
    ("CallByName", call_by_name),
    ("VarPtr", var_ptr),
    ("StrPtr", str_ptr),
    ("ObjPtr", obj_ptr),
    ("MsgBox", msg_box),
    ("InputBox", input_box),
    ("Command", command),
    ("Error", error),
    ("Input", input),
    ("Shell", shell),
    ("Environ", environ),
    ("GetSetting", get_setting),
    ("GetAllSettings", get_all_settings),
    ("IMEStatus", i_m_e_status),
    ("MacID", mac_i_d),
    ("MacScript", mac_script),
    ("IsMissing", is_missing),
];

fn find_expr_handler(name: &str) -> Option<ExprFn> {
    EXPR_HANDLERS
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, handler)| *handler)
}

fn i_if(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    _: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    let condition = interpreter.eval_expr(&args[0], frame)?.is_truthy();
    let value_expr = if condition { &args[1] } else { &args[2] };
    interpreter.eval_expr(value_expr, frame)
}

fn choose(
    interpreter: &mut Interpreter,
    effective_name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.len() < 2 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "Choose expects an index and at least one choice",
            Some(span),
        ));
    }
    let index = integer_arg(
        effective_name,
        &interpreter.eval_expr(&args[0], frame)?,
        span,
    )?;
    if index < 1 || index as usize >= args.len() {
        return Ok(Value::Null);
    }
    interpreter.eval_expr(&args[index as usize], frame)
}

fn switch(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.is_empty() || !args.len().is_multiple_of(2) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "Switch expects expression/value pairs",
            Some(span),
        ));
    }
    for pair in args.chunks(2) {
        if interpreter.eval_expr(&pair[0], frame)?.is_truthy() {
            return interpreter.eval_expr(&pair[1], frame);
        }
    }
    Ok(Value::Null)
}

fn call_by_name(
    interpreter: &mut Interpreter,
    effective_name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    dispatch_callbyname(interpreter, effective_name, args, frame, span)?.ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            "CallByName could not resolve the member",
            Some(span),
        )
    })
}

fn var_ptr(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    _: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    Ok(Value::Ptr(interpreter.varptr_expr(&args[0], frame)?))
}

fn str_ptr(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    _: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    let arg = &args[0];
    let value = interpreter.eval_expr(arg, frame)?;
    let text = match value {
        Value::String(text) => text,
        Value::Empty => Rc::new(String::new()),
        Value::Null | Value::Nothing | Value::Missing => return Ok(Value::Ptr(0)),
        other => other.to_output_string().into(),
    };
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    interpreter.temporary_wide_strings.push(wide);
    let ptr = interpreter
        .temporary_wide_strings
        .last()
        .map(|text| text.as_ptr() as usize)
        .unwrap_or(0);
    Ok(Value::Ptr(ptr))
}

fn obj_ptr(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    let value = interpreter.eval_expr(&args[0], frame)?;
    match value {
        Value::Object(obj) => {
            let ptr = std::rc::Rc::as_ptr(&obj) as usize;
            Ok(Value::Ptr(ptr))
        }
        Value::Collection(coll) => {
            let ptr = std::rc::Rc::as_ptr(&coll) as usize;
            Ok(Value::Ptr(ptr))
        }

        Value::Nothing => Ok(Value::Ptr(0)),
        _ => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "ObjPtr requires an object",
            Some(span),
        )),
    }
}

fn msg_box(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.is_empty() || args.len() > 5 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "MsgBox expects 1 to 5 arguments",
            Some(span),
        ));
    }
    let prompt = interpreter.eval_expr(&args[0], frame)?.to_output_string();
    let title = if args.len() >= 3 && !matches!(args[2].kind, ExprKind::Missing) {
        interpreter.eval_expr(&args[2], frame)?.to_output_string()
    } else {
        DEFAULT_DIALOG_TITLE.to_string()
    };

    #[cfg(windows)]
    {
        use crate::runtime::{TypeName, coerce_assignment};
        let buttons_val = if args.len() >= 2 && !matches!(args[1].kind, ExprKind::Missing) {
            interpreter.eval_expr(&args[1], frame)?
        } else {
            Value::Int32(0) // vbOKOnly
        };
        let buttons = coerce_assignment(&TypeName::Int32, buttons_val, span)?
            .to_output_string()
            .parse::<i32>()
            .unwrap_or(0);

        use windows::Win32::UI::WindowsAndMessaging::{MESSAGEBOX_STYLE, MessageBoxW};
        use windows::core::HSTRING;
        let result = unsafe {
            MessageBoxW(
                None,
                &HSTRING::from(prompt),
                &HSTRING::from(title),
                MESSAGEBOX_STYLE(buttons as u32),
            )
        };
        Ok(Value::Int16(result.0 as i16))
    }
    #[cfg(not(windows))]
    {
        // Evaluate buttons anyway to maintain side-effects parity
        if args.len() >= 2 && !matches!(args[1].kind, ExprKind::Missing) {
            let _ = interpreter.eval_expr(&args[1], frame)?;
        }

        println!("{title}: {prompt}");
        // Nothing to click, so report the same answer an OK press would.
        Ok(Value::Int16(1))
    }
}

fn input_box(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.is_empty() || args.len() > 7 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "InputBox expects 1 to 7 arguments",
            Some(span),
        ));
    }
    let prompt = interpreter.eval_expr(&args[0], frame)?.to_output_string();
    let title = if args.len() >= 2 && !matches!(args[1].kind, ExprKind::Missing) {
        interpreter.eval_expr(&args[1], frame)?.to_output_string()
    } else {
        DEFAULT_DIALOG_TITLE.to_string()
    };
    let default = if args.len() >= 3 && !matches!(args[2].kind, ExprKind::Missing) {
        interpreter.eval_expr(&args[2], frame)?.to_output_string()
    } else {
        String::new()
    };

    // For now, let's use console ReadLine as a fallback for InputBox
    println!("{title}: {prompt}");
    if !default.is_empty() {
        println!("Default: {default}");
    }
    print!("> ");
    use std::io::{self, Write};
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    let result = input.trim_end_matches(['\r', '\n']).to_string();
    if result.is_empty() && !default.is_empty() {
        return Ok(Value::String(Rc::new(default)));
    }
    Ok(Value::String(Rc::new(result)))
}

fn command(
    _: &mut Interpreter,
    _: &str,
    _: &[Expr],
    _: &mut Frame,
    _: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    let command = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    Ok(Value::String(Rc::new(command)))
}

fn error(
    interpreter: &mut Interpreter,
    effective_name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.len() > 1 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "Error expects 0 to 1 arguments",
            Some(span),
        ));
    }
    let number = if let Some(arg) = args.first() {
        integer_arg(effective_name, &interpreter.eval_expr(arg, frame)?, span)?
    } else {
        0
    };
    Ok(Value::String(Rc::new(error_description(number))))
}

fn input(
    interpreter: &mut Interpreter,
    effective_name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    let count = integer_arg(
        effective_name,
        &interpreter.eval_expr(&args[0], frame)?,
        span,
    )?;
    let number = file_number_arg(
        effective_name,
        &interpreter.eval_expr(&args[1], frame)?,
        span,
    )?;
    Ok(Value::String(Rc::from(
        interpreter.input_file_chars(number, count, span)?,
    )))
}

fn shell(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.is_empty() || args.len() > 2 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "Shell expects 1 to 2 arguments",
            Some(span),
        ));
    }
    let command = interpreter.eval_expr(&args[0], frame)?.to_output_string();
    let child = if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", &command])
            .spawn()
    } else {
        std::process::Command::new("sh")
            .args(["-c", &command])
            .spawn()
    }
    .map_err(|error| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::RUNTIME,
            format!("Shell failed to start '{}': {}", command, error),
            Some(span),
        )
    })?;
    Ok(Value::Int64(i64::from(child.id())))
}

fn environ(
    interpreter: &mut Interpreter,
    effective_name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    let value = interpreter.eval_expr(&args[0], frame)?;
    Ok(Value::String(Rc::new(environ_value(
        effective_name,
        &value,
        span,
    )?)))
}

fn get_setting(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.len() < 3 || args.len() > 4 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "GetSetting expects 3 to 4 arguments",
            Some(span),
        ));
    }
    for arg in args.iter().take(3) {
        let _ = interpreter.eval_expr(arg, frame)?;
    }
    let default = if let Some(default) = args.get(3) {
        interpreter.eval_expr(default, frame)?.to_output_string()
    } else {
        String::new()
    };
    Ok(Value::String(Rc::new(default)))
}

fn get_all_settings(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    _: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    for arg in args {
        let _ = interpreter.eval_expr(arg, frame)?;
    }
    Ok(empty_string_matrix())
}

fn i_m_e_status(
    _: &mut Interpreter,
    _: &str,
    _: &[Expr],
    _: &mut Frame,
    _: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    Ok(Value::Int64(0))
}

fn mac_i_d(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    _: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    let text = interpreter.eval_expr(&args[0], frame)?.to_output_string();
    let mut bytes = [b' '; 4];
    for (slot, byte) in bytes.iter_mut().zip(text.bytes()) {
        *slot = byte;
    }
    let value = bytes
        .iter()
        .fold(0_i64, |acc, byte| (acc << 8) | i64::from(*byte));
    Ok(Value::Int64(value))
}

fn mac_script(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    let _ = interpreter.eval_expr(&args[0], frame)?;
    Err(Diagnostic::new(
        crate::runtime::DiagnosticCode::UNSUPPORTED,
        "Compatibility diagnostic: MacScript requires an Office VBA Mac host and is not supported by the standalone Valo runtime; replace MacScript with a platform API, shell command, or host-specific adapter",
        Some(span),
    )
    .with_help("replace MacScript with a platform API, shell command, or host-specific adapter"))
}

fn is_missing(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Expr],
    frame: &mut Frame,
    _: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    // Reports whether an optional parameter was left out at the call site.
    let value = interpreter.eval_expr(&args[0], frame)?;
    Ok(Value::Boolean(matches!(value, Value::Missing)))
}
pub(crate) fn dispatch_function(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    // Handle VBA namespace fallback: VBA.Join(...) -> Join(...)
    let effective_name = strip_vba_namespace(name);

    // Check the argument count once, against the registry, before anything is
    // evaluated. Every implementation below can then rely on having a count its
    // own entry allows, instead of restating the check.
    // A name that is not in the registry is not a builtin, and everything below
    // is a search for one. Saying so here is what keeps a call to a user
    // procedure from walking the dispatch tables.
    let Some(builtin) = crate::runtime::builtins::lookup(effective_name) else {
        return Ok(None);
    };
    builtin.check_arity(args.len(), span)?;

    // Builtins that need their arguments unevaluated, or that need the storage
    // an argument names rather than its value.
    if let Some(handler) = find_expr_handler(effective_name) {
        return handler(interpreter, effective_name, args, frame, span).map(Some);
    }

    // Special forms that require lazy evaluation or direct Expr access

    if effective_name.eq_ignore_ascii_case("Debug.Assert") {
        expect_arg_count(effective_name, args, 1, span)?;
        let condition = interpreter.eval_expr(&args[0], frame)?.is_truthy();
        if !condition {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::RUNTIME_ERROR,
                "Assertion failed".to_string(),
                Some(args[0].span),
            ));
        }
        return Ok(Some(Value::Empty));
    }

    if effective_name.eq_ignore_ascii_case("Console.ReadLine") {
        let result = console::exec_console("ReadLine", &[], span)?;
        return Ok(Some(Value::String(Rc::new(result.unwrap_or_default()))));
    }

    if is_in_group(effective_name, BuiltinGroup::File) {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(interpreter.eval_expr(arg, frame)?);
        }
        return dispatch_file_function(interpreter, effective_name, &values, span);
    }

    if is_in_group(effective_name, BuiltinGroup::DateTime) {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(interpreter.eval_expr(arg, frame)?);
        }
        return dispatch_datetime_function(interpreter, effective_name, &values, span);
    }

    if is_in_group(effective_name, BuiltinGroup::FileSystem) {
        let path = interpreter.eval_expr(&args[0], frame)?.to_output_string();
        let act = find_handler(FILE_SYSTEM_HANDLERS, effective_name)
            .expect("the FileSystem group and its handler table list the same builtins");
        act(
            interpreter,
            effective_name,
            &[Value::String(Rc::new(path))],
            span,
        )?;
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

    if let Some(val) = types::eval_types(interpreter, effective_name, &values, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = arrays::eval_arrays(interpreter, effective_name, &values, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = strings::eval_strings(interpreter, effective_name, &values, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = math::eval_math(interpreter, effective_name, &values, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = eval_misc_value_function(interpreter, effective_name, &values, span)? {
        return Ok(Some(val));
    }

    Ok(None)
}

/// Builtins that read from or ask about open files and the file system.
pub(super) const FILE_HANDLERS: &[(&str, ValueFn)] = &[
    ("FreeFile", freefile),
    ("EOF", eof),
    ("FileAttr", fileattr),
    ("LOF", lof),
    ("Loc", loc_builtin),
    ("Seek", seek_builtin),
    ("GetAttr", getattr),
    ("FileLen", filelen),
    ("FileDateTime", filedatetime),
    ("CurDir", curdir),
    ("Dir", dir_builtin),
];

fn dispatch_file_function(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    match find_handler(FILE_HANDLERS, name) {
        Some(handler) => handler(interpreter, name, args, span).map(Some),
        None => Ok(None),
    }
}

fn freefile(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if !args.is_empty() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "FreeFile expects no arguments",
            Some(span),
        ));
    }
    Ok(Value::Int64(i64::from(interpreter.free_file_number())))
}

fn eof(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 1, span)?;
    let number = file_number_arg(name, &args[0], span)?;
    Ok(Value::Boolean(interpreter.eof_file(number, span)?))
}

fn fileattr(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 2, span)?;
    let number = file_number_arg(name, &args[0], span)?;
    let attribute = integer_arg(name, &args[1], span)?;
    Ok(Value::Int64(
        interpreter.file_attr(number, attribute, span)?,
    ))
}

fn lof(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 1, span)?;
    let number = file_number_arg(name, &args[0], span)?;
    Ok(Value::Int64(interpreter.lof_file(number, span)?))
}

fn loc_builtin(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 1, span)?;
    let number = file_number_arg(name, &args[0], span)?;
    Ok(Value::Int64(interpreter.loc_file(number, span)?))
}

fn seek_builtin(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 1, span)?;
    let number = file_number_arg(name, &args[0], span)?;
    Ok(Value::Int64(interpreter.seek_file_position(number, span)?))
}

fn getattr(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 1, span)?;
    Ok(Value::Int64(get_attr(&args[0].to_output_string(), span)?))
}

fn filelen(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 1, span)?;
    let path = args[0].to_output_string();
    let len = std::fs::metadata(&path).map_err(|error| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::FILE_IO,
            format!("Unable to get FileLen for '{}': {}", path, error),
            Some(span),
        )
    })?;
    Ok(Value::Int64(len.len() as i64))
}

fn filedatetime(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 1, span)?;
    let path = args[0].to_output_string();
    let modified = std::fs::metadata(&path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::FILE_IO,
                format!("Unable to get FileDateTime for '{}': {}", path, error),
                Some(span),
            )
        })?;
    Ok(Value::Date(system_time_to_vba_date(modified)?))
}

fn curdir(
    _: &mut Interpreter,
    _: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.len() > 1 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "CurDir expects 0 to 1 arguments",
            Some(span),
        ));
    }
    let cwd = std::env::current_dir().map_err(|error| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::FILE_IO,
            format!("Unable to get current directory: {}", error),
            Some(span),
        )
    })?;
    Ok(Value::String(Rc::new(cwd.display().to_string())))
}

fn dir_builtin(
    interpreter: &mut Interpreter,
    _: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    Ok(Value::String(Rc::new(interpreter.dir(args, span)?)))
}

/// Builtins over dates and times.
pub(super) const DATETIME_HANDLERS: &[(&str, ValueFn)] = &[
    ("Timer", timer),
    ("Now", now),
    ("Date", date_builtin),
    ("Time", time_builtin),
    ("DateSerial", dateserial),
    ("TimeSerial", timeserial),
    ("DateValue", datevalue),
    ("TimeValue", timevalue),
    ("DateAdd", dateadd),
    ("DateDiff", datediff),
    ("DatePart", datepart),
    ("Year", year),
    ("Month", year),
    ("Day", year),
    ("Hour", hour),
    ("Minute", hour),
    ("Second", hour),
    ("Weekday", weekday),
    ("MonthName", monthname),
    ("WeekdayName", weekdayname),
];

fn dispatch_datetime_function(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    match find_handler(DATETIME_HANDLERS, name) {
        Some(handler) => handler(interpreter, name, args, span).map(Some),
        None => Ok(None),
    }
}

fn timer(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 0, span)?;
    Ok(Value::Double(timer_seconds()?))
}

fn now(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 0, span)?;
    Ok(Value::Date(system_time_to_vba_date(
        std::time::SystemTime::now(),
    )?))
}

fn date_builtin(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 0, span)?;
    let now = system_time_to_vba_date(std::time::SystemTime::now())?;
    Ok(Value::Date(now.floor()))
}

fn time_builtin(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 0, span)?;
    let now = system_time_to_vba_date(std::time::SystemTime::now())?;
    Ok(Value::Date(now.fract()))
}

fn dateserial(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 3, span)?;
    let year = integer_arg(name, &args[0], span)?;
    let month = integer_arg(name, &args[1], span)?;
    let day = integer_arg(name, &args[2], span)?;
    Ok(Value::Date(date_serial(year, month, day)))
}

fn timeserial(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 3, span)?;
    let hour = integer_arg(name, &args[0], span)?;
    let minute = integer_arg(name, &args[1], span)?;
    let second = integer_arg(name, &args[2], span)?;
    Ok(Value::Date(time_serial(hour, minute, second)))
}

fn datevalue(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 1, span)?;
    Ok(Value::Date(parse_date_value(
        &args[0].to_output_string(),
        span,
    )?))
}

fn timevalue(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 1, span)?;
    Ok(Value::Date(parse_time_value(
        &args[0].to_output_string(),
        span,
    )?))
}

fn dateadd(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 3, span)?;
    let interval = args[0].to_output_string();
    let number = integer_arg(name, &args[1], span)?;
    let date = date_arg(name, &args[2], span)?;
    Ok(Value::Date(date_add(&interval, number, date, span)?))
}

fn datediff(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.len() < 3 || args.len() > 5 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "DateDiff expects 3 to 5 arguments",
            Some(span),
        ));
    }
    let interval = args[0].to_output_string();
    let start = date_arg(name, &args[1], span)?;
    let end = date_arg(name, &args[2], span)?;
    Ok(Value::Int64(date_diff(&interval, start, end, span)?))
}

fn datepart(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.len() < 2 || args.len() > 4 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "DatePart expects 2 to 4 arguments",
            Some(span),
        ));
    }
    let interval = args[0].to_output_string();
    let date = date_arg(name, &args[1], span)?;
    Ok(Value::Int64(date_part(&interval, date, span)?))
}

fn year(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 1, span)?;
    let serial = date_arg(name, &args[0], span)?;
    let (year, month, day) = civil_from_days(serial.floor() as i64 - UNIX_EPOCH_AS_VBA);
    let value = match name.to_ascii_lowercase().as_str() {
        "year" => year,
        "month" => i64::from(month),
        "day" => i64::from(day),
        _ => unreachable!(),
    };
    Ok(Value::Int64(value))
}

fn hour(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    expect_value_count(name, args, 1, span)?;
    let serial = date_arg(name, &args[0], span)?;
    let total = seconds_since_midnight(serial);
    let value = match name.to_ascii_lowercase().as_str() {
        "hour" => total / 3600,
        "minute" => (total % 3600) / 60,
        "second" => total % 60,
        _ => unreachable!(),
    };
    Ok(Value::Int64(value))
}

fn weekday(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.is_empty() || args.len() > 2 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "Weekday expects 1 to 2 arguments",
            Some(span),
        ));
    }
    let serial = date_arg(name, &args[0], span)?;
    let first_day = args
        .get(1)
        .and_then(crate::runtime::numeric::value_to_i64)
        .unwrap_or(1);
    Ok(Value::Int64(weekday_value(serial, first_day)))
}

fn monthname(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.is_empty() || args.len() > 2 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "MonthName expects 1 to 2 arguments",
            Some(span),
        ));
    }
    let month = integer_arg(name, &args[0], span)?;
    let abbreviate = args.get(1).is_some_and(Value::is_truthy);
    Ok(Value::String(Rc::new(month_name(month, abbreviate, span)?)))
}

fn weekdayname(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    if args.is_empty() || args.len() > 3 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "WeekdayName expects 1 to 3 arguments",
            Some(span),
        ));
    }
    let weekday = integer_arg(name, &args[0], span)?;
    let abbreviate = args.get(1).is_some_and(Value::is_truthy);
    Ok(Value::String(Rc::new(weekday_name(
        weekday, abbreviate, span,
    )?)))
}

fn environ_value(
    name: &str,
    value: &Value,
    span: crate::runtime::Span,
) -> Result<String, Diagnostic> {
    match value {
        Value::String(key) => Ok(std::env::var(key.as_ref()).unwrap_or_default()),
        Value::Empty | Value::Missing => Ok(String::new()),
        _ => {
            let index = integer_arg(name, value, span)?;
            if index <= 0 {
                return Ok(String::new());
            }
            Ok(std::env::vars()
                .nth((index - 1) as usize)
                .map(|(key, value)| format!("{key}={value}"))
                .unwrap_or_default())
        }
    }
}

/// Statements that create or remove a file or directory.
///
/// Each takes the single path argument the dispatcher has already evaluated.
pub(super) const FILE_SYSTEM_HANDLERS: &[(&str, ValueFn)] = &[
    ("Kill", kill_path),
    ("MkDir", make_directory),
    ("RmDir", remove_directory),
    ("ChDir", change_directory),
];

/// Defines a file-system statement that acts on one path.
macro_rules! path_statements {
    ($($fn_name:ident => $method:ident,)*) => {
        $(
            fn $fn_name(
                interpreter: &mut Interpreter,
                _: &str,
                args: &[Value],
                span: crate::runtime::Span,
            ) -> Result<Value, Diagnostic> {
                interpreter.$method(&args[0].to_output_string(), span)?;
                Ok(Value::Empty)
            }
        )*
    };
}

path_statements! {
    kill_path => kill_path,
    make_directory => mkdir_path,
    remove_directory => rmdir_path,
    change_directory => chdir_path,
}

/// Colour and layout builtins that do not belong to a larger area.
pub(super) const MISC_HANDLERS: &[(&str, ValueFn)] = &[
    ("RGB", rgb),
    ("QBColor", qb_color),
    ("Spc", spaces),
    ("Tab", spaces),
];

fn eval_misc_value_function(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    match find_handler(MISC_HANDLERS, name) {
        Some(handler) => handler(interpreter, name, args, span).map(Some),
        None => Ok(None),
    }
}

/// Packs three components into the little-endian order VBA colours use.
fn rgb(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    let red = color_component(name, &args[0], span)?;
    let green = color_component(name, &args[1], span)?;
    let blue = color_component(name, &args[2], span)?;
    Ok(Value::Int64(red | (green << 8) | (blue << 16)))
}

/// The sixteen colours of the original QuickBASIC palette.
fn qb_color(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    const COLORS: [i64; 16] = [
        0x000000, 0x800000, 0x008000, 0x808000, 0x000080, 0x800080, 0x008080, 0xC0C0C0, 0x808080,
        0xFF0000, 0x00FF00, 0xFFFF00, 0x0000FF, 0xFF00FF, 0x00FFFF, 0xFFFFFF,
    ];
    let index = integer_arg(name, &args[0], span)?;
    COLORS
        .get(index as usize)
        .map(|color| Value::Int64(*color))
        .ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "QBColor color must be between 0 and 15",
                Some(span),
            )
        })
}

/// Backs both `Spc` and `Tab`, which both produce runs of spaces.
fn spaces(
    _: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Value, Diagnostic> {
    let count = integer_arg(name, &args[0], span)?.max(0) as usize;
    Ok(Value::String(Rc::new(" ".repeat(count))))
}

fn color_component(
    name: &str,
    value: &Value,
    span: crate::runtime::Span,
) -> Result<i64, Diagnostic> {
    let value = integer_arg(name, value, span)?;
    Ok(value.clamp(0, 255))
}

fn get_attr(path: &str, span: crate::runtime::Span) -> Result<i64, Diagnostic> {
    let metadata = std::fs::metadata(path).map_err(|error| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::FILE_IO,
            format!("Unable to get attributes for '{}': {}", path, error),
            Some(span),
        )
    })?;
    let mut attr = if metadata.is_dir() { 16 } else { 0 };
    if metadata.permissions().readonly() {
        attr |= 1;
    }
    Ok(attr)
}

fn empty_string_matrix() -> Value {
    Value::Array(std::rc::Rc::new(crate::runtime::ArrayValue {
        element_type: crate::runtime::TypeName::Variant,
        elements: Vec::new(),
        bounds: vec![
            crate::runtime::ArrayBound {
                lower: 0,
                upper: -1,
            },
            crate::runtime::ArrayBound { lower: 0, upper: 1 },
        ],
        allocated: true,
        dynamic: true,
    }))
}

fn error_description(number: i64) -> String {
    match number {
        0 => String::new(),
        5 => "Invalid procedure call or argument".to_string(),
        6 => "Overflow".to_string(),
        7 => "Out of memory".to_string(),
        9 => "Subscript out of range".to_string(),
        11 => "Division by zero".to_string(),
        13 => "Type mismatch".to_string(),
        53 => "File not found".to_string(),
        76 => "Path not found".to_string(),
        _ => format!("Application-defined or object-defined error ({number})"),
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
        let code = if args.len() < expected {
            crate::runtime::DiagnosticCode::ARGUMENT_NOT_OPTIONAL
        } else {
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT
        };
        Err(Diagnostic::new(
            code,
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
                crate::runtime::DiagnosticCode::UNSUPPORTED,
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

fn date_add(
    interval: &str,
    number: i64,
    serial: f64,
    span: crate::runtime::Span,
) -> Result<f64, Diagnostic> {
    let days = serial.floor() as i64 - UNIX_EPOCH_AS_VBA;
    let fraction = serial.fract();
    let (year, month, day) = civil_from_days(days);
    let result = match interval.to_ascii_lowercase().as_str() {
        "yyyy" => date_add_months(year, month, day, number * 12) + fraction,
        "q" => date_add_months(year, month, day, number * 3) + fraction,
        "m" => date_add_months(year, month, day, number) + fraction,
        "y" | "d" | "w" => serial + number as f64,
        "ww" => serial + (number * 7) as f64,
        "h" => serial + number as f64 / 24.0,
        "n" => serial + number as f64 / 1_440.0,
        "s" => serial + number as f64 / 86_400.0,
        _ => return Err(invalid_interval(interval, span)),
    };
    Ok(result)
}

fn date_add_months(year: i64, month: u32, day: u32, months: i64) -> f64 {
    let month_index = i64::from(month) - 1 + months;
    let target_year = year + month_index.div_euclid(12);
    let target_month = month_index.rem_euclid(12) + 1;
    let target_day = day.min(days_in_month(target_year, target_month as u32));
    date_serial(target_year, target_month, i64::from(target_day))
}

fn days_in_month(year: i64, month: u32) -> u32 {
    let next_month_index = i64::from(month);
    let next_year = year + next_month_index.div_euclid(12);
    let next_month = next_month_index.rem_euclid(12) + 1;
    let this_month_days = days_from_civil(year, month, 1);
    let next_month_days = days_from_civil(next_year, next_month as u32, 1);
    (next_month_days - this_month_days) as u32
}

fn date_diff(
    interval: &str,
    start: f64,
    end: f64,
    span: crate::runtime::Span,
) -> Result<i64, Diagnostic> {
    let start_days = start.floor() as i64 - UNIX_EPOCH_AS_VBA;
    let end_days = end.floor() as i64 - UNIX_EPOCH_AS_VBA;
    let (start_year, start_month, _) = civil_from_days(start_days);
    let (end_year, end_month, _) = civil_from_days(end_days);
    let diff = match interval.to_ascii_lowercase().as_str() {
        "yyyy" => end_year - start_year,
        "q" => (end_year - start_year) * 4 + (i64::from(end_month) - i64::from(start_month)) / 3,
        "m" => (end_year - start_year) * 12 + i64::from(end_month) - i64::from(start_month),
        "y" | "d" | "w" => end_days - start_days,
        "ww" => (end_days - start_days) / 7,
        "h" => ((end - start) * 24.0).trunc() as i64,
        "n" => ((end - start) * 1_440.0).trunc() as i64,
        "s" => ((end - start) * 86_400.0).trunc() as i64,
        _ => return Err(invalid_interval(interval, span)),
    };
    Ok(diff)
}

fn date_part(interval: &str, serial: f64, span: crate::runtime::Span) -> Result<i64, Diagnostic> {
    let days = serial.floor() as i64 - UNIX_EPOCH_AS_VBA;
    let (year, month, day) = civil_from_days(days);
    let seconds = seconds_since_midnight(serial);
    let value = match interval.to_ascii_lowercase().as_str() {
        "yyyy" => year,
        "q" => ((month - 1) / 3 + 1).into(),
        "m" => month.into(),
        "y" => days - days_from_civil(year, 1, 1) + 1,
        "d" => day.into(),
        "w" => weekday_value(serial, 1),
        "ww" => ((days - days_from_civil(year, 1, 1)) / 7) + 1,
        "h" => seconds / 3600,
        "n" => (seconds % 3600) / 60,
        "s" => seconds % 60,
        _ => return Err(invalid_interval(interval, span)),
    };
    Ok(value)
}

fn invalid_interval(interval: &str, span: crate::runtime::Span) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
        format!("Invalid date interval '{}'", interval),
        Some(span),
    )
}

pub(crate) fn parse_date_value(value: &str, span: crate::runtime::Span) -> Result<f64, Diagnostic> {
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

pub(crate) fn parse_time_value(value: &str, span: crate::runtime::Span) -> Result<f64, Diagnostic> {
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
                crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
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
                        crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
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
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
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
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            format!("{name} expects exactly {expected} argument(s)"),
            Some(span),
        ))
    }
}
