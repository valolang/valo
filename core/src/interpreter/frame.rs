use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::{Diagnostic, ObjectValue, Span, TypeName, Value};
use crate::{Expr, ExprKind};

use super::arrays::{read_array_element, write_array_element};
use super::records::RuntimeType;
use super::values::{coerce_assignment, default_value, key};

#[derive(Debug, Default)]
pub(crate) struct Frame {
    variables: HashMap<String, Variable>,
}

impl Frame {
    pub(crate) fn declare(
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

    pub(crate) fn assign(
        &mut self,
        name: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let variable = self.variables.get_mut(&key(name)).ok_or_else(|| {
            Diagnostic::new(format!("Variable '{}' is not declared", name), Some(span))
        })?;

        *variable.cell.borrow_mut() = coerce_assignment(&variable.ty, value, span)?;
        Ok(())
    }

    pub(crate) fn get(&self, name: &str, span: Span) -> Result<Value, Diagnostic> {
        self.variables
            .get(&key(name))
            .map(|variable| variable.cell.borrow().clone())
            .ok_or_else(|| {
                Diagnostic::new(format!("Variable '{}' is not declared", name), Some(span))
            })
    }

    pub(crate) fn variable(&self, name: &str, span: Span) -> Result<Variable, Diagnostic> {
        self.variables.get(&key(name)).cloned().ok_or_else(|| {
            Diagnostic::new(format!("Variable '{}' is not declared", name), Some(span))
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

    pub(crate) fn simple_index_value(&self, expr: &Expr, span: Span) -> Result<i64, Diagnostic> {
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
pub(crate) struct Variable {
    pub(crate) ty: TypeName,
    pub(crate) cell: Rc<RefCell<Value>>,
}
