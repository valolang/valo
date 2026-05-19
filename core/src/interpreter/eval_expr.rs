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
                if let Some(value) = self.eval_builtin_function(name, args, frame, expr.span)? {
                    return Ok(value);
                }
                if name.eq_ignore_ascii_case("IsMissing") {
                    if args.len() != 1 {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::GENERIC,
                            "IsMissing expects exactly one argument",
                            Some(expr.span),
                        ));
                    }
                    let value = self.eval_expr(&args[0], frame)?;
                    return Ok(Value::Boolean(matches!(value, Value::Missing)));
                }
                if name.eq_ignore_ascii_case("LBound") || name.eq_ignore_ascii_case("UBound") {
                    if args.is_empty() || args.len() > 2 {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::GENERIC,
                            format!("{} expects one array argument and optional dimension", name),
                            Some(expr.span),
                        ));
                    }
                    if args.len() == 2 {
                        let dimension = self.eval_integer_expr(
                            &args[1],
                            frame,
                            "Array dimension must be Integer",
                        )?;
                        if dimension != 1 {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                "Only one-dimensional arrays are supported",
                                Some(args[1].span),
                            ));
                        }
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
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            "Array access requires exactly one index",
                            Some(expr.span),
                        ));
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
                let left = self.resolve_default_value(left_value, expr.span)?;
                let right_value = self.eval_expr(right, frame)?;
                if matches!(right_value, Value::Missing) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "Missing optional argument cannot be used as a value",
                        Some(right.span),
                    ));
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
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                message,
                Some(expr.span),
            )),
        }
    }

    fn eval_builtin_function(
        &mut self,
        name: &str,
        args: &[Expr],
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<Option<Value>, Diagnostic> {
        if name.eq_ignore_ascii_case("IsObject") {
            self.expect_builtin_arg_count(name, args, 1, span)?;
            let value = self.eval_expr(&args[0], frame)?;
            return Ok(Some(Value::Boolean(matches!(
                value,
                Value::Object(_) | Value::Nothing
            ))));
        }
        if name.eq_ignore_ascii_case("IsArray") {
            self.expect_builtin_arg_count(name, args, 1, span)?;
            let value = self.eval_expr(&args[0], frame)?;
            return Ok(Some(Value::Boolean(matches!(value, Value::Array { .. }))));
        }
        if name.eq_ignore_ascii_case("IsNull") {
            self.expect_builtin_arg_count(name, args, 1, span)?;
            let value = self.eval_expr(&args[0], frame)?;
            return Ok(Some(Value::Boolean(matches!(value, Value::Null))));
        }
        if name.eq_ignore_ascii_case("IsError") {
            self.expect_builtin_arg_count(name, args, 1, span)?;
            return Ok(Some(Value::Boolean(false)));
        }
        if name.eq_ignore_ascii_case("VarType") {
            self.expect_builtin_arg_count(name, args, 1, span)?;
            let value = self.eval_expr(&args[0], frame)?;
            return Ok(Some(Value::Integer(vartype(&value))));
        }
        if name.eq_ignore_ascii_case("TypeName") {
            self.expect_builtin_arg_count(name, args, 1, span)?;
            let value = self.eval_expr(&args[0], frame)?;
            return Ok(Some(Value::String(value_type_name(&value))));
        }
        if name.eq_ignore_ascii_case("IIf") {
            self.expect_builtin_arg_count(name, args, 3, span)?;
            let condition = self.eval_expr(&args[0], frame)?.is_truthy();
            let value = if condition { &args[1] } else { &args[2] };
            return Ok(Some(self.eval_expr(value, frame)?));
        }
        if name.eq_ignore_ascii_case("CStr") {
            self.expect_builtin_arg_count(name, args, 1, span)?;
            let value = self.eval_expr(&args[0], frame)?;
            return Ok(Some(Value::String(value.to_output_string())));
        }
        if name.eq_ignore_ascii_case("StrComp") {
            if args.len() < 2 || args.len() > 3 {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "StrComp expects two strings and optional compare mode",
                    Some(span),
                ));
            }
            let left = self.eval_expr(&args[0], frame)?.to_output_string();
            let right = self.eval_expr(&args[1], frame)?.to_output_string();
            let text_compare = if args.len() == 3 {
                self.eval_integer_expr(&args[2], frame, "Compare mode must be Integer")? == 1
            } else {
                self.option_compare == crate::OptionCompare::Text
            };
            let (left, right) = if text_compare {
                (left.to_ascii_lowercase(), right.to_ascii_lowercase())
            } else {
                (left, right)
            };
            let result = match left.cmp(&right) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            return Ok(Some(Value::Integer(result)));
        }
        if name.eq_ignore_ascii_case("Sgn") {
            self.expect_builtin_arg_count(name, args, 1, span)?;
            let value = self.eval_integer_expr(&args[0], frame, "Sgn argument must be Integer")?;
            return Ok(Some(Value::Integer(value.signum())));
        }
        if name.eq_ignore_ascii_case("Int") {
            self.expect_builtin_arg_count(name, args, 1, span)?;
            let value = self.eval_integer_expr(&args[0], frame, "Int argument must be Integer")?;
            return Ok(Some(Value::Integer(value)));
        }
        Ok(None)
    }

    fn expect_builtin_arg_count(
        &self,
        name: &str,
        args: &[Expr],
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
}

fn vartype(value: &Value) -> i64 {
    match value {
        Value::Empty => 0,
        Value::Null => 1,
        Value::Integer(_) => 2,
        Value::String(_) => 8,
        Value::Object(_) | Value::Nothing => 9,
        Value::Boolean(_) => 11,
        Value::Array { .. } => 8192,
        Value::Record { .. } | Value::Missing => 12,
    }
}

fn value_type_name(value: &Value) -> String {
    match value {
        Value::Empty => "Empty".to_string(),
        Value::Null => "Null".to_string(),
        Value::Integer(_) => "Integer".to_string(),
        Value::String(_) => "String".to_string(),
        Value::Object(object) => object.borrow().class_name.clone(),
        Value::Nothing => "Nothing".to_string(),
        Value::Boolean(_) => "Boolean".to_string(),
        Value::Array { .. } => "Array".to_string(),
        Value::Record { type_name, .. } => type_name.clone(),
        Value::Missing => "Missing".to_string(),
    }
}
