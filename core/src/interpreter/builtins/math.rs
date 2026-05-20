use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64;
use crate::runtime::{Diagnostic, Value};
use crate::Expr;
use super::super::Frame;
use super::super::Interpreter;

pub(crate) fn eval_math(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("Sgn") {
        super::expect_arg_count(name, args, 1, span)?;
        let value =
            interpreter.eval_integer_expr(&args[0], frame, "Sgn argument must be Integer")?;
        return Ok(Some(Value::Integer(value.signum())));
    }
    if name.eq_ignore_ascii_case("Int") {
        super::expect_arg_count(name, args, 1, span)?;
        let value = interpreter.eval_integer_expr(&args[0], frame, "Int argument must be Integer")?;
        return Ok(Some(Value::Integer(value)));
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
            interpreter.eval_integer_expr(&args[0], frame, "Randomize seed must be Integer")? as u64
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
