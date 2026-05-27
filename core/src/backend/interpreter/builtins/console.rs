use crate::runtime::{Diagnostic, Value};

pub(crate) fn exec_console(
    method: &str,
    args: &[Value],
    _span: crate::runtime::Span,
) -> Result<Option<String>, Diagnostic> {
    if method.eq_ignore_ascii_case("WriteLine") || method.eq_ignore_ascii_case("Write") {
        let mut parts = Vec::new();
        for value in args {
            if matches!(value, Value::Missing) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Optional argument was omitted here and cannot be printed as a value",
                    None,
                ));
            }
            parts.push(value.to_output_string());
        }
        let line = parts.join(" ");
        if method.eq_ignore_ascii_case("Write") {
            print!("{line}");
            use std::io::Write;
            let _ = std::io::stdout().flush();
            return Ok(None);
        } else {
            return Ok(Some(line));
        }
    }

    if method.eq_ignore_ascii_case("ReadLine") {
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_ok() {
            return Ok(Some(line.trim_end_matches(['\r', '\n']).to_string()));
        }
        return Ok(Some(String::new()));
    }

    Ok(None)
}
