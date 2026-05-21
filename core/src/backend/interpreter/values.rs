use std::collections::HashMap;

use crate::runtime::{Diagnostic, Span, TypeName, Value, coerce_assignment};
use crate::{BinaryOp, Expr, ExprKind, UnaryOp};

use super::records::{RuntimeEnum, RuntimeType};

pub(crate) fn default_value(
    ty: &TypeName,
    types: &HashMap<String, RuntimeType>,
    enums: &HashMap<String, RuntimeEnum>,
    span: Span,
) -> Result<Value, Diagnostic> {
    if let Some(value) = ty.builtin_default_value() {
        return Ok(value);
    }

    let TypeName::User(name) = ty else {
        unreachable!("builtin types are handled above");
    };
    if name.eq_ignore_ascii_case("Object") {
        return Ok(Value::Nothing);
    }
    if enums.contains_key(&key(name)) {
        return Ok(Value::Int64(0));
    }
    let type_def = types.get(&key(name)).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Type '{}' is not defined", name),
            Some(span),
        )
    });
    let Ok(type_def) = type_def else {
        return Ok(Value::Nothing);
    };

    let mut fields = HashMap::new();
    for field in &type_def.fields {
        let value = if let Some(initializer) = &field.initializer {
            let value = eval_const_default(initializer, enums, span)?;
            coerce_assignment(&field.ty, value, initializer.span)?
        } else {
            default_value(&field.ty, types, enums, span)?
        };
        fields.insert(key(&field.name), value);
    }

    Ok(Value::Record {
        type_name: type_def.name.clone(),
        fields,
    })
}

fn eval_const_default(
    expr: &Expr,
    enums: &HashMap<String, RuntimeEnum>,
    span: Span,
) -> Result<Value, Diagnostic> {
    match &expr.kind {
        ExprKind::String(value) => Ok(Value::String(value.clone())),
        ExprKind::Integer(value) => Ok(Value::Int64(*value)),
        ExprKind::Double(value) => Ok(Value::Double(*value)),
        ExprKind::Boolean(value) => Ok(Value::Boolean(*value)),
        ExprKind::Empty => Ok(Value::Empty),
        ExprKind::Null => Ok(Value::Null),
        ExprKind::Unary {
            op: UnaryOp::Negate,
            expr,
        } => match eval_const_default(expr, enums, span)? {
            Value::Int64(value) => Ok(Value::Int64(-value)),
            Value::Double(value) => Ok(Value::Double(-value)),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Structure field initializer must be numeric",
                Some(expr.span),
            )),
        },
        ExprKind::Binary { left, op, right } => {
            let left = eval_const_default(left, enums, span)?;
            let right = eval_const_default(right, enums, span)?;
            crate::runtime::ops::eval_binary(
                left,
                match op {
                    BinaryOp::Add => crate::runtime::ops::RuntimeBinaryOp::Add,
                    BinaryOp::Subtract => crate::runtime::ops::RuntimeBinaryOp::Subtract,
                    BinaryOp::Multiply => crate::runtime::ops::RuntimeBinaryOp::Multiply,
                    BinaryOp::Exponent => crate::runtime::ops::RuntimeBinaryOp::Exponent,
                    BinaryOp::Divide => crate::runtime::ops::RuntimeBinaryOp::Divide,
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "Structure field initializer must be a constant expression",
                            Some(expr.span),
                        ));
                    }
                },
                right,
                crate::runtime::compare::RuntimeOptionCompare::Binary,
                span,
            )
        }
        ExprKind::Variable(name) => {
            if let Some(enum_value) = enums
                .values()
                .find_map(|enum_| enum_.members.get(&key(name)).copied())
            {
                Ok(Value::Int64(enum_value))
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Structure field initializer must be a constant expression",
                    Some(expr.span),
                ))
            }
        }
        _ => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Structure field initializer must be a constant expression",
            Some(expr.span),
        )),
    }
}

pub(crate) fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}
