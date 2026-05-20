use super::super::Interpreter;
use crate::runtime::numeric::{value_to_f64, value_to_i64};
use crate::runtime::{Diagnostic, Value};
use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64;

pub(crate) fn eval_math(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("Sgn") {
        expect_value_count(name, args, 1, span)?;
        let num = value_to_f64(&args[0]).ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Sgn requires a numeric argument",
                Some(span),
            )
        })?;
        return Ok(Some(Value::Int16(if num > 0.0 {
            1
        } else if num < 0.0 {
            -1
        } else {
            0
        })));
    }
    if name.eq_ignore_ascii_case("Int") {
        expect_value_count(name, args, 1, span)?;
        let num = value_to_f64(&args[0]).ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Int requires a numeric argument",
                Some(span),
            )
        })?;
        return Ok(Some(Value::Int64(num.floor() as i64)));
    }
    if name.eq_ignore_ascii_case("Randomize") {
        if args.len() > 1 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Randomize expects at most 1 argument",
                Some(span),
            ));
        }
        let seed = if args.is_empty() {
            rand::thread_rng().r#gen::<u64>()
        } else {
            value_to_i64(&args[0]).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Randomize seed must be Integer",
                    Some(span),
                )
            })? as u64
        };
        interpreter.rng = Pcg64::seed_from_u64(seed);
        return Ok(Some(Value::Empty));
    }
    if name.eq_ignore_ascii_case("Rnd") {
        if args.len() > 1 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Rnd expects at most 1 argument",
                Some(span),
            ));
        }
        return Ok(Some(Value::Double(interpreter.rng.r#gen::<f64>())));
    }

    Ok(None)
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
