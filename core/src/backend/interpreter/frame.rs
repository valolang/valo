use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ArrayDecl;
use crate::runtime::{
    ArrayBound, Diagnostic, ObjectValue, Span, TypeName, Value, coerce_assignment,
};

use super::arrays::{read_array_element, redim_array, write_array_element};
use super::records::{RuntimeEnum, RuntimeType};
use super::values::{default_value, key};

#[derive(Debug, Default, Clone)]
pub struct Frame {
    variables: HashMap<String, Variable>,
    module_key: Option<String>,
    with_stack: Vec<Value>,
    yielded_values: Option<Vec<Value>>,
    resume_next: bool,
    error_handler: Option<String>,
    handled_error_ip: Option<usize>,
}

impl Frame {
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn declare(
        &mut self,
        name: &str,
        ty: TypeName,
        array: Option<ArrayDecl>,
        _option_base: i64,
        span: Span,
        types: &HashMap<String, RuntimeType>,
        enums: &HashMap<String, RuntimeEnum>,
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
                        elements.push(default_value(&ty, types, enums, span)?);
                    }
                    true
                }
                ArrayDecl::Dynamic => {
                    is_dynamic = true;
                    false
                }
            };

            Value::Array {
                element_type: ty.clone(),
                elements,
                bounds,
                allocated,
                dynamic: is_dynamic,
            }
        } else {
            default_value(&ty, types, enums, span)?
        };

        self.variables.insert(
            key,
            Variable {
                cell: Rc::new(RefCell::new(value)),
                ty,
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
        types: &HashMap<String, RuntimeType>,
        enums: &HashMap<String, RuntimeEnum>,
        static_frame: &mut Frame,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if !static_frame.variables.contains_key(&key) {
            static_frame.declare(name, ty.clone(), array, option_base, span, types, enums)?;
        }
        let variable = static_frame.variable(name, span)?;
        self.declare_alias(name, ty, variable, span)
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
                cell: Rc::new(RefCell::new(coerce_assignment(&ty, value, span)?)),
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
        types: &HashMap<String, RuntimeType>,
        enums: &HashMap<String, RuntimeEnum>,
    ) -> Result<(), Diagnostic> {
        self.declare(name, ty.clone(), array, option_base, span, types, enums)?;
        let variable = self.variables.get_mut(&key(name)).expect("declared");
        variable.module_level = true;
        variable.is_const = is_const;
        if let Some(value) = value {
            *variable.cell.borrow_mut() = coerce_assignment(&ty, value, span)?;
        }
        Ok(())
    }

    pub(crate) fn declare_alias(
        &mut self,
        name: &str,
        ty: TypeName,
        variable: Variable,
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
        if !variable.ty.same_type(&ty) {
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
                ty: TypeName::User(class_name.to_string()),
                cell: Rc::new(RefCell::new(Value::Object(object))),
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
        let variable = self.variables.get_mut(&key(name)).ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Variable '{}' is not declared", name),
                Some(span),
            )
            .with_primary_label("unknown variable")
            .with_help("declare the variable before using it")
        })?;
        if variable.is_const {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::INVALID_ASSIGNMENT,
                format!("Constant '{}' cannot be assigned", name),
                Some(span),
            )
            .with_primary_label("assignment to constant")
            .with_help("remove the assignment or use a non-Const variable"));
        }

        let old = variable.cell.borrow().clone();
        *variable.cell.borrow_mut() = coerce_assignment(&variable.ty, value, span)?;
        Ok(old)
    }

    pub(crate) fn assign_missing(&mut self, name: &str, span: Span) -> Result<(), Diagnostic> {
        let variable = self.variables.get_mut(&key(name)).ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Variable '{}' is not declared", name),
                Some(span),
            )
        })?;
        *variable.cell.borrow_mut() = Value::Missing;
        Ok(())
    }

    pub(crate) fn get(&self, name: &str, span: Span) -> Result<Value, Diagnostic> {
        self.variables
            .get(&key(name))
            .map(|variable| variable.cell.borrow().clone())
            .ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(span),
                )
                .with_primary_label("unknown variable")
                .with_help("declare the variable before using it")
            })
    }

    pub(crate) fn variable(&self, name: &str, span: Span) -> Result<Variable, Diagnostic> {
        self.variables.get(&key(name)).cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Variable '{}' is not declared", name),
                Some(span),
            )
            .with_primary_label("unknown variable")
            .with_help("declare the variable before using it")
        })
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
        let array = variable.cell.borrow();
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
        let mut array = variable.cell.borrow_mut();
        write_array_element(&mut array, indices, value, span)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn redim_array(
        &mut self,
        name: &str,
        new_bounds: Vec<ArrayBound>,
        preserve: bool,
        types: &HashMap<String, RuntimeType>,
        enums: &HashMap<String, RuntimeEnum>,
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
        let mut array = variable.cell.borrow_mut();
        redim_array(&mut array, new_bounds, preserve, types, enums, span)
    }

    pub(crate) fn erase_array(
        &mut self,
        name: &str,
        span: Span,
        types: &HashMap<String, RuntimeType>,
        enums: &HashMap<String, RuntimeEnum>,
    ) -> Result<(), Diagnostic> {
        let variable = self.variable(name, span)?;
        let mut array = variable.cell.borrow_mut();
        let Value::Array {
            element_type,
            elements,
            allocated,
            bounds,
            dynamic,
        } = &mut *array
        else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                "Erase target must be an array",
                Some(span),
            ));
        };
        if *dynamic {
            elements.clear();
            bounds.clear();
            *allocated = false;
        } else {
            for element in elements.iter_mut() {
                *element = default_value(element_type, types, enums, span)?;
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
    pub(crate) ty: TypeName,
    pub(crate) cell: Rc<RefCell<Value>>,
    pub(crate) dynamic_array: bool,
    pub(crate) is_const: bool,
    pub(crate) module_level: bool,
}
