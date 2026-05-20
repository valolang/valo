use crate::runtime::{Diagnostic, Value};

pub(crate) fn eval_arrays(
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("Array") {
        let len = args.len() as i64;
        return Ok(Some(Value::Array {
            element_type: crate::runtime::TypeName::Variant,
            elements: args.to_vec(),
            bounds: vec![crate::runtime::ArrayBound {
                lower: 0,
                upper: len - 1,
            }],
            allocated: true,
        }));
    }

    if name.eq_ignore_ascii_case("LBound") || name.eq_ignore_ascii_case("UBound") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                format!("{} expects one array argument and optional dimension", name),
                Some(span),
            ));
        }
        let dimension = if args.len() == 2 {
            super::super::values::value_to_i64(&args[1]).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Array dimension must be Integer",
                    Some(span),
                )
            })? as usize
        } else {
            1
        };
        let value = &args[0];
        let bound = if name.eq_ignore_ascii_case("LBound") {
            super::super::arrays::lbound(value, dimension, span)?
        } else {
            super::super::arrays::ubound(value, dimension, span)?
        };
        return Ok(Some(Value::Int64(bound)));
    }

    Ok(None)
}
