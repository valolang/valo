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
            ExprKind::Missing => Ok(Value::Missing),
            ExprKind::NamedArg { .. } => Err(Diagnostic::new(crate::runtime::DiagnosticCode::GENERIC, "Named arguments are only valid inside call argument lists", Some(expr.span),)),
            ExprKind::TypeOfIs {
                expr: object_expr,
                class_name,
            } => {
                let value = self.eval_expr(object_expr, frame)?;
                let result = match value {
                    Value::Object(object) => {
                        object.borrow().class_name.eq_ignore_ascii_case(class_name)
                    }
                    Value::Nothing => false,
                    _ => {
                        return Err(Diagnostic::new(crate::runtime::DiagnosticCode::TYPE_MISMATCH, "TypeOf requires a class object", Some(object_expr.span),));
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
                            if let Ok(me) = frame.get("me", expr.span) {
                                if let Ok(value) = self.read_member(&me, name, frame, expr.span) {
                                    return Ok(value);
                                }
                            }
                            Err(error)
                        }
                    }
                }
            }
            ExprKind::MemberAccess { object, field } => {
                if let ExprKind::Variable(name) = &object.kind
                    && name.eq_ignore_ascii_case("Err")
                {
                    if field.eq_ignore_ascii_case("Number") {
                        return Ok(Value::Integer(self.err_number));
                    }
                    if field.eq_ignore_ascii_case("Description") {
                        return Ok(Value::String(self.err_description.clone()));
                    }
                    if field.eq_ignore_ascii_case("Source") {
                        return Ok(Value::String(self.err_source.clone()));
                    }
                    if field.eq_ignore_ascii_case("HelpFile") {
                        return Ok(Value::String(self.err_help_file.clone()));
                    }
                    if field.eq_ignore_ascii_case("HelpContext") {
                        return Ok(Value::Integer(self.err_help_context));
                    }
                }
                if let ExprKind::Variable(enum_name) = &object.kind
                    && let Some(enum_) = self.enums.get(&super::values::key(enum_name))
                {
                    let value = enum_
                        .members
                        .get(&super::values::key(field))
                        .ok_or_else(|| {
                            Diagnostic::new(crate::runtime::DiagnosticCode::MEMBER_ACCESS, format!("Enum '{}' has no member '{}'", enum_.name, field), Some(expr.span),)
                        })?;
                    return Ok(Value::Integer(*value));
                }
                let object = self.eval_expr(object, frame)?;
                self.read_member(&object, field, frame, expr.span)
            }
            ExprKind::Call { name, args } => {
                if name.eq_ignore_ascii_case("IsMissing") {
                    if args.len() != 1 {
                        return Err(Diagnostic::new(crate::runtime::DiagnosticCode::GENERIC, "IsMissing expects exactly one argument", Some(expr.span),));
                    }
                    let value = self.eval_expr(&args[0], frame)?;
                    return Ok(Value::Boolean(matches!(value, Value::Missing)));
                }
                if name.eq_ignore_ascii_case("LBound") || name.eq_ignore_ascii_case("UBound") {
                    if args.len() != 1 {
                        return Err(Diagnostic::new(crate::runtime::DiagnosticCode::GENERIC, format!("{} expects exactly one argument", name), Some(expr.span),));
                    }
                    let value = self.eval_expr(&args[0], frame)?;
                    let bound = if name.eq_ignore_ascii_case("LBound") {
                        super::arrays::lbound(&value, expr.span)?
                    } else {
                        super::arrays::ubound(&value, expr.span)?
                    };
                    return Ok(Value::Integer(bound));
                }
                if frame.has_variable(name) {
                    if args.len() != 1 {
                        return Err(Diagnostic::new(crate::runtime::DiagnosticCode::ARRAY, "Array access requires exactly one index", Some(expr.span),));
                    }
                    let index =
                        self.eval_integer_expr(&args[0], frame, "Array index must be Integer")?;
                    return frame.get_array_element(name, index, expr.span);
                }
                self.call_function(name, args, frame, expr.span)
            }
            ExprKind::MemberCall {
                object,
                method,
                args,
            } => {
                let object = self.eval_expr(object, frame)?;
                self.call_method_function(object, method, args, frame, expr.span)
            }
            ExprKind::Unary { op, expr: inner } => {
                let value = self.eval_expr(inner, frame)?;
                match (op, value) {
                    (UnaryOp::Negate, Value::Integer(value)) => Ok(Value::Integer(-value)),
                    (UnaryOp::Negate, _) => Err(Diagnostic::new(crate::runtime::DiagnosticCode::TYPE_MISMATCH, "Unary '-' requires an Integer expression", Some(expr.span),)),
                    (UnaryOp::LogicalNot, Value::Boolean(value)) => Ok(Value::Boolean(!value)),
                    (UnaryOp::LogicalNot, _) => Err(Diagnostic::new(crate::runtime::DiagnosticCode::TYPE_MISMATCH, "Not requires a Boolean expression", Some(expr.span),)),
                }
            }
            ExprKind::Binary { left, op, right } => {
                let left_value = self.eval_expr(left, frame)?;
                if matches!(left_value, Value::Missing) {
                    return Err(Diagnostic::new(crate::runtime::DiagnosticCode::GENERIC, "Missing optional argument cannot be used as a value", Some(left.span),));
                }
                let left = self.resolve_default_value(left_value, expr.span)?;
                let right_value = self.eval_expr(right, frame)?;
                if matches!(right_value, Value::Missing) {
                    return Err(Diagnostic::new(crate::runtime::DiagnosticCode::GENERIC, "Missing optional argument cannot be used as a value", Some(right.span),));
                }
                let right = self.resolve_default_value(right_value, expr.span)?;
                eval_binary(left, *op, right, self.option_compare, expr.span)
            }
        }
    }

    pub(crate) fn resolve_default_value(
        &mut self,
        value: Value,
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
        self.call_property_get(value, &default_member, span)
    }

    pub(crate) fn eval_integer_expr(
        &mut self,
        expr: &Expr,
        frame: &mut Frame,
        message: &str,
    ) -> Result<i64, Diagnostic> {
        match self.eval_expr(expr, frame)? {
            Value::Integer(value) => Ok(value),
            _ => Err(Diagnostic::new(crate::runtime::DiagnosticCode::GENERIC, message, Some(expr.span))),
        }
    }
}
