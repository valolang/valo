use crate::runtime::{Diagnostic, Value};
use crate::{Expr, ExprKind, UnaryOp};

use super::values::eval_binary;
use super::{Frame, Interpreter};

impl Interpreter {
    pub(crate) fn eval_expr(
        &mut self,
        expr: &Expr,
        frame: &mut Frame,
    ) -> Result<Value, Diagnostic> {
        match &expr.kind {
            ExprKind::String(value) => Ok(Value::String(value.clone())),
            ExprKind::Integer(value) => Ok(Value::Integer(*value)),
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
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "TypeOf requires a class object",
                            Some(object_expr.span),
                        ));
                    }
                };
                Ok(Value::Boolean(result))
            }
            ExprKind::Me => frame.get("me", expr.span),
            ExprKind::WithTarget => frame.current_with_target(expr.span),
            ExprKind::New { class_name, args } => {
                self.new_object(class_name, args, frame, expr.span)
            }
            ExprKind::Variable(name) => {
                if name.eq_ignore_ascii_case("Erl") {
                    Ok(Value::Integer(self.erl))
                } else if let Some(value) = self.enum_members.get(&super::values::key(name)) {
                    Ok(Value::Integer(*value))
                } else {
                    match frame.get(name, expr.span) {
                        Ok(value) => Ok(value),
                        Err(error) => {
                            if let Ok(me) = frame.get("me", expr.span)
                                && let Ok(value) = self.read_member(&me, name, frame, expr.span)
                            {
                                return Ok(value);
                            }

                            Err(error)
                        }
                    }
                }
            }
            ExprKind::MemberAccess { object, field } => {
                if let ExprKind::Variable(name) = &object.kind
                    && name.eq_ignore_ascii_case("Err")
                    && let Some(val) = super::builtins::err::eval_err(self, field, expr.span)?
                {
                    return Ok(val);
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
                    return Ok(Value::Integer(*value));
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
                        return Ok(Value::Integer(*value));
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
                        _ => {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                format!("Variable '{}' is not an array", name),
                                Some(expr.span),
                            ));
                        }
                    }
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
                self.call_method_function(object, method, args, frame, expr.span)
            }
            ExprKind::Unary { op, expr: inner } => {
                let value = self.eval_expr(inner, frame)?;
                match (op, value) {
                    (UnaryOp::Negate, Value::Integer(value)) => Ok(Value::Integer(-value)),
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
                eval_binary(left, *op, right, self.option_compare, expr.span)
            }
        }
    }

    pub(crate) fn resolve_default_value(
        &mut self,
        value: Value,
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<Value, Diagnostic> {
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
            Value::Integer(value) => Ok(value),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                message,
                Some(expr.span),
            )),
        }
    }
}
