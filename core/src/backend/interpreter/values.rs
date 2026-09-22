use crate::runtime::well_known;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::numeric::{unary_negate, unary_positive};
use crate::runtime::{
    ArrayValue, Diagnostic, RecordValue, Span, TypeName, Value, coerce_assignment,
};
use crate::{BinaryOp, Expr, ExprKind, UnaryOp};

use super::Interpreter;
use super::records::RuntimeEnum;

pub(crate) fn default_value(
    ty: &TypeName,
    interpreter: &Interpreter,
    span: Span,
) -> Result<Value, Diagnostic> {
    if let Some(value) = ty.builtin_default_value() {
        return Ok(value);
    }

    // A tuple has no declaration to look up: its default is built straight from
    // its elements', the same way a tuple literal is built from its values.
    if let TypeName::Tuple(elements) = ty {
        let mut fields = HashMap::with_capacity(elements.len() * 2);
        for (index, element) in elements.iter().enumerate() {
            let value = default_value(&element.ty, interpreter, span)?;
            fields.insert(
                key(&crate::runtime::TupleElement::positional_name(index)),
                value.clone(),
            );
            if let Some(name) = &element.name {
                fields.insert(key(name), value);
            }
        }
        return Ok(Value::Record(Rc::new(RecordValue {
            type_name: ty.display_name(),
            resolved_type: ty.clone(),
            fields,
        })));
    }

    let (name, display_name, bindings) = match ty {
        TypeName::User(name) => (name.clone(), name.clone(), Vec::new()),
        TypeName::GenericInstance { name, args } => {
            let params = interpreter
                .types
                .get(&key(name))
                .map(|type_def| type_def.type_params.clone())
                .or_else(|| {
                    interpreter
                        .interfaces
                        .get(&key(name))
                        .map(|interface_def| interface_def.type_params.clone())
                })
                .or_else(|| {
                    interpreter
                        .classes
                        .get(&key(name))
                        .map(|class_def| class_def.type_params.clone())
                })
                .unwrap_or_default();
            (
                name.clone(),
                ty.display_name(),
                params.into_iter().zip(args.iter().cloned()).collect(),
            )
        }
        _ => unreachable!("builtin types are handled above"),
    };
    if name.eq_ignore_ascii_case(well_known::OBJECT) || name.eq_ignore_ascii_case(well_known::FUNC)
    {
        return Ok(Value::Nothing);
    }
    if interpreter.enums.contains_key(&key(&name)) {
        return Ok(Value::Int64(0));
    }
    if interpreter.interfaces.contains_key(&key(&name)) {
        return Ok(Value::Nothing);
    }
    if interpreter.classes.contains_key(&key(&name)) {
        return Ok(Value::Nothing);
    }
    // A delegate-typed variable starts empty and holds a lambda or a procedure
    // address once one is assigned. There is nothing to construct.
    if interpreter.delegates.contains(&key(&name)) {
        return Ok(Value::Nothing);
    }
    let type_def = interpreter
        .types
        .get(&key(&display_name))
        .or_else(|| interpreter.types.get(&key(&name)))
        .ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Type '{}' is not defined", display_name),
                Some(span),
            )
        })?;

    let mut fields = HashMap::new();
    for field in &type_def.fields {
        let field_ty = resolve_record_field_type(&field.ty, &type_def.name, interpreter);
        let field_ty = field_ty.substitute_generics(&bindings);
        let value = if let Some(array) = &field.array {
            default_array_value(&field_ty, array, interpreter, span)?
        } else if let Some(initializer) = &field.initializer {
            let value = eval_const_default(initializer, &interpreter.enums, span)?;
            coerce_assignment(&field_ty, value, initializer.span)?
        } else {
            default_value(&field_ty, interpreter, span)?
        };
        fields.insert(key(&field.name), value);
    }

    Ok(Value::Record(Rc::new(RecordValue {
        type_name: display_name,
        resolved_type: ty.clone(),
        fields,
    })))
}

