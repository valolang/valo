use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::{Diagnostic, ObjectValue, Span, TypeName, Value};
use crate::{
    BinaryOp, ClassMember, ClassProperty, Expr, ExprKind, Function, PassingMode, Procedure,
    Program, PropertyKind, Stmt, TypeDecl, UnaryOp,
};

#[derive(Debug, Default)]
pub struct Interpreter {
    types: HashMap<String, RuntimeType>,
    classes: HashMap<String, RuntimeClass>,
    procedures: HashMap<String, Procedure>,
    functions: HashMap<String, Function>,
    output: Vec<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(mut self, program: &Program) -> Result<Vec<String>, Diagnostic> {
        for type_decl in &program.types {
            self.types
                .insert(key(&type_decl.name), RuntimeType::from(type_decl));
        }
        for class_decl in &program.classes {
            self.classes
                .insert(key(&class_decl.name), RuntimeClass::from(class_decl));
        }
        for procedure in &program.procedures {
            self.procedures
                .insert(key(&procedure.name), procedure.clone());
        }
        for function in &program.functions {
            self.functions.insert(key(&function.name), function.clone());
        }

        let Some(main) = program
            .procedures
            .iter()
            .find(|procedure| procedure.name.eq_ignore_ascii_case("main"))
        else {
            return Err(Diagnostic::new("Program must contain Sub Main()", None));
        };

        let mut frame = Frame::default();
        match self.exec_block(&main.body, &mut frame)? {
            ControlFlow::Continue => Ok(self.output),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                "Return is only allowed inside Function",
                Some(main.span),
            )),
        }
    }

    fn exec_block(
        &mut self,
        statements: &[Stmt],
        frame: &mut Frame,
    ) -> Result<ControlFlow, Diagnostic> {
        for stmt in statements {
            match self.exec_stmt(stmt, frame)? {
                ControlFlow::Continue => {}
                flow @ ControlFlow::Return(_) => return Ok(flow),
            }
        }
        Ok(ControlFlow::Continue)
    }

    fn exec_stmt(&mut self, stmt: &Stmt, frame: &mut Frame) -> Result<ControlFlow, Diagnostic> {
        match stmt {
            Stmt::Dim {
                name,
                ty,
                array_size,
                span,
            } => {
                frame.declare(name, ty.clone(), *array_size, *span, &self.types)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::Assign { name, expr, span } => {
                let value = self.eval_expr(expr, frame)?;
                frame.assign(name, value, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::SetAssign { name, expr, span } => {
                let value = self.eval_expr(expr, frame)?;
                frame.assign(name, value, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::ArrayAssign {
                name,
                index,
                expr,
                span,
            } => {
                let index = self.eval_integer_expr(index, frame, "Array index must be Integer")?;
                let value = self.eval_expr(expr, frame)?;
                frame.assign_array_element(name, index, value, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::MemberAssign {
                target,
                field,
                expr,
                span,
            } => {
                let value = self.eval_expr(expr, frame)?;
                self.assign_member(target, field, value, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::ConsoleWriteLine { args, .. } => {
                let mut parts = Vec::new();
                for arg in args {
                    parts.push(self.eval_expr(arg, frame)?.to_output_string());
                }
                self.output.push(parts.join(" "));
                Ok(ControlFlow::Continue)
            }
            Stmt::SubCall { name, args, span } => {
                self.call_sub(name, args, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::MemberSubCall {
                object,
                method,
                args,
                span,
            } => {
                let object = self.eval_expr(object, frame)?;
                self.call_method_sub(object, method, args, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::Return { expr, .. } => {
                let value = self.eval_expr(expr, frame)?;
                Ok(ControlFlow::Return(value))
            }
            Stmt::If {
                condition,
                then_body,
                elseif_branches,
                else_body,
                ..
            } => {
                if self.eval_expr(condition, frame)?.is_truthy() {
                    self.exec_block(then_body, frame)
                } else {
                    for branch in elseif_branches {
                        if self.eval_expr(&branch.condition, frame)?.is_truthy() {
                            return self.exec_block(&branch.body, frame);
                        }
                    }
                    self.exec_block(else_body, frame)
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                while self.eval_expr(condition, frame)?.is_truthy() {
                    match self.exec_block(body, frame)? {
                        ControlFlow::Continue => {}
                        flow @ ControlFlow::Return(_) => return Ok(flow),
                    }
                }
                Ok(ControlFlow::Continue)
            }
            Stmt::For {
                variable,
                start,
                end,
                step,
                body,
                span,
            } => {
                let mut current =
                    self.eval_integer_expr(start, frame, "For start value must be Integer")?;
                let end = self.eval_integer_expr(end, frame, "For end value must be Integer")?;
                let step = match step {
                    Some(step) => {
                        self.eval_integer_expr(step, frame, "For step value must be Integer")?
                    }
                    None => 1,
                };

                if step == 0 {
                    return Err(Diagnostic::new("For Step cannot be zero", Some(*span)));
                }

                loop {
                    if (step > 0 && current > end) || (step < 0 && current < end) {
                        break;
                    }

                    frame.assign(variable, Value::Integer(current), *span)?;
                    match self.exec_block(body, frame)? {
                        ControlFlow::Continue => {}
                        flow @ ControlFlow::Return(_) => return Ok(flow),
                    }
                    current += step;
                }

                Ok(ControlFlow::Continue)
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expr, frame: &mut Frame) -> Result<Value, Diagnostic> {
        match &expr.kind {
            ExprKind::String(value) => Ok(Value::String(value.clone())),
            ExprKind::Integer(value) => Ok(Value::Integer(*value)),
            ExprKind::Boolean(value) => Ok(Value::Boolean(*value)),
            ExprKind::Nothing => Ok(Value::Nothing),
            ExprKind::Me => frame.get("me", expr.span),
            ExprKind::New { class_name, args } => {
                self.new_object(class_name, args, frame, expr.span)
            }
            ExprKind::Variable(name) => frame.get(name, expr.span),
            ExprKind::MemberAccess { object, field } => {
                let object = self.eval_expr(object, frame)?;
                self.read_member(&object, field, frame, expr.span)
            }
            ExprKind::Call { name, args } => {
                if frame.has_variable(name) {
                    if args.len() != 1 {
                        return Err(Diagnostic::new(
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
                let object = self.eval_expr(object, frame)?;
                self.call_method_function(object, method, args, frame, expr.span)
            }
            ExprKind::Unary { op, expr: inner } => {
                let value = self.eval_expr(inner, frame)?;
                match (op, value) {
                    (UnaryOp::Negate, Value::Integer(value)) => Ok(Value::Integer(-value)),
                    (UnaryOp::Negate, _) => Err(Diagnostic::new(
                        "Unary '-' requires an Integer expression",
                        Some(expr.span),
                    )),
                    (UnaryOp::LogicalNot, Value::Boolean(value)) => Ok(Value::Boolean(!value)),
                    (UnaryOp::LogicalNot, _) => Err(Diagnostic::new(
                        "Not requires a Boolean expression",
                        Some(expr.span),
                    )),
                }
            }
            ExprKind::Binary { left, op, right } => {
                let left = self.eval_expr(left, frame)?;
                let right = self.eval_expr(right, frame)?;
                eval_binary(left, *op, right, expr.span)
            }
        }
    }

    fn eval_integer_expr(
        &mut self,
        expr: &Expr,
        frame: &mut Frame,
        message: &str,
    ) -> Result<i64, Diagnostic> {
        match self.eval_expr(expr, frame)? {
            Value::Integer(value) => Ok(value),
            _ => Err(Diagnostic::new(message, Some(expr.span))),
        }
    }

    fn call_function(
        &mut self,
        name: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let function = self.functions.get(&key(name)).cloned().ok_or_else(|| {
            Diagnostic::new(format!("Function '{}' is not defined", name), Some(span))
        })?;

        if args.len() != function.params.len() {
            return Err(Diagnostic::new(
                format!(
                    "Function '{}' expects {} argument(s), got {}",
                    function.name,
                    function.params.len(),
                    args.len()
                ),
                Some(span),
            ));
        }

        let mut frame = Frame::default();
        for (param, arg) in function.params.iter().zip(args) {
            match param.mode {
                PassingMode::ByVal => {
                    let value = self.eval_expr(arg, caller_frame)?;
                    frame.declare(&param.name, param.ty.clone(), None, param.span, &self.types)?;
                    frame.assign(&param.name, value, param.span)?;
                }
                PassingMode::ByRef => {
                    let ExprKind::Variable(arg_name) = &arg.kind else {
                        return Err(Diagnostic::new(
                            "ByRef argument must be a variable",
                            Some(arg.span),
                        ));
                    };
                    let variable = caller_frame.variable(arg_name, arg.span)?;
                    frame.declare_alias(&param.name, param.ty.clone(), variable, param.span)?;
                }
            }
        }

        match self.exec_block(&function.body, &mut frame)? {
            ControlFlow::Return(value) => coerce_assignment(&function.return_type, value, span),
            ControlFlow::Continue => Err(Diagnostic::new(
                format!("Function '{}' must return a value", function.name),
                Some(function.span),
            )),
        }
    }

    fn call_sub(
        &mut self,
        name: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let procedure =
            self.procedures.get(&key(name)).cloned().ok_or_else(|| {
                Diagnostic::new(format!("Sub '{}' is not defined", name), Some(span))
            })?;

        if args.len() != procedure.params.len() {
            return Err(Diagnostic::new(
                format!(
                    "Sub '{}' expects {} argument(s), got {}",
                    procedure.name,
                    procedure.params.len(),
                    args.len()
                ),
                Some(span),
            ));
        }

        let mut frame = Frame::default();
        for (param, arg) in procedure.params.iter().zip(args) {
            match param.mode {
                PassingMode::ByVal => {
                    let value = self.eval_expr(arg, caller_frame)?;
                    frame.declare(&param.name, param.ty.clone(), None, param.span, &self.types)?;
                    frame.assign(&param.name, value, param.span)?;
                }
                PassingMode::ByRef => {
                    let ExprKind::Variable(arg_name) = &arg.kind else {
                        return Err(Diagnostic::new(
                            "ByRef argument must be a variable",
                            Some(arg.span),
                        ));
                    };
                    let variable = caller_frame.variable(arg_name, arg.span)?;
                    frame.declare_alias(&param.name, param.ty.clone(), variable, param.span)?;
                }
            }
        }

        match self.exec_block(&procedure.body, &mut frame)? {
            ControlFlow::Continue => Ok(()),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                "Return is only allowed inside Function",
                Some(procedure.span),
            )),
        }
    }

    fn new_object(
        &mut self,
        class_name: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let class = self.classes.get(&key(class_name)).cloned().ok_or_else(|| {
            Diagnostic::new(format!("Class '{}' is not defined", class_name), Some(span))
        })?;
        let mut fields = HashMap::new();
        for field in &class.fields {
            fields.insert(
                key(&field.name),
                default_value(&field.ty, &self.types, span)?,
            );
        }
        let object = Value::Object(Rc::new(RefCell::new(ObjectValue {
            class_name: class.name.clone(),
            fields,
        })));
        if let Some(init) = class.subs.get("initialize") {
            self.call_method_sub(object.clone(), &init.name, args, caller_frame, span)?;
        } else if !args.is_empty() {
            return Err(Diagnostic::new(
                format!("Class '{}' has no Initialize constructor", class.name),
                Some(span),
            ));
        }
        Ok(object)
    }

    fn call_method_sub(
        &mut self,
        object: Value,
        method: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let instance = ensure_object(object, span)?;
        let class_name = instance.borrow().class_name.clone();
        let class = self
            .classes
            .get(&key(&class_name))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(format!("Class '{}' is not defined", class_name), Some(span))
            })?;
        let procedure = class.subs.get(&key(method)).cloned().ok_or_else(|| {
            Diagnostic::new(
                format!("Class '{}' has no method '{}'", class.name, method),
                Some(span),
            )
        })?;
        let mut frame = Frame::default();
        frame.declare_object_alias("me", &class.name, instance, span)?;
        self.bind_parameters(&procedure.params, args, caller_frame, &mut frame)?;
        match self.exec_block(&procedure.body, &mut frame)? {
            ControlFlow::Continue => Ok(()),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                "Return is only allowed inside Function",
                Some(procedure.span),
            )),
        }
    }

    fn call_method_function(
        &mut self,
        object: Value,
        method: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let instance = ensure_object(object, span)?;
        let class_name = instance.borrow().class_name.clone();
        let class = self
            .classes
            .get(&key(&class_name))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(format!("Class '{}' is not defined", class_name), Some(span))
            })?;
        let function = class.functions.get(&key(method)).cloned().ok_or_else(|| {
            Diagnostic::new(
                format!("Class '{}' has no method '{}'", class.name, method),
                Some(span),
            )
        })?;
        let mut frame = Frame::default();
        frame.declare_object_alias("me", &class.name, instance, span)?;
        self.bind_parameters(&function.params, args, caller_frame, &mut frame)?;
        match self.exec_block(&function.body, &mut frame)? {
            ControlFlow::Return(value) => coerce_assignment(&function.return_type, value, span),
            ControlFlow::Continue => Err(Diagnostic::new(
                format!("Function '{}' must return a value", function.name),
                Some(function.span),
            )),
        }
    }

    fn call_property_get(
        &mut self,
        object: Value,
        property: &str,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let instance = ensure_object(object, span)?;
        let class_name = instance.borrow().class_name.clone();
        let class = self
            .classes
            .get(&key(&class_name))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(format!("Class '{}' is not defined", class_name), Some(span))
            })?;
        let accessor = class
            .properties
            .get(&key(property))
            .and_then(|property| property.get.clone())
            .ok_or_else(|| {
                Diagnostic::new(
                    format!("Property '{}' has no Get accessor", property),
                    Some(span),
                )
            })?;
        let mut frame = Frame::default();
        frame.declare_object_alias("me", &class.name, instance, span)?;
        match self.exec_block(&accessor.body, &mut frame)? {
            ControlFlow::Return(value) => coerce_assignment(
                accessor.return_type.as_ref().expect("get return type"),
                value,
                span,
            ),
            ControlFlow::Continue => Err(Diagnostic::new(
                format!("Property Get '{}' must return a value", accessor.name),
                Some(accessor.span),
            )),
        }
    }

    fn call_property_set(
        &mut self,
        object: Value,
        property: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let instance = ensure_object(object, span)?;
        let class_name = instance.borrow().class_name.clone();
        let class = self
            .classes
            .get(&key(&class_name))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(format!("Class '{}' is not defined", class_name), Some(span))
            })?;
        let property_sig = class.properties.get(&key(property)).ok_or_else(|| {
            Diagnostic::new(
                format!(
                    "Class '{}' has no field or property '{}'",
                    class.name, property
                ),
                Some(span),
            )
        })?;
        let accessor = if matches!(value, Value::Object(_) | Value::Nothing) {
            property_sig.set.as_ref().or(property_sig.let_.as_ref())
        } else {
            property_sig.let_.as_ref()
        }
        .cloned()
        .ok_or_else(|| {
            Diagnostic::new(
                format!("Property '{}' has no Let or Set accessor", property),
                Some(span),
            )
        })?;
        let Some(param) = accessor.params.first() else {
            return Err(Diagnostic::new(
                format!(
                    "Property {:?} '{}' expects one parameter",
                    accessor.kind, property
                ),
                Some(accessor.span),
            ));
        };
        let mut frame = Frame::default();
        frame.declare_object_alias("me", &class.name, instance, span)?;
        frame.declare(&param.name, param.ty.clone(), None, param.span, &self.types)?;
        frame.assign(&param.name, value, span)?;
        match self.exec_block(&accessor.body, &mut frame)? {
            ControlFlow::Continue => Ok(()),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                "Return is only allowed inside Function or Property Get",
                Some(accessor.span),
            )),
        }
    }

    fn read_member(
        &mut self,
        value: &Value,
        member: &str,
        _frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if object_has_field(value, member) {
            return read_field_member(value, member, span);
        }
        if matches!(value, Value::Nothing) {
            return Err(Diagnostic::new("Object reference is Nothing", Some(span)));
        }
        if matches!(value, Value::Object(_)) {
            return self.call_property_get(value.clone(), member, span);
        }
        read_field_member(value, member, span)
    }

    fn assign_member(
        &mut self,
        target: &Expr,
        member: &str,
        value: Value,
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match &target.kind {
            ExprKind::Variable(name) => {
                let variable = frame.variable(name, target.span)?;
                self.assign_member_to_variable(variable, member, value, span)
            }
            ExprKind::Me => {
                let variable = frame.variable("me", target.span)?;
                self.assign_member_to_variable(variable, member, value, span)
            }
            ExprKind::Call { name, args } => {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "Array access requires exactly one index",
                        Some(target.span),
                    ));
                }
                let index = frame.simple_index_value(&args[0], span)?;
                let variable = frame.variable(name, target.span)?;
                let mut root = variable.cell.borrow_mut();
                let element = array_element_mut(&mut root, index, span)?;
                if object_has_field(element, member) || !matches!(element, Value::Object(_)) {
                    return write_member(element, member, value, span);
                }
                let object = element.clone();
                drop(root);
                self.call_property_set(object, member, value, span)
            }
            _ => Err(Diagnostic::new(
                "Member assignment target must be a variable or Me",
                Some(target.span),
            )),
        }
    }

    fn assign_member_to_variable(
        &mut self,
        variable: Variable,
        member: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let mut root = variable.cell.borrow_mut();
        if object_has_field(&root, member) || !matches!(&*root, Value::Object(_)) {
            return write_member(&mut root, member, value, span);
        }
        let object = root.clone();
        drop(root);
        self.call_property_set(object, member, value, span)
    }

    fn bind_parameters(
        &mut self,
        params: &[crate::Parameter],
        args: &[Expr],
        caller_frame: &mut Frame,
        callee_frame: &mut Frame,
    ) -> Result<(), Diagnostic> {
        if args.len() != params.len() {
            return Err(Diagnostic::new(
                format!("Expected {} argument(s), got {}", params.len(), args.len()),
                args.first().map(|arg| arg.span),
            ));
        }
        for (param, arg) in params.iter().zip(args) {
            match param.mode {
                PassingMode::ByVal => {
                    let value = self.eval_expr(arg, caller_frame)?;
                    callee_frame.declare(
                        &param.name,
                        param.ty.clone(),
                        None,
                        param.span,
                        &self.types,
                    )?;
                    callee_frame.assign(&param.name, value, param.span)?;
                }
                PassingMode::ByRef => {
                    let ExprKind::Variable(arg_name) = &arg.kind else {
                        return Err(Diagnostic::new(
                            "ByRef argument must be a variable",
                            Some(arg.span),
                        ));
                    };
                    let variable = caller_frame.variable(arg_name, arg.span)?;
                    callee_frame.declare_alias(
                        &param.name,
                        param.ty.clone(),
                        variable,
                        param.span,
                    )?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct Frame {
    variables: HashMap<String, Variable>,
}

impl Frame {
    fn declare(
        &mut self,
        name: &str,
        ty: TypeName,
        array_size: Option<usize>,
        span: Span,
        types: &HashMap<String, RuntimeType>,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if self.variables.contains_key(&key) {
            return Err(Diagnostic::new(
                format!("Variable '{}' is already declared", name),
                Some(span),
            ));
        }

        let value = if let Some(size) = array_size {
            let mut elements = Vec::new();
            for _ in 0..=size {
                elements.push(default_value(&ty, types, span)?);
            }
            Value::Array {
                element_type: ty.clone(),
                elements,
            }
        } else {
            default_value(&ty, types, span)?
        };

        self.variables.insert(
            key,
            Variable {
                cell: Rc::new(RefCell::new(value)),
                ty,
            },
        );
        Ok(())
    }

    fn declare_alias(
        &mut self,
        name: &str,
        ty: TypeName,
        variable: Variable,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if self.variables.contains_key(&key) {
            return Err(Diagnostic::new(
                format!("Variable '{}' is already declared", name),
                Some(span),
            ));
        }
        if !variable.ty.same_type(&ty) {
            return Err(Diagnostic::new(
                format!(
                    "ByRef argument type {} must match parameter type {}",
                    variable.ty.display_name(),
                    ty.display_name()
                ),
                Some(span),
            ));
        }

        self.variables.insert(key, variable);
        Ok(())
    }

    fn declare_object_alias(
        &mut self,
        name: &str,
        class_name: &str,
        object: Rc<RefCell<ObjectValue>>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if self.variables.contains_key(&key) {
            return Err(Diagnostic::new(
                format!("Variable '{}' is already declared", name),
                Some(span),
            ));
        }
        self.variables.insert(
            key,
            Variable {
                ty: TypeName::User(class_name.to_string()),
                cell: Rc::new(RefCell::new(Value::Object(object))),
            },
        );
        Ok(())
    }

    fn assign(&mut self, name: &str, value: Value, span: Span) -> Result<(), Diagnostic> {
        let variable = self.variables.get_mut(&key(name)).ok_or_else(|| {
            Diagnostic::new(format!("Variable '{}' is not declared", name), Some(span))
        })?;

        *variable.cell.borrow_mut() = coerce_assignment(&variable.ty, value, span)?;
        Ok(())
    }

    fn get(&self, name: &str, span: Span) -> Result<Value, Diagnostic> {
        self.variables
            .get(&key(name))
            .map(|variable| variable.cell.borrow().clone())
            .ok_or_else(|| {
                Diagnostic::new(format!("Variable '{}' is not declared", name), Some(span))
            })
    }

    fn variable(&self, name: &str, span: Span) -> Result<Variable, Diagnostic> {
        self.variables.get(&key(name)).cloned().ok_or_else(|| {
            Diagnostic::new(format!("Variable '{}' is not declared", name), Some(span))
        })
    }

    fn has_variable(&self, name: &str) -> bool {
        self.variables.contains_key(&key(name))
    }

    fn get_array_element(&self, name: &str, index: i64, span: Span) -> Result<Value, Diagnostic> {
        let variable = self.variable(name, span)?;
        let array = variable.cell.borrow();
        read_array_element(&array, index, span)
    }

    fn assign_array_element(
        &mut self,
        name: &str,
        index: i64,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let variable = self.variable(name, span)?;
        let mut array = variable.cell.borrow_mut();
        write_array_element(&mut array, index, value, span)
    }

    fn simple_index_value(&self, expr: &Expr, span: Span) -> Result<i64, Diagnostic> {
        match &expr.kind {
            ExprKind::Integer(value) => Ok(*value),
            ExprKind::Variable(name) => match self.get(name, expr.span)? {
                Value::Integer(value) => Ok(value),
                _ => Err(Diagnostic::new("Array index must be Integer", Some(span))),
            },
            _ => Err(Diagnostic::new(
                "Array member assignment index must be an Integer literal or variable",
                Some(span),
            )),
        }
    }
}

#[derive(Debug, Clone)]
struct Variable {
    ty: TypeName,
    cell: Rc<RefCell<Value>>,
}

#[derive(Debug, Clone)]
enum ControlFlow {
    Continue,
    Return(Value),
}

fn eval_binary(left: Value, op: BinaryOp, right: Value, span: Span) -> Result<Value, Diagnostic> {
    match op {
        BinaryOp::Add => integer_binary(left, right, span, |a, b| a + b),
        BinaryOp::Subtract => integer_binary(left, right, span, |a, b| a - b),
        BinaryOp::Multiply => integer_binary(left, right, span, |a, b| a * b),
        BinaryOp::Divide => {
            let (a, b) = expect_integers(left, right, span)?;
            if b == 0 {
                return Err(Diagnostic::new("Division by zero", Some(span)));
            }
            Ok(Value::Integer(a / b))
        }
        BinaryOp::Modulo => {
            let (a, b) = expect_integers(left, right, span)?;
            if b == 0 {
                return Err(Diagnostic::new("Modulo by zero", Some(span)));
            }
            Ok(Value::Integer(a % b))
        }
        BinaryOp::Concat => Ok(Value::String(format!(
            "{}{}",
            left.to_output_string(),
            right.to_output_string()
        ))),
        BinaryOp::LogicalAnd => boolean_binary(left, right, span, |a, b| a && b),
        BinaryOp::LogicalOr => boolean_binary(left, right, span, |a, b| a || b),
        BinaryOp::Equal => Ok(Value::Boolean(values_equal(&left, &right))),
        BinaryOp::NotEqual => Ok(Value::Boolean(!values_equal(&left, &right))),
        BinaryOp::Is => Ok(Value::Boolean(values_identical(&left, &right))),
        BinaryOp::Less => compare_values(left, right, span, |ord| ord.is_lt()),
        BinaryOp::Greater => compare_values(left, right, span, |ord| ord.is_gt()),
        BinaryOp::LessEqual => compare_values(left, right, span, |ord| ord.is_le()),
        BinaryOp::GreaterEqual => compare_values(left, right, span, |ord| ord.is_ge()),
    }
}

fn integer_binary(
    left: Value,
    right: Value,
    span: Span,
    op: impl FnOnce(i64, i64) -> i64,
) -> Result<Value, Diagnostic> {
    let (a, b) = expect_integers(left, right, span)?;
    Ok(Value::Integer(op(a, b)))
}

fn expect_integers(left: Value, right: Value, span: Span) -> Result<(i64, i64), Diagnostic> {
    match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => Ok((a, b)),
        _ => Err(Diagnostic::new(
            "Arithmetic operators require Integer operands",
            Some(span),
        )),
    }
}

fn boolean_binary(
    left: Value,
    right: Value,
    span: Span,
    op: impl FnOnce(bool, bool) -> bool,
) -> Result<Value, Diagnostic> {
    match (left, right) {
        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(op(a, b))),
        _ => Err(Diagnostic::new(
            "Logical operators require Boolean operands",
            Some(span),
        )),
    }
}

fn compare_values(
    left: Value,
    right: Value,
    span: Span,
    predicate: impl FnOnce(std::cmp::Ordering) -> bool,
) -> Result<Value, Diagnostic> {
    let ordering = match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => a.cmp(&b),
        (Value::String(a), Value::String(b)) => a.cmp(&b),
        _ => {
            return Err(Diagnostic::new(
                "Comparison requires matching Integer or String operands",
                Some(span),
            ));
        }
    };

    Ok(Value::Boolean(predicate(ordering)))
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Integer(a), Value::Integer(b)) => a == b,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Empty, Value::Empty) => true,
        _ => false,
    }
}

fn values_identical(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Nothing, Value::Nothing) => true,
        (Value::Object(left), Value::Object(right)) => Rc::ptr_eq(left, right),
        _ => false,
    }
}

