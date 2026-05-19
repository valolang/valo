use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::{Diagnostic, ObjectValue, Span, TypeName, Value};
use crate::{ArrayDecl, Expr, ExprKind};

use super::arrays::{read_array_element, redim_array, write_array_element};
use super::records::{RuntimeEnum, RuntimeType};
use super::values::{coerce_assignment, default_value, key};

#[derive(Debug, Default)]
pub(crate) struct Frame {
    variables: HashMap<String, Variable>,
    with_stack: Vec<Value>,
    resume_next: bool,
    error_handler: Option<String>,
    handled_error_ip: Option<usize>,
}

impl Frame {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn declare(
        &mut self,
        name: &str,
        ty: TypeName,
        array: Option<ArrayDecl>,
        option_base: i64,
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
            let allocated = match array {
                ArrayDecl::Fixed(upper_bound) => {
                    if upper_bound < option_base {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::OPTION,
                            "Array upper bound must be greater than or equal to Option Base",
                            Some(span),
                        ));
                    }
                    for _ in option_base..=upper_bound {
                        elements.push(default_value(&ty, types, enums, span)?);
                    }
                    true
                }
                ArrayDecl::Dynamic => false,
            };
            Value::Array {
                element_type: ty.clone(),
                elements,
                lower_bound: option_base,
                allocated,
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

    pub(crate) fn assign(&mut self, name: &str, value: Value, span: Span) -> Result<(), Diagnostic> {
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

        *variable.cell.borrow_mut() = coerce_assignment(&variable.ty, value, span)?;
        Ok(())
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

    pub(crate) fn has_variable(&self, name: &str) -> bool {
        self.variables.contains_key(&key(name))
    }

    pub(crate) fn get_array_element(
        &self,
        name: &str,
        index: i64,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let variable = self.variable(name, span)?;
        let array = variable.cell.borrow();
        read_array_element(&array, index, span)
    }

    pub(crate) fn assign_array_element(
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn redim_array(
        &mut self,
        name: &str,
        upper_bound: i64,
        option_base: i64,
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
        redim_array(
            &mut array,
            upper_bound,
            option_base,
            preserve,
            types,
            enums,
            span,
        )
    }

    pub(crate) fn simple_index_value(&self, expr: &Expr, span: Span) -> Result<i64, Diagnostic> {
        match &expr.kind {
            ExprKind::Integer(value) => Ok(*value),
            ExprKind::Variable(name) => match self.get(name, expr.span)? {
                Value::Integer(value) => Ok(value),
                _ => Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    "Array index must be Integer",
                    Some(span),
                )),
            },
            _ => Err(Diagnostic::new(crate::runtime::DiagnosticCode::ARRAY, "Array member assignment index must be an Integer literal or variable", Some(span),)),
        }
    }

    pub(crate) fn push_with_target(&mut self, value: Value) {
        self.with_stack.push(value);
    }

    pub(crate) fn pop_with_target(&mut self) {
        self.with_stack.pop();
    }

    pub(crate) fn current_with_target(&self, span: Span) -> Result<Value, Diagnostic> {
        self.with_stack.last().cloned().ok_or_else(|| {
            Diagnostic::new(crate::runtime::DiagnosticCode::TYPE_MISMATCH, "Dot member access requires an active With block", Some(span),)
            .with_primary_label("no active With target")
            .with_help("use dotted member access only inside a With block")
        })
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