fn default_array_value(
    element_type: &TypeName,
    array: &crate::ArrayDecl,
    interpreter: &Interpreter,
    span: Span,
) -> Result<Value, Diagnostic> {
    let mut elements = Vec::new();
    let mut bounds = Vec::new();
    let allocated = match array {
        crate::ArrayDecl::Fixed(fixed_bounds) => {
            let mut total_len: usize = 1;
            for bound in fixed_bounds {
                total_len *= (bound.upper - bound.lower + 1) as usize;
                bounds.push(*bound);
            }
            for _ in 0..total_len {
                elements.push(default_value(element_type, interpreter, span)?);
            }
            true
        }
        crate::ArrayDecl::Dynamic => false,
    };

    Ok(Value::Array(Rc::new(ArrayValue {
        element_type: element_type.clone(),
        elements,
        bounds,
        allocated,
        dynamic: matches!(array, crate::ArrayDecl::Dynamic),
    })))
}

fn resolve_record_field_type(
    ty: &TypeName,
    owner_type_name: &str,
    interpreter: &Interpreter,
) -> TypeName {
    match ty {
        TypeName::User(name) if !name.contains('.') => owner_type_name
            .rsplit_once('.')
            .map(|(module, _)| format!("{module}.{name}"))
            .filter(|qualified| {
                let key = key(qualified);
                interpreter.types.contains_key(&key)
                    || interpreter.classes.contains_key(&key)
                    || interpreter.interfaces.contains_key(&key)
                    || interpreter.enums.contains_key(&key)
            })
            .map(TypeName::User)
            .unwrap_or_else(|| ty.clone()),
        TypeName::GenericInstance { name, args } if !name.contains('.') => {
            let resolved_name = owner_type_name
                .rsplit_once('.')
                .map(|(module, _)| format!("{module}.{name}"))
                .filter(|qualified| {
                    let key = key(qualified);
                    interpreter.types.contains_key(&key)
                        || interpreter.classes.contains_key(&key)
                        || interpreter.interfaces.contains_key(&key)
                        || interpreter.enums.contains_key(&key)
                })
                .unwrap_or_else(|| name.clone());
            TypeName::GenericInstance {
                name: resolved_name,
                args: args
                    .iter()
                    .map(|arg| resolve_record_field_type(arg, owner_type_name, interpreter))
                    .collect(),
            }
        }
        TypeName::Array(inner) => TypeName::Array(Box::new(resolve_record_field_type(
            inner,
            owner_type_name,
            interpreter,
        ))),
        _ => ty.clone(),
    }
}

fn eval_const_default(
    expr: &Expr,
    enums: &HashMap<String, RuntimeEnum>,
    span: Span,
) -> Result<Value, Diagnostic> {
    match &expr.kind {
        ExprKind::String(value) => Ok(Value::String(Rc::new(value.clone()))),
        ExprKind::Integer(value) => Ok(Value::Int64(*value)),
        ExprKind::Long(value) => Ok(Value::Int32(*value)),
        ExprKind::LongLong(value) => Ok(Value::Int64(*value)),
        ExprKind::Single(value) => Ok(Value::Single(*value)),
        ExprKind::Double(value) => Ok(Value::Double(*value)),
        ExprKind::Currency(value) => Ok(Value::Currency(*value)),
        ExprKind::Decimal(value) => Ok(Value::Decimal(*value)),
        ExprKind::Boolean(value) => Ok(Value::Boolean(*value)),
        ExprKind::Empty => Ok(Value::Empty),
        ExprKind::Null => Ok(Value::Null),
        ExprKind::Unary { op, expr } => {
            let value = eval_const_default(expr, enums, span)?;
            match op {
                UnaryOp::Positive => unary_positive(value, expr.span),
                UnaryOp::Negate => unary_negate(value, expr.span),
                UnaryOp::LogicalNot => Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Structure field initializer must be numeric",
                    Some(expr.span),
                )),
            }
        }
        ExprKind::AddressOf(_) => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "AddressOf is not allowed in constant expressions",
            Some(expr.span),
        )),
        ExprKind::PassingModeOverride { expr: inner, .. } => eval_const_default(inner, enums, span),
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
    crate::runtime::fold(name)
}

/// Calls `f` with the case-folded lookup key for `name`, avoiding the
/// allocation that `key` performs.
pub(crate) fn with_key<R>(name: &str, f: impl FnOnce(&str) -> R) -> R {
    crate::runtime::with_folded(name, f)
}