fn default_value(
    ty: &TypeName,
    types: &HashMap<String, RuntimeType>,
    span: Span,
) -> Result<Value, Diagnostic> {
    if let Some(value) = ty.builtin_default_value() {
        return Ok(value);
    }

    let TypeName::User(name) = ty else {
        unreachable!("builtin types are handled above");
    };
    let type_def = types
        .get(&key(name))
        .ok_or_else(|| Diagnostic::new(format!("Type '{}' is not defined", name), Some(span)));
    let Ok(type_def) = type_def else {
        return Ok(Value::Nothing);
    };

    let mut fields = HashMap::new();
    for field in &type_def.fields {
        fields.insert(key(&field.name), default_value(&field.ty, types, span)?);
    }

    Ok(Value::Record {
        type_name: type_def.name.clone(),
        fields,
    })
}

fn object_has_field(value: &Value, field: &str) -> bool {
    if let Value::Object(object) = value {
        return object.borrow().fields.contains_key(&key(field));
    }
    false
}

fn ensure_object(value: Value, span: Span) -> Result<Rc<RefCell<ObjectValue>>, Diagnostic> {
    match value {
        Value::Object(object) => Ok(object),
        Value::Nothing => Err(Diagnostic::new("Object reference is Nothing", Some(span))),
        _ => Err(Diagnostic::new(
            "Method call requires an object",
            Some(span),
        )),
    }
}

