use crate::runtime::{Diagnostic, Value};
use crate::{Expr, ExprKind, UnaryOp};

use super::{Frame, Interpreter};
use crate::runtime::compare::RuntimeOptionCompare;
use crate::runtime::ops::{RuntimeBinaryOp, eval_binary};

impl Interpreter {
    pub(crate) fn eval_expr(
        &mut self,
        expr: &Expr,
        frame: &mut Frame,
    ) -> Result<Value, Diagnostic> {
        match &expr.kind {
            ExprKind::String(value) => Ok(Value::String(value.clone())),
            ExprKind::Integer(value) => {
                let val = *value;
                if val >= i16::MIN as i64 && val <= i16::MAX as i64 {
                    Ok(Value::Int16(val as i16))
                } else if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
                    Ok(Value::Int32(val as i32))
                } else {
                    Ok(Value::Int64(val))
                }
            }
            ExprKind::Double(value) => Ok(Value::Double(*value)),
            ExprKind::Boolean(value) => Ok(Value::Boolean(*value)),
            ExprKind::Nothing => Ok(Value::Nothing),
            ExprKind::Empty => Ok(Value::Empty),
            ExprKind::Null => Ok(Value::Null),
            ExprKind::Missing => Ok(Value::Missing),
            ExprKind::NamedArg { .. } => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Named arguments are only valid inside call argument lists",
                Some(expr.span),
            )),
            ExprKind::TypeOfIs {
                expr: object_expr,
                class_name,
            } => {
                let value = self.eval_expr(object_expr, frame)?;
                let result = match value {
                    Value::Object(object) => {
                        let object_class = object.borrow().class_name.clone();
                        object_class.eq_ignore_ascii_case(class_name)
                            || object_class
                                .rsplit_once('.')
                                .is_some_and(|(_, local)| local.eq_ignore_ascii_case(class_name))
                    }
                    Value::Nothing => false,
                    _ => false,
                };
                Ok(Value::Boolean(result))
            }
            ExprKind::Me => frame.get("me", expr.span),
            ExprKind::WithTarget => frame.current_with_target(expr.span),
            ExprKind::New { class_name, args } => {
                self.new_object(class_name, args, frame, expr.span)
            }
            ExprKind::IIf {
                condition,
                true_expr,
                false_expr,
            } => {
                let cond = self.eval_expr(condition, frame)?;
                if cond.is_truthy() {
                    self.eval_expr(true_expr, frame)
                } else {
                    self.eval_expr(false_expr, frame)
                }
            }
            ExprKind::Index { target, args } => {
                let target_val = self.eval_expr(target, frame)?;
                self.eval_index_expr(target_val, args, frame, expr.span)
            }
            ExprKind::Variable(name) => {
                if name.eq_ignore_ascii_case("Erl") {
                    Ok(Value::Int64(self.erl))
                } else if name.eq_ignore_ascii_case("VBA")
                    || name.eq_ignore_ascii_case("Console")
                    || name.eq_ignore_ascii_case("Err")
                {
                    Ok(Value::Empty)
                } else if let Some(value) = self.enum_members.get(&super::values::key(name)) {
                    Ok(Value::Int64(*value))
                } else {
                    match frame.get(name, expr.span) {
                        Ok(value) => Ok(value),
                        Err(error) => {
                            if let Ok(me) = frame.get("me", expr.span)
                                && let Ok(value) = self.read_member(&me, name, frame, expr.span)
                            {
                                return Ok(value);
                            }
                            if self.functions.contains_key(&super::values::key(name))
                                || self.has_declared_function(name)
                            {
                                return self.call_function(name, &[], frame, expr.span);
                            }

                            Err(error)
                        }
                    }
                }
            }
            ExprKind::MemberAccess { object, field } => {
                if let ExprKind::Variable(name) = &object.kind {
                    if name.eq_ignore_ascii_case("Err")
                        && let Some(val) = super::builtins::err::eval_err(self, field, expr.span)?
                    {
                        return Ok(val);
                    }
                    if name.eq_ignore_ascii_case("VBA") || name.eq_ignore_ascii_case("Console") {
                        // VBA doesn't have fields in our current builtin model, but we handle it here
                        // to prevent it being treated as a potential module qualifier that might fail later.
                        // Actually dispatch_function handles it for MemberCall.
                    }
                }
                if let ExprKind::Variable(enum_name) = &object.kind
                    && let Some(enum_) = self.enums.get(&super::values::key(enum_name))
                {
                    let value = enum_
                        .members
                        .get(&super::values::key(field))
                        .ok_or_else(|| {
                            Diagnostic::new(
                                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                format!("Enum '{}' has no member '{}'", enum_.name, field),
                                Some(expr.span),
                            )
                        })?;
                    return Ok(Value::Int64(*value));
                }
                if let ExprKind::MemberAccess {
                    object: module_object,
                    field: enum_name,
                } = &object.kind
                    && let ExprKind::Variable(module_name) = &module_object.kind
                    && let Ok(module_key) =
                        self.resolve_module_qualifier(module_name, frame, expr.span)
                {
                    let enum_key = super::interpreter::qualified_symbol_key(&module_key, enum_name);
                    if let Some(enum_) = self.enums.get(&enum_key) {
                        if frame.module_key() != Some(module_key.as_str())
                            && !self.public_enums.contains(&enum_key)
                        {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::PRIVATE_ACCESS,
                                format!("Enum '{}.{}' is Private", module_name, enum_name),
                                Some(expr.span),
                            ));
                        }
                        let value =
                            enum_
                                .members
                                .get(&super::values::key(field))
                                .ok_or_else(|| {
                                    Diagnostic::new(
                                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                        format!("Enum '{}' has no member '{}'", enum_.name, field),
                                        Some(expr.span),
                                    )
                                })?;
                        return Ok(Value::Int64(*value));
                    }
                }
                if let ExprKind::Variable(module_name) = &object.kind
                    && let Ok(module_key) =
                        self.resolve_module_qualifier(module_name, frame, expr.span)
                {
                    let Some(module_frame) = self.module_frames.get(&module_key) else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                            format!("Module '{}' is not loaded", module_name),
                            Some(expr.span),
                        ));
                    };
                    let value = module_frame.get(field, expr.span).map_err(|_| {
                        Diagnostic::new(
                            crate::runtime::DiagnosticCode::UNKNOWN_QUALIFIED_SYMBOL,
                            format!("Module '{}' has no member '{}'", module_name, field),
                            Some(expr.span),
                        )
                    })?;
                    if frame.module_key() != Some(module_key.as_str())
                        && !self
                            .public_values
                            .get(&module_key)
                            .is_some_and(|values| values.contains(&super::values::key(field)))
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::PRIVATE_ACCESS,
                            format!("Module member '{}.{}' is Private", module_name, field),
                            Some(expr.span),
                        ));
                    }
                    return Ok(value);
                }
                let object = self.eval_expr(object, frame)?;
                self.read_member(&object, field, frame, expr.span)
            }
            ExprKind::Call { name, args } => {
                if let Some(value) =
                    super::builtins::dispatch_function(self, name, args, frame, expr.span)?
                {
                    return Ok(value);
                }
                if frame.has_variable(name) {
                    let value = frame.get(name, expr.span)?;
                    match value {
                        Value::Array { .. } => {
                            let mut dims = Vec::new();
                            for arg in args {
                                dims.push(self.eval_integer_expr(
                                    arg,
                                    frame,
                                    "Array index must be Integer",
                                )?);
                            }
                            return frame.get_array_element(name, &dims, expr.span);
                        }
                        Value::Object(ref object) => {
                            let class_name = object.borrow().class_name.clone();
                            if let Some(default_member) = self
                                .classes
                                .get(&super::values::key(&class_name))
                                .and_then(|c| c.default_member.clone())
                            {
                                return self.call_method_function(
                                    value.clone(),
                                    &default_member,
                                    args,
                                    frame,
                                    expr.span,
                                );
                            }
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                format!(
                                    "Variable '{}' is not an array or a class with a default property",
                                    name
                                ),
                                Some(expr.span),
                            ));
                        }
                        Value::Record { ref type_name, .. } => {
                            if let Some(default_member) = self
                                .types
                                .get(&super::values::key(type_name))
                                .and_then(|t| t.default_property.clone())
                            {
                                return self.call_record_function(
                                    value.clone(),
                                    &default_member,
                                    args,
                                    frame,
                                    expr.span,
                                );
                            }
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                format!(
                                    "Variable '{}' is not an array or a Structure with a default property",
                                    name
                                ),
                                Some(expr.span),
                            ));
                        }
                        _ => {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                format!("Variable '{}' is not an array", name),
                                Some(expr.span),
                            ));
                        }
                    }
                }
                if let Ok(me) = frame.get("me", expr.span)
                    && let Ok(field_value) = self.read_member(&me, name, frame, expr.span)
                    && matches!(field_value, Value::Array { .. })
                {
                    let mut dims = Vec::new();
                    for arg in args {
                        dims.push(self.eval_integer_expr(
                            arg,
                            frame,
                            "Array index must be Integer",
                        )?);
                    }
                    return self.read_bare_class_field_array_element(me, name, &dims, expr.span);
                }
                self.call_function(name, args, frame, expr.span)
            }
            ExprKind::MemberCall {
                object,
                method,
                args,
            } => {
                if let ExprKind::Variable(module_name) = &object.kind
                    && self
                        .resolve_module_qualifier(module_name, frame, expr.span)
                        .is_ok()
                {
                    return self.call_module_function(module_name, method, args, frame, expr.span);
                }
                let object = self.eval_expr(object, frame)?;
                if matches!(object, Value::Record { .. }) {
                    return self.call_record_function(object, method, args, frame, expr.span);
                }
                self.call_method_function(object, method, args, frame, expr.span)
            }
            ExprKind::Unary { op, expr: inner } => {
                let value = self.eval_expr(inner, frame)?;
                match (op, value) {
                    (UnaryOp::Negate, Value::Int64(value)) => Ok(Value::Int64(-value)),
                    (UnaryOp::Negate, Value::Int32(value)) => Ok(Value::Int32(-value)),
                    (UnaryOp::Negate, Value::Int16(value)) => Ok(Value::Int16(-value)),
                    (UnaryOp::Negate, _) => Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        "Unary '-' requires an Integer expression",
                        Some(expr.span),
                    )),
                    (UnaryOp::LogicalNot, Value::Boolean(value)) => Ok(Value::Boolean(!value)),
                    (UnaryOp::LogicalNot, _) => Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        "Not requires a Boolean expression",
                        Some(expr.span),
                    )),
                }
            }
            ExprKind::AddressOf(inner) => {
                let name = match &inner.kind {
                    ExprKind::Variable(name) => name,
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::GENERIC,
                            "AddressOf target must be a method name",
                            Some(inner.span),
                        ));
                    }
                };
                let ptr = self.create_callback(name, expr.span)?;
                Ok(Value::FuncPtr(ptr))
            }
            ExprKind::Binary { left, op, right } => {
                let left_value = self.eval_expr(left, frame)?;
                if matches!(left_value, Value::Missing) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "Missing optional argument cannot be used as a value",
                        Some(left.span),
                    ));
                }
                let left = self.resolve_default_value(left_value, frame, expr.span)?;
                let right_value = self.eval_expr(right, frame)?;
                if matches!(right_value, Value::Missing) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "Missing optional argument cannot be used as a value",
                        Some(right.span),
                    ));
                }
                let right = self.resolve_default_value(right_value, frame, expr.span)?;
                let runtime_op = match op {
                    crate::BinaryOp::Add => RuntimeBinaryOp::Add,
                    crate::BinaryOp::Subtract => RuntimeBinaryOp::Subtract,
                    crate::BinaryOp::Multiply => RuntimeBinaryOp::Multiply,
                    crate::BinaryOp::Exponent => RuntimeBinaryOp::Exponent,
                    crate::BinaryOp::Divide => RuntimeBinaryOp::Divide,
                    crate::BinaryOp::IntegerDivide => RuntimeBinaryOp::IntegerDivide,
                    crate::BinaryOp::Modulo => RuntimeBinaryOp::Modulo,
                    crate::BinaryOp::Concat => RuntimeBinaryOp::Concat,
                    crate::BinaryOp::LogicalAnd => RuntimeBinaryOp::LogicalAnd,
                    crate::BinaryOp::LogicalOr => RuntimeBinaryOp::LogicalOr,
                    crate::BinaryOp::LogicalXor => RuntimeBinaryOp::LogicalXor,
                    crate::BinaryOp::LogicalEqv => RuntimeBinaryOp::LogicalEqv,
                    crate::BinaryOp::LogicalImp => RuntimeBinaryOp::LogicalImp,
                    crate::BinaryOp::Equal => RuntimeBinaryOp::Equal,
                    crate::BinaryOp::NotEqual => RuntimeBinaryOp::NotEqual,
                    crate::BinaryOp::Less => RuntimeBinaryOp::Less,
                    crate::BinaryOp::Greater => RuntimeBinaryOp::Greater,
                    crate::BinaryOp::LessEqual => RuntimeBinaryOp::LessEqual,
                    crate::BinaryOp::GreaterEqual => RuntimeBinaryOp::GreaterEqual,
                    crate::BinaryOp::Is => RuntimeBinaryOp::Is,
                    crate::BinaryOp::Like => RuntimeBinaryOp::Like,
                };
                let runtime_compare = match self.option_compare {
                    crate::OptionCompare::Binary => RuntimeOptionCompare::Binary,
                    crate::OptionCompare::Text => RuntimeOptionCompare::Text,
                };
                eval_binary(left, runtime_op, right, runtime_compare, expr.span)
            }
        }
    }

    pub(crate) fn eval_index_expr(
        &mut self,
        target: Value,
        args: &[crate::Expr],
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<Value, Diagnostic> {
        match target {
            Value::Array { .. } => {
                let mut dims = Vec::new();
                for arg in args {
                    dims.push(self.eval_integer_expr(arg, frame, "Array index must be Integer")?);
                }
                super::arrays::read_array_element(&target, &dims, span)
            }
            Value::Object(ref object) => {
                let class_name = object.borrow().class_name.clone();
                if let Some(default_member) = self
                    .classes
                    .get(&super::values::key(&class_name))
                    .and_then(|c| c.default_member.clone())
                {
                    return self.call_method_function(
                        target.clone(),
                        &default_member,
                        args,
                        frame,
                        span,
                    );
                }
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    format!("Class '{}' has no default property", class_name),
                    Some(span),
                ))
            }
            Value::Record { ref type_name, .. } => {
                if let Some(default_member) = self
                    .types
                    .get(&super::values::key(type_name))
                    .and_then(|t| t.default_property.clone())
                {
                    return self.call_record_function(
                        target.clone(),
                        &default_member,
                        args,
                        frame,
                        span,
                    );
                }
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    format!("Structure '{}' has no default property", type_name),
                    Some(span),
                ))
            }
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                "Target is not an array or a class with a default property",
                Some(span),
            )),
        }
    }

    pub(crate) fn resolve_default_value(
        &mut self,
        value: Value,
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<Value, Diagnostic> {
        if matches!(value, Value::Record { .. }) {
            let type_name = match &value {
                Value::Record { type_name, .. } => type_name.clone(),
                _ => unreachable!(),
            };
            if let Some(type_def) = self.types.get(&super::values::key(&type_name))
                && let Some(default_member) = type_def.default_property.clone()
            {
                return self.call_record_property_get(value, &default_member, &[], frame, span);
            }
            return Ok(value);
        }
        let Value::Object(object) = &value else {
            return Ok(value);
        };
        let class_name = object.borrow().class_name.clone();
        let Some(class) = self.classes.get(&super::values::key(&class_name)) else {
            return Ok(value);
        };
        let Some(default_member) = class.default_member.clone() else {
            return Ok(value);
        };
        self.call_property_get(value, &default_member, &[], frame, span)
    }

    pub(crate) fn eval_integer_expr(
        &mut self,
        expr: &Expr,
        frame: &mut Frame,
        message: &str,
    ) -> Result<i64, Diagnostic> {
        match self.eval_expr(expr, frame)? {
            Value::Byte(value) => Ok(value as i64),
            Value::Int16(value) => Ok(value as i64),
            Value::Int32(value) => Ok(value as i64),
            Value::Int64(value) => Ok(value),
            Value::UInt32(value) => Ok(value as i64),
            Value::UInt64(value) => Ok(value as i64),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                message,
                Some(expr.span),
            )),
        }
    }
}
