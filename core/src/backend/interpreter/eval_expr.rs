use std::rc::Rc;

use crate::runtime::numeric::{unary_negate, unary_positive};
use crate::runtime::{Diagnostic, LambdaValue, Value};
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
            ExprKind::DateLiteral(value) => parse_date_literal(value, expr.span),
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
            ExprKind::Long(value) => Ok(Value::Int32(*value)),
            ExprKind::LongLong(value) => Ok(Value::Int64(*value)),
            ExprKind::Single(value) => Ok(Value::Single(*value)),
            ExprKind::Double(value) => Ok(Value::Double(*value)),
            ExprKind::Currency(value) => Ok(Value::Currency(*value)),
            ExprKind::Decimal(value) => Ok(Value::Decimal(*value)),
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
                        self.class_derives_from(&object_class, class_name)
                    }
                    Value::Nothing => false,
                    _ => false,
                };
                Ok(Value::Boolean(result))
            }
            ExprKind::Me | ExprKind::MyBase | ExprKind::MyClass => frame.get("me", expr.span),
            ExprKind::WithTarget => frame.current_with_target(expr.span),
            ExprKind::New {
                class_name,
                args,
                initializer,
            } => {
                let obj = self.new_object(class_name, args, frame, expr.span)?;
                if let Some(init_list) = initializer {
                    for init_expr in init_list {
                        let val = self.eval_expr(init_expr, frame)?;
                        match &obj {
                            Value::Collection(coll) => {
                                coll.borrow_mut().add(val, None, None, None).map_err(|e| {
                                    Diagnostic::new(
                                        crate::runtime::DiagnosticCode::ARRAY,
                                        e,
                                        Some(init_expr.span),
                                    )
                                })?;
                            }
                            _ => {
                                // In VB.NET, this would call the 'Add' method if available.
                                // For now, we only support Collection.
                                return Err(Diagnostic::new(
                                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                                    "Collection initializer is only supported for Collection types",
                                    Some(expr.span),
                                ));
                            }
                        }
                    }
                }
                Ok(obj)
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
                } else if name.eq_ignore_ascii_case("FreeFile") {
                    Ok(Value::Int64(i64::from(self.free_file_number())))
                } else if name.eq_ignore_ascii_case("Timer")
                    || name.eq_ignore_ascii_case("Now")
                    || name.eq_ignore_ascii_case("Date")
                    || name.eq_ignore_ascii_case("Time")
                    || name.eq_ignore_ascii_case("Rnd")
                {
                    match super::builtins::dispatch_function(self, name, &[], frame, expr.span)? {
                        Some(value) => Ok(value),
                        None => unreachable!("bare zero-argument builtin should dispatch"),
                    }
                } else if name.eq_ignore_ascii_case("VBA")
                    || name.eq_ignore_ascii_case("Console")
                    || name.eq_ignore_ascii_case("Err")
                {
                    Ok(Value::Empty)
                } else if let Some(constant) = crate::runtime::vba::vba_constant(name) {
                    Ok(constant.value())
                } else if let Some(value) = self.enum_members.get(&super::values::key(name)) {
                    Ok(Value::Int64(*value))
                } else {
                    match frame.get(name, expr.span) {
                        Ok(value) => Ok(value),
                        Err(error) => {
                            if let Ok(me) = frame.get("me", expr.span) {
                                if let Ok(value) = self.read_member(&me, name, frame, expr.span) {
                                    return Ok(value);
                                }
                                // Try shared members of the class me belongs to
                                if let Value::Object(ref obj) = me {
                                    let class_name = obj.borrow().class_name.clone();
                                    if let Ok(value) =
                                        self.read_shared_member(&class_name, name, frame, expr.span)
                                    {
                                        return Ok(value);
                                    }
                                }
                            } else if let Some(class_name) = frame.class_context() {
                                let class_name = class_name.to_string();
                                // Try shared members of the current class context (for shared methods)
                                if let Ok(value) =
                                    self.read_shared_member(&class_name, name, frame, expr.span)
                                {
                                    return Ok(value);
                                }
                            }
                            if self.functions.contains_key(&super::values::key(name))
                                || self.has_declared_function(name)
                            {
                                return self.call_function(name, &[], &[], frame, expr.span);
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
                    if name.eq_ignore_ascii_case("VBA") {
                        if let Some(constant) = crate::runtime::vba::vba_constant(field) {
                            return Ok(constant.value());
                        }
                        if field.eq_ignore_ascii_case("Timer")
                            || field.eq_ignore_ascii_case("Now")
                            || field.eq_ignore_ascii_case("Date")
                            || field.eq_ignore_ascii_case("Time")
                            || field.eq_ignore_ascii_case("Rnd")
                        {
                            match super::builtins::dispatch_function(
                                self,
                                field,
                                &[],
                                frame,
                                expr.span,
                            )? {
                                Some(value) => return Ok(value),
                                None => unreachable!("bare zero-argument builtin should dispatch"),
                            }
                        }
                    }
                    if name.eq_ignore_ascii_case("Console") {
                        // Console doesn't have fields in our current builtin model, but we handle it
                        // here to prevent it being treated as a potential module qualifier.
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
                                crate::runtime::DiagnosticCode::MEMBER_IS_PRIVATE,
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
                            crate::runtime::DiagnosticCode::MEMBER_IS_PRIVATE,
                            format!("Module member '{}.{}' is Private", module_name, field),
                            Some(expr.span),
                        ));
                    }
                    return Ok(value);
                }
                if let ExprKind::Variable(class_name) = &object.kind
                    && !frame.has_variable(class_name)
                    && let Ok(resolved) = self.resolve_user_type_name(class_name, frame, expr.span)
                    && self.classes.contains_key(&super::values::key(&resolved))
                {
                    return self.read_shared_member(&resolved, field, frame, expr.span);
                }
                let object = self.eval_expr(object, frame)?;
                self.read_member(&object, field, frame, expr.span)
            }
            ExprKind::Call {
                name,
                type_args,
                args,
            } => {
                if let Some(value) =
                    super::builtins::dispatch_function(self, name, args, frame, expr.span)?
                {
                    return Ok(value);
                }
                if frame.has_variable(name) {
                    let value = frame.get(name, expr.span)?;
                    match value {
                        Value::Array(_) => {
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
                        Value::Collection(_) => {
                            return self.call_method_function(
                                value.clone(),
                                "Item",
                                args,
                                frame,
                                expr.span,
                            );
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
                        Value::ComObject(ref com_obj) => {
                            let mut eval_args = Vec::with_capacity(args.len());
                            for arg in args {
                                eval_args.push(self.eval_expr(arg, frame)?);
                            }
                            return crate::runtime::com::invoke_default_com(
                                com_obj, &eval_args, 2, expr.span,
                            );
                        }
                        Value::Record(ref record) => {
                            if let Some(default_member) = self
                                .types
                                .get(&super::values::key(&record.type_name))
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
                        Value::Lambda(lambda) => {
                            let mut eval_args = Vec::with_capacity(args.len());
                            for arg in args {
                                eval_args.push(self.eval_expr(arg, frame)?);
                            }
                            return self.call_lambda_value(lambda, &eval_args, expr.span);
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
                if let Ok(me) = frame.get("me", expr.span) {
                    if let Ok(field_value) = self.read_member(&me, name, frame, expr.span)
                        && matches!(field_value, Value::Array(_))
                    {
                        let mut dims = Vec::new();
                        for arg in args {
                            dims.push(self.eval_integer_expr(
                                arg,
                                frame,
                                "Array index must be Integer",
                            )?);
                        }
                        return self
                            .read_bare_class_field_array_element(me, name, &dims, expr.span);
                    }

                    if let Value::Object(ref obj) = me {
                        let class_name = obj.borrow().class_name.clone();
                        if let Some(class) = self.classes.get(&super::values::key(&class_name))
                            && (class.functions.contains_key(&super::values::key(name))
                                || class.properties.contains_key(&super::values::key(name)))
                        {
                            return self.call_method_function(
                                me.clone(),
                                name,
                                args,
                                frame,
                                expr.span,
                            );
                        }
                    }
                }
                self.call_function(name, type_args, args, frame, expr.span)
            }
            ExprKind::MemberCall {
                object,
                method,
                type_args: _,
                args,
            } => {
                if let ExprKind::Variable(module_name) = &object.kind
                    && self
                        .resolve_module_qualifier(module_name, frame, expr.span)
                        .is_ok()
                {
                    return self.call_module_function(module_name, method, args, frame, expr.span);
                }
                if let ExprKind::Variable(class_name) = &object.kind
                    && !frame.has_variable(class_name)
                    && self.classes.contains_key(&super::values::key(class_name))
                {
                    return self.call_shared_function(class_name, method, args, frame, expr.span);
                }
                let object = self.eval_expr(object, frame)?;
                if let Ok(field_value) = self.read_member(&object, method, frame, expr.span)
                    && matches!(field_value, Value::Array(_))
                {
                    return self.eval_index_expr(field_value, args, frame, expr.span);
                }
                if matches!(object, Value::Record(_) | Value::BoxedRecord(_, _)) {
                    return self.call_record_function(object, method, args, frame, expr.span);
                }
                self.call_method_function(object, method, args, frame, expr.span)
            }
            ExprKind::Unary { op, expr: inner } => {
                let value = self.eval_expr(inner, frame)?;
                let operator_kind = match op {
                    crate::UnaryOp::Positive => Some(crate::OperatorKind::UnaryPlus),
                    crate::UnaryOp::Negate => Some(crate::OperatorKind::UnaryMinus),
                    crate::UnaryOp::LogicalNot => Some(crate::OperatorKind::Not),
                };
                if let Some(kind) = operator_kind
                    && let Some(res) =
                        self.call_overloaded_unary_operator(kind, value.clone(), expr.span)?
                {
                    return Ok(res);
                }
                match (op, value) {
                    (UnaryOp::Positive, value) => unary_positive(value, expr.span),
                    (UnaryOp::Negate, value) => unary_negate(value, expr.span),
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
            ExprKind::PassingModeOverride { expr: inner, .. } => self.eval_expr(inner, frame),
            ExprKind::Lambda { params, body } => Ok(Value::Lambda(Rc::new(LambdaValue {
                params: params.clone(),
                body: (**body).clone(),
            }))),
            ExprKind::Await(expr) => self.eval_expr(expr, frame),
            ExprKind::Binary { left, op, right } => {
                let left_value = self.eval_expr(left, frame)?;
                if matches!(left_value, Value::Missing) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "Optional argument was omitted here and cannot be used in this expression",
                        Some(left.span),
                    ));
                }

                if *op == crate::BinaryOp::LogicalAndAlso {
                    if !left_value.is_truthy() {
                        return Ok(Value::Boolean(false));
                    }
                } else if *op == crate::BinaryOp::LogicalOrElse && left_value.is_truthy() {
                    return Ok(Value::Boolean(true));
                }

                let right_value = self.eval_expr(right, frame)?;
                if matches!(right_value, Value::Missing) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "Optional argument was omitted here and cannot be used in this expression",
                        Some(right.span),
                    ));
                }

                // Check for overloaded operators
                let operator_kind = match op {
                    crate::BinaryOp::Add => Some(crate::OperatorKind::Add),
                    crate::BinaryOp::Subtract => Some(crate::OperatorKind::Subtract),
                    crate::BinaryOp::Multiply => Some(crate::OperatorKind::Multiply),
                    crate::BinaryOp::Divide => Some(crate::OperatorKind::Divide),
                    crate::BinaryOp::IntegerDivide => Some(crate::OperatorKind::IntegerDivide),
                    crate::BinaryOp::Exponent => Some(crate::OperatorKind::Exponent),
                    crate::BinaryOp::Modulo => Some(crate::OperatorKind::Modulo),
                    crate::BinaryOp::LogicalAnd => Some(crate::OperatorKind::And),
                    crate::BinaryOp::LogicalOr => Some(crate::OperatorKind::Or),
                    crate::BinaryOp::LogicalXor => Some(crate::OperatorKind::Xor),
                    crate::BinaryOp::Equal => Some(crate::OperatorKind::Equal),
                    crate::BinaryOp::NotEqual => Some(crate::OperatorKind::NotEqual),
                    crate::BinaryOp::Less => Some(crate::OperatorKind::Less),
                    crate::BinaryOp::Greater => Some(crate::OperatorKind::Greater),
                    crate::BinaryOp::LessEqual => Some(crate::OperatorKind::LessEqual),
                    crate::BinaryOp::GreaterEqual => Some(crate::OperatorKind::GreaterEqual),
                    crate::BinaryOp::Like => Some(crate::OperatorKind::Like),
                    crate::BinaryOp::Concat => Some(crate::OperatorKind::Concatenate),
                    _ => None,
                };

                if let Some(kind) = operator_kind
                    && let Some(value) = self.call_overloaded_binary_operator(
                        left_value.clone(),
                        kind,
                        right_value.clone(),
                        expr.span,
                    )?
                {
                    return Ok(value);
                }

                let left = self.resolve_default_value(left_value, frame, expr.span)?;
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
                    crate::BinaryOp::LogicalAnd | crate::BinaryOp::LogicalAndAlso => {
                        RuntimeBinaryOp::LogicalAnd
                    }
                    crate::BinaryOp::LogicalOr | crate::BinaryOp::LogicalOrElse => {
                        RuntimeBinaryOp::LogicalOr
                    }
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
                    crate::BinaryOp::IsNot => RuntimeBinaryOp::IsNot,
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
            Value::Array(_) => {
                let mut dims = Vec::new();
                for arg in args {
                    dims.push(self.eval_integer_expr(arg, frame, "Array index must be Integer")?);
                }
                super::arrays::read_array_element(&target, &dims, span)
            }
            Value::Collection(ref collection) => {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "Collection indexing requires exactly one argument",
                        Some(span),
                    ));
                }
                let index_or_key = self.eval_expr(&args[0], frame)?;
                collection.borrow().item(&index_or_key).map_err(|e| {
                    Diagnostic::new(crate::runtime::DiagnosticCode::ARRAY, e, Some(span))
                })
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
            Value::Record(ref record) => {
                if let Some(default_member) = self
                    .types
                    .get(&super::values::key(&record.type_name))
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
                    format!("Structure '{}' has no default property", record.type_name),
                    Some(span),
                ))
            }
            Value::Lambda(lambda) => {
                let mut eval_args = Vec::with_capacity(args.len());
                for arg in args {
                    eval_args.push(self.eval_expr(arg, frame)?);
                }
                self.call_lambda_value(lambda, &eval_args, span)
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
        if let Value::Record(record) = &value {
            if let Some(type_def) = self.types.get(&super::values::key(&record.type_name))
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

fn parse_date_literal(value: &str, span: crate::runtime::Span) -> Result<Value, Diagnostic> {
    let parts = value.split('/').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Date literal must use m/d/yyyy syntax",
            Some(span),
        ));
    }
    let month = parts[0].parse::<i64>().map_err(|_| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Date literal month must be numeric",
            Some(span),
        )
    })?;
    let day = parts[1].parse::<i64>().map_err(|_| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Date literal day must be numeric",
            Some(span),
        )
    })?;
    let year = parts[2].parse::<i64>().map_err(|_| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Date literal year must be numeric",
            Some(span),
        )
    })?;
    Ok(Value::Date(date_serial(year, month, day)))
}

fn date_serial(year: i64, month: i64, day: i64) -> f64 {
    const UNIX_EPOCH_AS_VBA: i64 = 25_569;
    let month_index = month - 1;
    let normalized_year = year + month_index.div_euclid(12);
    let normalized_month = month_index.rem_euclid(12) + 1;
    let days = days_from_civil(normalized_year, normalized_month as u32, 1) + day - 1;
    (days + UNIX_EPOCH_AS_VBA) as f64
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let month = month as i64;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