fn read_field_member(value: &Value, field: &str, span: Span) -> Result<Value, Diagnostic> {
    if let Value::Object(object) = value {
        let object = object.borrow();
        return object.fields.get(&key(field)).cloned().ok_or_else(|| {
            Diagnostic::new(
                format!("Class '{}' has no field '{}'", object.class_name, field),
                Some(span),
            )
        });
    }
    if matches!(value, Value::Nothing) {
        return Err(Diagnostic::new("Object reference is Nothing", Some(span)));
    }
    let Value::Record { type_name, fields } = value else {
        return Err(Diagnostic::new(
            "Member access requires a user-defined Type value",
            Some(span),
        ));
    };

    fields.get(&key(field)).cloned().ok_or_else(|| {
        Diagnostic::new(
            format!("Type '{}' has no field '{}'", type_name, field),
            Some(span),
        )
    })
}

fn read_array_element(value: &Value, index: i64, span: Span) -> Result<Value, Diagnostic> {
    let Value::Array { elements, .. } = value else {
        return Err(Diagnostic::new("Value is not an array", Some(span)));
    };
    let index = checked_index(index, elements.len(), span)?;
    Ok(elements[index].clone())
}

fn write_array_element(
    value: &mut Value,
    index: i64,
    new_value: Value,
    span: Span,
) -> Result<(), Diagnostic> {
    let Value::Array {
        element_type,
        elements,
    } = value
    else {
        return Err(Diagnostic::new("Value is not an array", Some(span)));
    };
    let index = checked_index(index, elements.len(), span)?;
    elements[index] = coerce_assignment(element_type, new_value, span)?;
    Ok(())
}

