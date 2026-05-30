use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

use crate::ArrayDecl;
use crate::runtime::{
    ArrayBound, ArrayValue, Diagnostic, ObjectValue, Span, TypeName, Value, coerce_assignment,
};

use super::Interpreter;
use super::arrays::{read_array_element, redim_array, write_array_element};
use super::records::{RuntimeInterface, RuntimeType};
use super::values::{default_value, key};

#[derive(Debug, Clone)]
pub(crate) enum VariableCell {
    Direct(Rc<RefCell<Value>>),
    ArrayElement {
        array: Rc<RefCell<Value>>,
        index: usize,
    },
    Member {
        object: Rc<RefCell<Value>>,
        member: String,
    },
}

impl VariableCell {
    pub fn borrow(&self) -> Ref<'_, Value> {
        match self {
            VariableCell::Direct(cell) => cell.borrow(),
            VariableCell::ArrayElement { array, index } => Ref::map(array.borrow(), |val| {
                if let Value::Array(array) = val {
                    &array.elements[*index]
                } else {
                    panic!("Expected array in VariableCell::ArrayElement");
                }
            }),
            VariableCell::Member { object, member } => Ref::map(object.borrow(), |v| {
                if let Value::Record(record) = v {
                    record.fields.get(&key(member)).expect("field missing")
                } else {
                    panic!("Expected record in VariableCell::Member");
                }
            }),
        }
    }

    pub fn borrow_mut(&self) -> RefMut<'_, Value> {
        match self {
            VariableCell::Direct(cell) => cell.borrow_mut(),
            VariableCell::ArrayElement { array, index } => RefMut::map(array.borrow_mut(), |val| {
                if let Value::Array(array) = val {
                    &mut Rc::make_mut(array).elements[*index]
                } else {
                    panic!("Expected array in VariableCell::ArrayElement");
                }
            }),
            VariableCell::Member { object, member } => RefMut::map(object.borrow_mut(), |v| {
                if let Value::Record(record) = v {
                    Rc::make_mut(record)
                        .fields
                        .get_mut(&key(member))
                        .expect("field missing")
                } else {
                    panic!("Expected record in VariableCell::Member");
                }
            }),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Frame {
    variables: HashMap<String, Variable>,
    return_slots: HashMap<String, Value>,
    module_key: Option<String>,
    class_context: Option<String>,
    with_stack: Vec<Value>,
    yielded_values: Option<Vec<Value>>,
    resume_next: bool,
    error_handler: Option<String>,
    handled_error_ip: Option<usize>,
}

impl Frame {
    pub(crate) fn set_return_slot(&mut self, slot: String, value: Value) {
        self.return_slots.insert(slot, value);
    }

    pub(crate) fn get_return_slot(&self, slot: &str) -> Option<Value> {
        self.return_slots.get(slot).cloned()
    }
    pub(crate) fn set_yield_mode(&mut self) {
        self.yielded_values = Some(Vec::new());
    }

    pub(crate) fn yield_value(&mut self, value: Value) {
        if let Some(yielded) = &mut self.yielded_values {
            yielded.push(value);
        }
    }

    pub(crate) fn take_yielded_values(&mut self) -> Option<Vec<Value>> {
        self.yielded_values.take()
    }

    pub(crate) fn set_module_key(&mut self, module_key: String) {
        self.module_key = Some(module_key);
    }

    pub(crate) fn module_key(&self) -> Option<&str> {
        self.module_key.as_deref()
    }

    pub(crate) fn set_class_context(&mut self, class_name: String) {
        self.class_context = Some(class_name);
    }

    pub(crate) fn class_context(&self) -> Option<&str> {
        self.class_context.as_deref()
    }

    pub(crate) fn current_class_name(&self) -> Option<String> {
        if let Some(context) = &self.class_context {
            return Some(context.clone());
        }
        self.variables
            .get("me")
            .and_then(|variable| match &variable.ty {
                TypeName::User(name) => Some(name.clone()),
                _ => None,
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn declare(
        &mut self,
        name: &str,
        ty: TypeName,
        array: Option<ArrayDecl>,
        _option_base: i64,
        span: Span,
        interpreter: &Interpreter,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if self.variables.contains_key(&key) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Variable '{}' is already declared", name),
                Some(span),
            ));
        }

        let dynamic_array = matches!(array, Some(ArrayDecl::Dynamic));
        let value = if let Some(array) = array {
            let mut elements = Vec::new();
            let mut bounds = Vec::new();
            let mut is_dynamic = false;
            let allocated = match array {
                ArrayDecl::Fixed(fixed_bounds) => {
                    let mut total_len: usize = 1;
                    for bound in fixed_bounds {
                        total_len *= (bound.upper - bound.lower + 1) as usize;
                        bounds.push(bound);
                    }
                    for _ in 0..total_len {
                        elements.push(default_value(&ty, interpreter, span)?);
                    }
                    true
                }
                ArrayDecl::Dynamic => {
                    is_dynamic = true;
                    false
                }
            };

            Value::Array(Rc::new(ArrayValue {
                element_type: ty.clone(),
                elements,
                bounds,
                allocated,
                dynamic: is_dynamic,
            }))
        } else {
            default_value(&ty, interpreter, span)?
        };

        self.variables.insert(
            key,
            Variable {
                name: name.to_string(),
                ty: ty.clone(),
                cell: VariableCell::Direct(Rc::new(RefCell::new(value))),
                dynamic_array,
                is_const: false,
                module_level: false,
            },
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn declare_static(
        &mut self,
        name: &str,
        ty: TypeName,
        array: Option<ArrayDecl>,
        option_base: i64,
        span: Span,
        interpreter: &Interpreter,
        static_frame: &mut Frame,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if !static_frame.variables.contains_key(&key) {
            static_frame.declare(name, ty.clone(), array, option_base, span, interpreter)?;
        }
        let variable = static_frame.variable(name, span)?;
        self.declare_alias(
            name,
            ty,
            variable,
            span,
            &interpreter.types,
            &interpreter.interfaces,
        )
    }

    pub(crate) fn declare_const(
        &mut self,
        name: &str,
        ty: TypeName,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if self.variables.contains_key(&key) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Variable '{}' is already declared", name),
                Some(span),
            ));
        }
        self.variables.insert(
            key,
            Variable {
                name: name.to_string(),
                cell: VariableCell::Direct(Rc::new(RefCell::new(coerce_assignment(
                    &ty, value, span,
                )?))),
                ty,
                dynamic_array: false,
                is_const: true,
                module_level: false,
            },
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn declare_module(
        &mut self,
        name: &str,
        ty: TypeName,
        array: Option<ArrayDecl>,
        option_base: i64,
        is_const: bool,
        value: Option<Value>,
        span: Span,
        interpreter: &Interpreter,
    ) -> Result<(), Diagnostic> {
        self.declare(name, ty.clone(), array, option_base, span, interpreter)?;
        let variable = self.variables.get_mut(&key(name)).expect("declared");
        variable.module_level = true;
        variable.is_const = is_const;
        if let Some(value) = value {
            *variable.borrow_mut() = coerce_assignment(&ty, value, span)?;
        }
        Ok(())
    }

    pub(crate) fn declare_alias(
        &mut self,
        name: &str,
        ty: TypeName,
        variable: Variable,
        span: Span,
        types: &HashMap<String, RuntimeType>,
        _interfaces: &HashMap<String, RuntimeInterface>,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if self.variables.contains_key(&key) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Variable '{}' is already declared", name),
                Some(span),
            ));
        }

        let mut types_match = variable.ty.same_type(&ty);

        if !types_match
            && let TypeName::User(target_name) = &ty
            && let TypeName::User(source_name) = &variable.ty
        {
            // Case 1: Source is Structure, Target is Interface (implemented by Structure)
            if let Some(record_sig) = types.get(&super::values::key(source_name))
                && record_sig
                    .implements
                    .iter()
                    .any(|i| i.display_name().eq_ignore_ascii_case(target_name))
            {
                types_match = true;
            }
            // Case 2: Source is Interface (boxed record), Target is original Structure
            else if let Some(target_sig) = types.get(&super::values::key(target_name))
                && target_sig
                    .implements
                    .iter()
                    .any(|i| i.display_name().eq_ignore_ascii_case(source_name))
            {
                types_match = true;
            }
            // Case 3: Same name (case-insensitive)
            else if source_name.eq_ignore_ascii_case(target_name) {
                types_match = true;
            }
        }

        if !types_match {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
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

    pub(crate) fn inherit_modules_from(&mut self, source: &Frame) -> Result<(), Diagnostic> {
        for (name, variable) in &source.variables {
            if variable.module_level && !self.variables.contains_key(name) {
                self.variables.insert(name.clone(), variable.clone());
            }
        }
        Ok(())
    }

    pub(crate) fn declare_object_alias(
        &mut self,
        name: &str,
        class_name: &str,
        object: Rc<RefCell<ObjectValue>>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if self.variables.contains_key(&key) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Variable '{}' is already declared", name),
                Some(span),
            ));
        }
        self.variables.insert(
            key,
            Variable {
                name: name.to_string(),
                ty: TypeName::User(class_name.to_string()),
                cell: VariableCell::Direct(Rc::new(RefCell::new(Value::Object(object)))),
                dynamic_array: false,
                is_const: false,
                module_level: false,
            },
        );
        Ok(())
    }

    pub(crate) fn assign(
        &mut self,
        name: &str,
        value: Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let variable_key = key(name);
        if !self.variables.contains_key(&variable_key) {
            return Err(self.unknown_variable(name, span));
        }
        let variable = self
            .variables
            .get_mut(&variable_key)
            .expect("checked above");
        if variable.is_const {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::INVALID_ASSIGNMENT,
                format!("Constant '{}' cannot be assigned", name),
                Some(span),
            )
            .with_primary_label("assignment to constant")
            .with_help("remove the assignment or use a non-Const variable"));
        }

        let old = variable.borrow().clone();
        *variable.borrow_mut() = coerce_assignment(&variable.ty, value, span)?;
        Ok(old)
    }

    pub(crate) fn unknown_variable(&self, name: &str, span: Span) -> Diagnostic {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Variable '{}' is not declared", name),
            Some(span),
        )
        .with_primary_label("unknown variable")
        .with_help("declare the variable before using it")
        .with_name_suggestion(
            name,
            self.variables
                .values()
                .map(|variable| variable.name.as_str()),
        )
    }

    pub(crate) fn assign_missing(&mut self, name: &str, span: Span) -> Result<(), Diagnostic> {
        let variable_key = key(name);
        if !self.variables.contains_key(&variable_key) {
            return Err(self.unknown_variable(name, span));
        }
        let variable = self
            .variables
            .get_mut(&variable_key)
            .expect("checked above");
        *variable.borrow_mut() = Value::Missing;
        Ok(())
    }

    pub(crate) fn get(&self, name: &str, span: Span) -> Result<Value, Diagnostic> {
        self.variables
            .get(&key(name))
            .map(|variable| variable.borrow().clone())
            .ok_or_else(|| self.unknown_variable(name, span))
    }

    pub(crate) fn variable(&self, name: &str, span: Span) -> Result<Variable, Diagnostic> {
        self.variables
            .get(&key(name))
            .cloned()
            .ok_or_else(|| self.unknown_variable(name, span))
    }

    pub(crate) fn remove_variable(&mut self, name: &str) -> Option<Variable> {
        self.variables.remove(&key(name))
    }

    pub(crate) fn has_variable(&self, name: &str) -> bool {
        self.variables.contains_key(&key(name))
    }

    pub(crate) fn get_array_element(
        &self,
        name: &str,
        indices: &[i64],
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let variable = self.variable(name, span)?;
        let array = variable.borrow();
        read_array_element(&array, indices, span)
    }

    pub(crate) fn assign_array_element(
        &mut self,
        name: &str,
        indices: &[i64],
        value: Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let variable = self.variable(name, span)?;
        let mut array = variable.borrow_mut();
        write_array_element(&mut array, indices, value, span)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn redim_array(
        &mut self,
        name: &str,
        new_bounds: Vec<ArrayBound>,
        preserve: bool,
        interpreter: &Interpreter,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let variable = self.variable(name, span)?;
        if !variable.dynamic_array {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                "ReDim target must be a dynamic array",
                Some(span),
            )
            .with_primary_label("ReDim target is not a dynamic array"));
        }
        let mut array = variable.borrow_mut();
        redim_array(&mut array, new_bounds, preserve, interpreter, span)
    }

    pub(crate) fn erase_array(
        &mut self,
        name: &str,
        span: Span,
        interpreter: &Interpreter,
    ) -> Result<(), Diagnostic> {
        let variable = self.variable(name, span)?;
        let mut array_val = variable.borrow_mut();
        let Value::Array(array) = &mut *array_val else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                "Erase target must be an array",
                Some(span),
            ));
        };
        let array = Rc::make_mut(array);
        if array.dynamic {
            array.elements.clear();
            array.bounds.clear();
            array.allocated = false;
        } else {
            for element in &mut array.elements {
                *element = default_value(&array.element_type, interpreter, span)?;
            }
        }
        Ok(())
    }

    pub(crate) fn push_with_target(&mut self, value: Value) {
        self.with_stack.push(value);
    }

    pub(crate) fn pop_with_target(&mut self) {
        self.with_stack.pop();
    }

    pub(crate) fn current_with_target(&self, span: Span) -> Result<Value, Diagnostic> {
        self.with_stack.last().cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Dot member access requires an active With block",
                Some(span),
            )
            .with_primary_label("no active With target")
            .with_help("use dotted member access only inside a With block")
        })
    }

    pub(crate) fn simple_index_value(
        &self,
        expr: &crate::Expr,
        span: Span,
    ) -> Result<i64, Diagnostic> {
        use crate::ExprKind;
        match &expr.kind {
            ExprKind::Integer(value) => Ok(*value),
            ExprKind::Variable(name) => match self.get(name, expr.span)? {
                Value::Byte(v) => Ok(v as i64),
                Value::Int16(v) => Ok(v as i64),
                Value::Int32(v) => Ok(v as i64),
                Value::Int64(v) => Ok(v),
                Value::UInt32(v) => Ok(v as i64),
                Value::UInt64(v) => Ok(v as i64),
                _ => Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    "Array index must be Integer",
                    Some(span),
                )),
            },
            ExprKind::PassingModeOverride { expr: inner, .. } => {
                self.simple_index_value(inner, span)
            }
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                "Array member assignment index must be an Integer literal or variable",
                Some(span),
            )),
        }
    }

    pub(crate) fn into_variables(self) -> HashMap<String, Variable> {
        self.variables
    }

    pub(crate) fn set_resume_next(&mut self, enabled: bool) {
        self.resume_next = enabled;
        if enabled {
            self.error_handler = None;
        }
    }

    pub(crate) fn resume_next(&self) -> bool {
        self.resume_next
    }

    pub(crate) fn set_error_handler(&mut self, label: Option<String>) {
        self.error_handler = label;
        self.resume_next = false;
        self.handled_error_ip = None;
    }

    pub(crate) fn error_handler(&self) -> Option<&str> {
        self.error_handler.as_deref()
    }

    pub(crate) fn set_handled_error_ip(&mut self, ip: usize) {
        self.handled_error_ip = Some(ip);
    }

    pub(crate) fn handled_error_ip(&self) -> Option<usize> {
        self.handled_error_ip
    }

    pub(crate) fn clear_handled_error(&mut self) {
        self.handled_error_ip = None;
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Variable {
    pub(crate) name: String,
    pub(crate) ty: TypeName,
    pub(crate) cell: VariableCell,
    pub(crate) dynamic_array: bool,
    pub(crate) is_const: bool,
    pub(crate) module_level: bool,
}

impl Variable {
    pub fn borrow(&self) -> Ref<'_, Value> {
        self.cell.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, Value> {
        self.cell.borrow_mut()
    }
}