fn array_element_mut(value: &mut Value, index: i64, span: Span) -> Result<&mut Value, Diagnostic> {
    let Value::Array { elements, .. } = value else {
        return Err(Diagnostic::new("Value is not an array", Some(span)));
    };
    let index = checked_index(index, elements.len(), span)?;
    Ok(&mut elements[index])
}

fn checked_index(index: i64, len: usize, span: Span) -> Result<usize, Diagnostic> {
    if index < 0 || index as usize >= len {
        return Err(Diagnostic::new(
            format!("Array index {} is out of bounds for length {}", index, len),
            Some(span),
        ));
    }
    Ok(index as usize)
}

fn write_member(
    value: &mut Value,
    field: &str,
    new_value: Value,
    span: Span,
) -> Result<(), Diagnostic> {
    if let Value::Object(object) = value {
        let mut object = object.borrow_mut();
        let Some(slot) = object.fields.get_mut(&key(field)) else {
            return Err(Diagnostic::new(
                format!("Class '{}' has no field '{}'", object.class_name, field),
                Some(span),
            ));
        };
        let ty = slot.type_name();
        *slot = coerce_assignment(&ty, new_value, span)?;
        return Ok(());
    }
    if matches!(value, Value::Nothing) {
        return Err(Diagnostic::new("Object reference is Nothing", Some(span)));
    }
    let Value::Record { type_name, fields } = value else {
        return Err(Diagnostic::new(
            "Member assignment requires a user-defined Type value",
            Some(span),
        ));
    };

    let Some(slot) = fields.get_mut(&key(field)) else {
        return Err(Diagnostic::new(
            format!("Type '{}' has no field '{}'", type_name, field),
            Some(span),
        ));
    };

    let ty = slot.type_name();
    *slot = coerce_assignment(&ty, new_value, span)?;
    Ok(())
}

fn coerce_assignment(ty: &TypeName, value: Value, span: Span) -> Result<Value, Diagnostic> {
    if matches!(value, Value::Nothing) && matches!(ty, TypeName::User(_)) {
        return Ok(value);
    }
    if ty.same_type(&TypeName::Variant) || ty.same_type(&value.type_name()) {
        Ok(value)
    } else {
        Err(Diagnostic::new(
            format!(
                "Cannot assign {} value to {} variable",
                value.type_name().display_name(),
                ty.display_name()
            ),
            Some(span),
        ))
    }
}

fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}

#[derive(Debug, Clone)]
struct RuntimeType {
    name: String,
    fields: Vec<RuntimeField>,
}

impl From<&TypeDecl> for RuntimeType {
    fn from(value: &TypeDecl) -> Self {
        Self {
            name: value.name.clone(),
            fields: value
                .fields
                .iter()
                .map(|field| RuntimeField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
struct RuntimeField {
    name: String,
    ty: TypeName,
}

#[derive(Debug, Clone)]
struct RuntimeClass {
    name: String,
    fields: Vec<RuntimeField>,
    subs: HashMap<String, Procedure>,
    functions: HashMap<String, Function>,
    properties: HashMap<String, RuntimeProperty>,
}

#[derive(Debug, Clone)]
struct RuntimeProperty {
    get: Option<RuntimePropertyAccessor>,
    let_: Option<RuntimePropertyAccessor>,
    set: Option<RuntimePropertyAccessor>,
}

#[derive(Debug, Clone)]
struct RuntimePropertyAccessor {
    name: String,
    kind: PropertyKind,
    params: Vec<crate::Parameter>,
    return_type: Option<TypeName>,
    body: Vec<Stmt>,
    span: Span,
}

impl From<&crate::ClassDecl> for RuntimeClass {
    fn from(value: &crate::ClassDecl) -> Self {
        let mut fields = Vec::new();
        let mut subs = HashMap::new();
        let mut functions = HashMap::new();
        let mut properties = HashMap::new();
        for member in &value.members {
            match member {
                ClassMember::Field(field) => fields.push(RuntimeField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                }),
                ClassMember::Sub(method) => {
                    subs.insert(key(&method.procedure.name), method.procedure.clone());
                }
                ClassMember::Function(method) => {
                    functions.insert(key(&method.function.name), method.function.clone());
                }
                ClassMember::Property(property) => {
                    let property_entry =
                        properties
                            .entry(key(&property.name))
                            .or_insert_with(|| RuntimeProperty {
                                get: None,
                                let_: None,
                                set: None,
                            });
                    let accessor = RuntimePropertyAccessor::from(property);
                    match property.kind {
                        PropertyKind::Get => property_entry.get = Some(accessor),
                        PropertyKind::Let => property_entry.let_ = Some(accessor),
                        PropertyKind::Set => property_entry.set = Some(accessor),
                    }
                }
            }
        }
        Self {
            name: value.name.clone(),
            fields,
            subs,
            functions,
            properties,
        }
    }
}

impl From<&ClassProperty> for RuntimePropertyAccessor {
    fn from(value: &ClassProperty) -> Self {
        Self {
            name: value.name.clone(),
            kind: value.kind,
            params: value.params.clone(),
            return_type: value.return_type.clone(),
            body: value.body.clone(),
            span: value.span,
        }
    }
}

pub fn run(program: &Program) -> Result<Vec<String>, Diagnostic> {
    Interpreter::new().run(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    use crate::semantics::validate;

    fn parse_and_validate(source: &str) -> Result<Program, Diagnostic> {
        let program = Parser::parse_source(source)?;
        validate(&program)?;
        Ok(program)
    }

    fn run_source(source: &str) -> Vec<String> {
        let program = parse_and_validate(source).unwrap();
        run(&program).unwrap()
    }

    fn source_error(source: &str) -> String {
        match parse_and_validate(source) {
            Ok(program) => run(&program).unwrap_err().to_string(),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn runs_variables_and_console_writeline() {
        let output = run_source(
            r#"
Sub Main()
    Dim name As String
    Dim count As Integer
    name = "Valo"
    count = 40 + 2
    Console.WriteLine("Hello, " & name & " " & count)
End Sub
"#,
        );

        assert_eq!(output, vec!["Hello, Valo 42"]);
    }

    #[test]
    fn runs_control_flow() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    Dim total As Integer
    i = 1
    total = 0
    While i <= 5
        total = total + i
        i = i + 1
    Wend
    If total = 15 Then
        Console.WriteLine("ok")
    Else
        Console.WriteLine("bad")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["ok"]);
    }

    #[test]
    fn reports_line_and_column_for_parse_errors() {
        let error = Parser::parse_source(
            r#"
Sub Main()
    Dim x As Integer
    x = 
End Sub
"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("line 4, column"));
    }

    #[test]
    fn variable_names_are_case_insensitive() {
        let output = run_source(
            r#"
Sub Main()
    Dim Name As String
    name = "Valo"
    Console.WriteLine(NAME)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn runs_nested_if_and_while_blocks() {
        let output = run_source(
            r#"
Sub Main()
    Dim outer As Integer
    Dim inner As Integer
    outer = 0

    While outer < 2
        inner = 0
        While inner < 2
            If outer = 1 Then
                Console.WriteLine("outer " & outer & ", inner " & inner)
            Else
                Console.WriteLine("skip")
            End If
            inner = inner + 1
        Wend
        outer = outer + 1
    Wend
End Sub
"#,
        );

        assert_eq!(
            output,
            vec!["skip", "skip", "outer 1, inner 0", "outer 1, inner 1"]
        );
    }

    #[test]
    fn reports_division_by_zero() {
        let error = source_error(
            r#"
Sub Main()
    Dim x As Integer
    x = 1 / 0
End Sub
"#,
        );

        assert!(error.contains("Division by zero"));
        assert!(error.contains("line 4, column"));
    }

    #[test]
    fn reports_undefined_variables() {
        let error = source_error(
            r#"
Sub Main()
    missing = 1
End Sub
"#,
        );

        assert!(error.contains("Variable 'missing' is not declared"));
    }

    #[test]
    fn reports_type_mismatch_errors() {
        let error = source_error(
            r#"
Sub Main()
    Dim x As Integer
    x = "nope"
End Sub
"#,
        );

        assert!(error.contains("Cannot assign String value to Integer variable"));
        assert!(error.contains("line 4, column"));
    }

    #[test]
    fn runs_simple_ascending_for_loop() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 3
        Console.WriteLine(i)
    Next
End Sub
"#,
        );

        assert_eq!(output, vec!["1", "2", "3"]);
    }

    #[test]
    fn runs_descending_for_loop_with_negative_step() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    For i = 3 To 1 Step -1
        Console.WriteLine(i)
    Next
End Sub
"#,
        );

        assert_eq!(output, vec!["3", "2", "1"]);
    }

    #[test]
    fn reports_undeclared_for_loop_variable() {
        let error = source_error(
            r#"
Sub Main()
    For i = 1 To 3
        Console.WriteLine(i)
    Next
End Sub
"#,
        );

        assert!(error.contains("Variable 'i' is not declared"));
    }

    #[test]
    fn reports_non_integer_for_loop_variable() {
        let error = source_error(
            r#"
Sub Main()
    Dim i As String
    For i = 1 To 3
        Console.WriteLine(i)
    Next
End Sub
"#,
        );

        assert!(error.contains("For loop variable 'i' must be Integer"));
    }

    #[test]
    fn runs_simple_integer_function() {
        let output = run_source(
            r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Console.WriteLine(Add(10, 20))
End Sub
"#,
        );

        assert_eq!(output, vec!["30"]);
    }

    #[test]
    fn runs_string_returning_function() {
        let output = run_source(
            r#"
Function Greeting(ByVal name As String) As String
    Return "Hello, " & name
End Function

Sub Main()
    Console.WriteLine(Greeting("Valo"))
End Sub
"#,
        );

        assert_eq!(output, vec!["Hello, Valo"]);
    }

    #[test]
    fn uses_function_call_inside_expression() {
        let output = run_source(
            r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Console.WriteLine(Add(1, 2) + Add(3, 4))
End Sub
"#,
        );

        assert_eq!(output, vec!["10"]);
    }

    #[test]
    fn reports_wrong_function_argument_count() {
        let error = source_error(
            r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Console.WriteLine(Add(1))
End Sub
"#,
        );

        assert!(error.contains("Function 'Add' expects 2 argument(s), got 1"));
    }

    #[test]
    fn reports_unknown_function() {
        let error = source_error(
            r#"
Sub Main()
    Console.WriteLine(Missing())
End Sub
"#,
        );

        assert!(error.contains("Function 'Missing' is not defined"));
    }

    #[test]
    fn reports_duplicate_parameter() {
        let error = source_error(
            r#"
Function Bad(ByVal value As Integer, ByVal VALUE As Integer) As Integer
    Return value
End Function

Sub Main()
    Console.WriteLine(Bad(1, 2))
End Sub
"#,
        );

        assert!(error.contains("Parameter 'VALUE' is already declared"));
    }

    #[test]
    fn reports_return_outside_function() {
        let error = source_error(
            r#"
Sub Main()
    Return 1
End Sub
"#,
        );

        assert!(error.contains("Return is only allowed inside Function"));
    }

    #[test]
    fn reports_missing_return() {
        let error = source_error(
            r#"
Function MissingReturn() As Integer
    Dim x As Integer
    x = 1
End Function

Sub Main()
    Console.WriteLine(MissingReturn())
End Sub
"#,
        );

        assert!(error.contains("Function 'MissingReturn' must return a value"));
    }

    #[test]
    fn reports_type_mismatch_return() {
        let error = source_error(
            r#"
Function Bad() As Integer
    Return "nope"
End Function

Sub Main()
    Console.WriteLine(Bad())
End Sub
"#,
        );

        assert!(error.contains("Cannot assign String value to Integer variable"));
    }

    #[test]
    fn isolates_main_and_function_local_variables() {
        let output = run_source(
            r#"
Function GetValue() As Integer
    Dim value As Integer
    value = 99
    Return value
End Function

Sub Main()
    Dim value As Integer
    value = 1
    Console.WriteLine(GetValue())
    Console.WriteLine(value)
End Sub
"#,
        );

        assert_eq!(output, vec!["99", "1"]);
    }

    #[test]
    fn runs_simple_sub_call() {
        let output = run_source(
            r#"
Sub SayHello()
    Console.WriteLine("Hello")
End Sub

Sub Main()
    SayHello()
End Sub
"#,
        );

        assert_eq!(output, vec!["Hello"]);
    }

    #[test]
    fn runs_sub_call_with_byval_parameter() {
        let output = run_source(
            r#"
Sub Show(ByVal value As String)
    Console.WriteLine(value)
End Sub

Sub Main()
    Show("Valo")
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn byref_sub_parameter_mutates_caller_variable() {
        let output = run_source(
            r#"
Sub Increment(ByRef value As Integer)
    value = value + 1
End Sub

Sub Main()
    Dim x As Integer
    x = 10
    Increment(x)
    Console.WriteLine(x)
End Sub
"#,
        );

        assert_eq!(output, vec!["11"]);
    }

    #[test]
    fn reports_byref_literal_argument() {
        let error = source_error(
            r#"
Sub Increment(ByRef value As Integer)
    value = value + 1
End Sub

Sub Main()
    Increment(10)
End Sub
"#,
        );

        assert!(error.contains("ByRef argument must be a variable"));
    }

    #[test]
    fn reports_byref_expression_argument() {
        let error = source_error(
            r#"
Sub Increment(ByRef value As Integer)
    value = value + 1
End Sub

Sub Main()
    Dim x As Integer
    x = 10
    Increment(x + 1)
End Sub
"#,
        );

        assert!(error.contains("ByRef argument must be a variable"));
    }

    #[test]
    fn reports_wrong_sub_argument_count() {
        let error = source_error(
            r#"
Sub Show(ByVal value As String)
    Console.WriteLine(value)
End Sub

Sub Main()
    Show()
End Sub
"#,
        );

        assert!(error.contains("Sub 'Show' expects 1 argument(s), got 0"));
    }

    #[test]
    fn reports_unknown_sub() {
        let error = source_error(
            r#"
Sub Main()
    Missing()
End Sub
"#,
        );

        assert!(error.contains("Sub 'Missing' is not defined"));
    }

    #[test]
    fn reports_duplicate_sub_function_name_conflict() {
        let error = source_error(
            r#"
Sub Same()
End Sub

Function Same() As Integer
    Return 1
End Function

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Name 'Same' conflicts with existing Sub"));
    }

    #[test]
    fn reports_duplicate_sub_name() {
        let error = source_error(
            r#"
Sub Same()
End Sub

Sub SAME()
End Sub

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Name 'SAME' conflicts with existing Sub"));
    }

    #[test]
    fn rejects_main_with_parameters() {
        let error = source_error(
            r#"
Sub Main(ByVal value As Integer)
    Console.WriteLine(value)
End Sub
"#,
        );

        assert!(error.contains("Sub Main() cannot have parameters"));
    }

    #[test]
    fn reports_sub_used_in_expression() {
        let error = source_error(
            r#"
Sub SayHello()
    Console.WriteLine("Hello")
End Sub

Sub Main()
    Dim value As Integer
    value = SayHello()
End Sub
"#,
        );

        assert!(error.contains("Sub 'SayHello' cannot be used as an expression"));
    }

    #[test]
    fn reports_function_called_as_statement() {
        let error = source_error(
            r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Add(1, 2)
End Sub
"#,
        );

        assert!(error.contains("Function 'Add' cannot be called as a statement"));
    }

    #[test]
    fn sub_calls_can_be_nested_and_call_functions() {
        let output = run_source(
            r#"
Function Label(ByVal value As Integer) As String
    Return "Value: " & value
End Function

Sub PrintLabel(ByVal value As Integer)
    Console.WriteLine(Label(value))
End Sub

Sub Outer(ByRef value As Integer)
    value = value + 1
    PrintLabel(value)
End Sub

Sub Main()
    Dim x As Integer
    x = 4
    Outer(x)
    Console.WriteLine(x)
End Sub
"#,
        );

        assert_eq!(output, vec!["Value: 5", "5"]);
    }

    #[test]
    fn declares_type_dims_variable_and_reads_default_fields() {
        let output = run_source(
            r#"
Type User
    Name As String
    Age As Integer
    Active As Boolean
End Type

Sub Main()
    Dim user As User
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Age)
    Console.WriteLine(user.Active)
End Sub
"#,
        );

        assert_eq!(output, vec!["", "0", "False"]);
    }

    #[test]
    fn assigns_and_reads_members() {
        let output = run_source(
            r#"
Type User
    Name As String
    Age As Integer
End Type

Sub Main()
    Dim user As User
    user.Name = "Valo"
    user.Age = 1
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Age)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo", "1"]);
    }

    #[test]
    fn returns_user_defined_type_from_function() {
        let output = run_source(
            r#"
Type User
    Name As String
    Age As Integer
    Active As Boolean
End Type

Function CreateUser(ByVal name As String, ByVal age As Integer) As User
    Dim u As User
    u.Name = name
    u.Age = age
    u.Active = True
    Return u
End Function

Sub Main()
    Dim user As User
    user = CreateUser("Valo", 1)
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Age)
    Console.WriteLine(user.Active)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo", "1", "True"]);
    }

    #[test]
    fn reports_unknown_type() {
        let error = source_error(
            r#"
Sub Main()
    Dim user As Missing
End Sub
"#,
        );

        assert!(error.contains("Type 'Missing' is not defined"));
    }

    #[test]
    fn reports_duplicate_type() {
        let error = source_error(
            r#"
Type User
    Name As String
End Type

Type user
    Age As Integer
End Type

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Type 'user' is already defined"));
    }

    #[test]
    fn reports_duplicate_field() {
        let error = source_error(
            r#"
Type User
    Name As String
    NAME As String
End Type

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Field 'NAME' is already declared in Type 'User'"));
    }

    #[test]
    fn reports_unknown_field() {
        let error = source_error(
            r#"
Type User
    Name As String
End Type

Sub Main()
    Dim user As User
    Console.WriteLine(user.Age)
End Sub
"#,
        );

        assert!(error.contains("Type 'User' has no field 'Age'"));
    }

    #[test]
    fn reports_field_type_mismatch() {
        let error = source_error(
            r#"
Type User
    Age As Integer
End Type

Sub Main()
    Dim user As User
    user.Age = "old"
End Sub
"#,
        );

        assert!(error.contains("Cannot assign String value to Integer variable"));
    }

    #[test]
    fn type_and_field_names_are_case_insensitive() {
        let output = run_source(
            r#"
Type User
    Name As String
End Type

Sub Main()
    Dim user As user
    USER.name = "Valo"
    Console.WriteLine(user.NAME)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn byref_type_parameter_can_mutate_field() {
        let output = run_source(
            r#"
Type User
    Name As String
    Active As Boolean
End Type

Sub Activate(ByRef user As User)
    user.Active = True
End Sub

Sub Main()
    Dim user As User
    user.Name = "Valo"
    Activate(user)
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Active)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo", "True"]);
    }

    #[test]
    fn declares_fixed_integer_array() {
        let output = run_source(
            r#"
Sub Main()
    Dim numbers(3) As Integer
    Console.WriteLine(numbers(0))
    Console.WriteLine(numbers(3))
End Sub
"#,
        );

        assert_eq!(output, vec!["0", "0"]);
    }

    #[test]
    fn assigns_and_reads_array_elements() {
        let output = run_source(
            r#"
Sub Main()
    Dim numbers(3) As Integer
    numbers(0) = 10
    numbers(1) = 20
    numbers(2) = 30
    Console.WriteLine(numbers(0))
    Console.WriteLine(numbers(1))
    Console.WriteLine(numbers(2))
End Sub
"#,
        );

        assert_eq!(output, vec!["10", "20", "30"]);
    }

    #[test]
    fn supports_expression_array_index() {
        let output = run_source(
            r#"
Sub Main()
    Dim numbers(3) As Integer
    Dim i As Integer
    i = 1
    numbers(i + 1) = 42
    Console.WriteLine(numbers(2))
End Sub
"#,
        );

        assert_eq!(output, vec!["42"]);
    }

    #[test]
    fn reports_array_bounds_error() {
        let error = source_error(
            r#"
Sub Main()
    Dim numbers(1) As Integer
    Console.WriteLine(numbers(2))
End Sub
"#,
        );

        assert!(error.contains("Array index 2 is out of bounds for length 2"));
    }

    #[test]
    fn reports_scalar_used_as_array() {
        let error = source_error(
            r#"
Sub Main()
    Dim number As Integer
    Console.WriteLine(number(0))
End Sub
"#,
        );

        assert!(error.contains("Variable 'number' is not an array"));
    }

    #[test]
    fn reports_wrong_array_element_type() {
        let error = source_error(
            r#"
Sub Main()
    Dim numbers(1) As Integer
    numbers(0) = "nope"
End Sub
"#,
        );

        assert!(error.contains("Cannot assign String value to Integer variable"));
    }

    #[test]
    fn supports_array_of_user_defined_type() {
        let output = run_source(
            r#"
Type User
    Name As String
    Age As Integer
End Type

Sub Main()
    Dim users(2) As User
    users(0).Name = "Valo"
    users(0).Age = 1
    Console.WriteLine(users(0).Name)
    Console.WriteLine(users(0).Age)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo", "1"]);
    }

    #[test]
    fn reports_array_used_as_scalar() {
        let error = source_error(
            r#"
Sub Main()
    Dim numbers(1) As Integer
    Console.WriteLine(numbers)
End Sub
"#,
        );

        assert!(error.contains("Array variable 'numbers' cannot be used as a scalar"));
    }

    #[test]
    fn creates_class_instance_and_calls_constructor() {
        let output = run_source(
            r#"
Class User
    Public Name As String

    Public Sub Initialize(ByVal name As String)
        Me.Name = name
    End Sub
End Class

Sub Main()
    Dim user As User
    user = New User("Valo")
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn class_method_mutation_persists_and_assignment_is_reference_like() {
        let output = run_source(
            r#"
Class User
    Public Name As String

    Public Sub Rename(ByVal name As String)
        Me.Name = name
    End Sub
End Class

Sub Main()
    Dim a As User
    Dim b As User
    a = New User()
    b = a
    b.Rename("Changed")
    Console.WriteLine(a.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Changed"]);
    }

    #[test]
    fn class_function_method_returns_value() {
        let output = run_source(
            r#"
Class User
    Private Age As Integer

    Public Sub Initialize(ByVal age As Integer)
        Me.Age = age
    End Sub

    Public Function IsAdult() As Boolean
        Return Me.Age >= 18
    End Function
End Class

Sub Main()
    Dim user As User
    user = New User(20)
    Console.WriteLine(user.IsAdult())
End Sub
"#,
        );

        assert_eq!(output, vec!["True"]);
    }

    #[test]
    fn private_field_access_outside_class_is_rejected() {
        let error = source_error(
            r#"
Class User
    Private Age As Integer
End Class

Sub Main()
    Dim user As User
    user = New User()
    Console.WriteLine(user.Age)
End Sub
"#,
        );

        assert!(error.contains("Member 'Age' is Private in Class 'User'"));
    }

    #[test]
    fn private_method_call_through_me_is_allowed() {
        let output = run_source(
            r#"
Class User
    Private Active As Boolean

    Private Sub SetActive(ByVal value As Boolean)
        Me.Active = value
    End Sub

    Public Sub Activate()
        Me.SetActive(True)
    End Sub

    Public Function IsActive() As Boolean
        Return Me.Active
    End Function
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Activate()
    Console.WriteLine(user.IsActive())
End Sub
"#,
        );

        assert_eq!(output, vec!["True"]);
    }

    #[test]
    fn private_method_call_outside_class_is_rejected() {
        let error = source_error(
            r#"
Class User
    Private Sub Hide()
    End Sub
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Hide()
End Sub
"#,
        );

        assert!(error.contains("Member 'Hide' is Private in Class 'User'"));
    }

    #[test]
    fn me_outside_class_is_rejected() {
        let error = source_error(
            r#"
Sub Main()
    Console.WriteLine(Me)
End Sub
"#,
        );

        assert!(error.contains("Me is only valid inside class methods"));
    }

    #[test]
    fn type_record_assignment_remains_value_copy() {
        let output = run_source(
            r#"
Type User
    Name As String
End Type

Sub Main()
    Dim a As User
    Dim b As User
    a.Name = "Original"
    b = a
    b.Name = "Changed"
    Console.WriteLine(a.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Original"]);
    }

    #[test]
    fn evaluates_and_behavior() {
        let output = run_source(
            r#"
Sub Main()
    Console.WriteLine(True And True)
    Console.WriteLine(True And False)
End Sub
"#,
        );

        assert_eq!(output, vec!["True", "False"]);
    }

    #[test]
    fn evaluates_or_behavior() {
        let output = run_source(
            r#"
Sub Main()
    Console.WriteLine(False Or True)
    Console.WriteLine(False Or False)
End Sub
"#,
        );

        assert_eq!(output, vec!["True", "False"]);
    }

    #[test]
    fn evaluates_not_behavior() {
        let output = run_source(
            r#"
Sub Main()
    Console.WriteLine(Not True)
    Console.WriteLine(Not False)
End Sub
"#,
        );

        assert_eq!(output, vec!["False", "True"]);
    }

    #[test]
    fn evaluates_mod_result() {
        let output = run_source(
            r#"
Sub Main()
    Console.WriteLine(10 Mod 3)
End Sub
"#,
        );

        assert_eq!(output, vec!["1"]);
    }

    #[test]
    fn reports_mod_by_zero() {
        let error = source_error(
            r#"
Sub Main()
    Console.WriteLine(10 Mod 0)
End Sub
"#,
        );

        assert!(error.contains("Modulo by zero"));
    }

    #[test]
    fn elseif_uses_first_matching_branch() {
        let output = run_source(
            r#"
Sub Main()
    Dim age As Integer
    Dim active As Boolean
    age = 20
    active = False

    If age < 18 Then
        Console.WriteLine("Denied")
    ElseIf age >= 18 And active Then
        Console.WriteLine("Allowed")
    ElseIf age >= 18 Then
        Console.WriteLine("Inactive")
    Else
        Console.WriteLine("Other")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["Inactive"]);
    }

    #[test]
    fn elseif_falls_through_to_else() {
        let output = run_source(
            r#"
Sub Main()
    Dim age As Integer
    age = 12

    If age > 20 Then
        Console.WriteLine("adult")
    ElseIf age = 18 Then
        Console.WriteLine("exact")
    Else
        Console.WriteLine("minor")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["minor"]);
    }

    #[test]
    fn logical_operator_precedence() {
        let output = run_source(
            r#"
Sub Main()
    Console.WriteLine(True Or False And False)
    Console.WriteLine(Not False And False)
    Console.WriteLine(Not (False And False))
End Sub
"#,
        );

        assert_eq!(output, vec!["True", "False", "True"]);
    }

    #[test]
    fn existing_if_without_elseif_still_works() {
        let output = run_source(
            r#"
Sub Main()
    If True Then
        Console.WriteLine("ok")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["ok"]);
    }

    #[test]
    fn property_read_calls_get() {
        let output = run_source(
            r#"
Class User
    Private mName As String

    Public Property Get Name() As String
        Return Me.mName & "!"
    End Property

    Public Sub SetName(ByVal value As String)
        Me.mName = value
    End Sub
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.SetName("Valo")
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo!"]);
    }

    #[test]
    fn property_assignment_calls_let() {
        let output = run_source(
            r#"
Class User
    Private mName As String

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value & " Runtime"
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Name = "Valo"
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo Runtime"]);
    }

    #[test]
    fn property_validation_logic_mutates_backing_field() {
        let output = run_source(
            r#"
Class User
    Private mAge As Integer

    Public Property Get Age() As Integer
        Return Me.mAge
    End Property

    Public Property Let Age(ByVal value As Integer)
        If value < 0 Then
            Me.mAge = 0
        Else
            Me.mAge = value
        End If
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Age = -1
    Console.WriteLine(user.Age)
End Sub
"#,
        );

        assert_eq!(output, vec!["0"]);
    }

    #[test]
    fn private_property_access_outside_class_is_rejected() {
        let error = source_error(
            r#"
Class User
    Private mName As String

    Private Property Get Name() As String
        Return Me.mName
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert!(error.contains("Member 'Name' is Private in Class 'User'"));
    }

    #[test]
    fn private_property_access_inside_class_is_allowed() {
        let output = run_source(
            r#"
Class User
    Private mName As String

    Private Property Get Name() As String
        Return Me.mName
    End Property

    Private Property Let Name(ByVal value As String)
        Me.mName = value
    End Property

    Public Sub Rename(ByVal value As String)
        Me.Name = value
    End Sub

    Public Function Label() As String
        Return Me.Name
    End Function
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Rename("Valo")
    Console.WriteLine(user.Label())
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn missing_get_when_reading_property_produces_error() {
        let error = source_error(
            r#"
Class User
    Public Property Let Name(ByVal value As String)
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert!(error.contains("Property 'Name' has no Get accessor"));
    }

    #[test]
    fn missing_let_or_set_when_assigning_property_produces_error() {
        let error = source_error(
            r#"
Class User
    Public Property Get Name() As String
        Return "Valo"
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Name = "Runtime"
End Sub
"#,
        );

        assert!(error.contains("Property 'Name' has no Let or Set accessor"));
    }

    #[test]
    fn duplicate_property_get_is_rejected() {
        let error = source_error(
            r#"
Class User
    Public Property Get Name() As String
        Return "a"
    End Property

    Public Property Get Name() As String
        Return "b"
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property Get 'Name' is already declared"));
    }

    #[test]
    fn duplicate_property_let_is_rejected() {
        let error = source_error(
            r#"
Class User
    Public Property Let Name(ByVal value As String)
    End Property

    Public Property Let Name(ByVal value As String)
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property Let 'Name' is already declared"));
    }

    #[test]
    fn property_conflicts_with_field_name() {
        let error = source_error(
            r#"
Class User
    Public Name As String

    Public Property Get Name() As String
        Return "Valo"
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property 'Name' conflicts with another member"));
    }

    #[test]
    fn property_conflicts_with_method_name() {
        let error = source_error(
            r#"
Class User
    Public Sub Name()
    End Sub

    Public Property Get Name() As String
        Return "Valo"
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property 'Name' conflicts with another member"));
    }

    #[test]
    fn property_get_missing_return_is_rejected() {
        let error = source_error(
            r#"
Class User
    Public Property Get Name() As String
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property Get 'Name' must return a value"));
    }

    #[test]
    fn property_let_with_wrong_parameter_count_is_rejected() {
        let error = source_error(
            r#"
Class User
    Public Property Let Name()
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property Let 'Name' must have exactly one parameter"));
    }

    #[test]
    fn property_set_assigns_object_reference() {
        let output = run_source(
            r#"
Class Owner
    Public Name As String
End Class

Class Item
    Private mOwner As Owner

    Public Property Get Owner() As Owner
        Return Me.mOwner
    End Property

    Public Property Set Owner(ByVal value As Owner)
        Me.mOwner = value
    End Property
End Class

Sub Main()
    Dim owner As Owner
    Dim item As Item
    owner = New Owner()
    owner.Name = "Valo"
    item = New Item()
    item.Owner = owner
    Console.WriteLine(item.Owner.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn set_object_assignment_works() {
        let output = run_source(
            r#"
Class User
    Public Name As String

    Public Sub Initialize(ByVal name As String)
        Me.Name = name
    End Sub
End Class

Sub Main()
    Dim user As User
    Set user = New User("Valo")
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn normal_object_assignment_still_works() {
        let output = run_source(
            r#"
Class User
    Public Name As String
End Class

Sub Main()
    Dim user As User
    Dim aliasUser As User
    user = New User()
    user.Name = "Valo"
    aliasUser = user
    Console.WriteLine(aliasUser.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn class_variable_defaults_to_nothing() {
        let output = run_source(
            r#"
Class User
End Class

Sub Main()
    Dim user As User
    If user Is Nothing Then
        Console.WriteLine("empty")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["empty"]);
    }

    #[test]
    fn field_access_on_nothing_errors_clearly() {
        let error = source_error(
            r#"
Class User
    Public Name As String
End Class

Sub Main()
    Dim user As User
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert!(error.contains("Object reference is Nothing"));
    }

    #[test]
    fn method_call_on_nothing_errors_clearly() {
        let error = source_error(
            r#"
Class User
    Public Sub Rename(ByVal value As String)
    End Sub
End Class

Sub Main()
    Dim user As User
    user.Rename("Valo")
End Sub
"#,
        );

        assert!(error.contains("Object reference is Nothing"));
    }

    #[test]
    fn set_user_to_nothing() {
        let output = run_source(
            r#"
Class User
End Class

Sub Main()
    Dim user As User
    Set user = New User()
    Set user = Nothing
    If user Is Nothing Then
        Console.WriteLine("empty")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["empty"]);
    }

    #[test]
    fn is_nothing_false_after_new() {
        let output = run_source(
            r#"
Class User
End Class

Sub Main()
    Dim user As User
    Set user = New User()
    If Not (user Is Nothing) Then
        Console.WriteLine("present")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["present"]);
    }

    #[test]
    fn object_identity_with_is() {
        let output = run_source(
            r#"
Class User
End Class

Sub Main()
    Dim user As User
    Dim aliasUser As User
    Set user = New User()
    Set aliasUser = user
    If user Is aliasUser Then
        Console.WriteLine("same")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["same"]);
    }

    #[test]
    fn set_rejected_for_integer() {
        let error = source_error(
            r#"
Sub Main()
    Dim value As Integer
    Set value = 1
End Sub
"#,
        );

        assert!(error.contains("Set target must be a class type"));
    }

    #[test]
    fn set_rejected_for_string() {
        let error = source_error(
            r#"
Sub Main()
    Dim value As String
    Set value = "Valo"
End Sub
"#,
        );

        assert!(error.contains("Set target must be a class type"));
    }

    #[test]
    fn nothing_rejected_for_builtin_values() {
        for source in [
            r#"
Sub Main()
    Dim value As Integer
    value = Nothing
End Sub
"#,
            r#"
Sub Main()
    Dim value As String
    value = Nothing
End Sub
"#,
            r#"
Sub Main()
    Dim value As Boolean
    value = Nothing
End Sub
"#,
        ] {
            let error = source_error(source);
            assert!(error.contains("Nothing requires a class object type"));
        }
    }

    #[test]
    fn nothing_rejected_for_type_records() {
        let error = source_error(
            r#"
Type User
    Name As String
End Type

Sub Main()
    Dim user As User
    user = Nothing
End Sub
"#,
        );

        assert!(error.contains("Nothing requires a class object type"));
    }
}
