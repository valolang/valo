use crate::runtime::well_known;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::ArrayDecl;
use crate::runtime::{
    ArrayBound, ArrayValue, Diagnostic, ObjectValue, Span, TypeName, Value, coerce_assignment,
};

use super::Interpreter;
use super::arrays::{read_array_element, redim_array, write_array_element};
use super::records::{RuntimeInterface, RuntimeType};
use super::values::{default_value, key, with_key};

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
                    with_key(member, |k| record.fields.get(k)).expect("field missing")
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
                    with_key(member, |k| Rc::make_mut(record).fields.get_mut(k))
                        .expect("field missing")
                } else {
                    panic!("Expected record in VariableCell::Member");
                }
            }),
        }
    }
}

/// Returns the value that assignment must hand back so the caller can run
/// deterministic termination on it.
///
/// Only object values have a `Class_Terminate` hook, so cloning anything else
/// (an array or record can be arbitrarily large) is pure waste.
/// Builds the frame key that holds a function's implicit return value.
pub(crate) fn return_slot_key(name: &str) -> String {
    crate::runtime::well_known::return_slot(name)
}

fn previous_for_termination(current: &Value) -> Value {
    match current {
        Value::Object(_) => current.clone(),
        _ => Value::Empty,
    }
}

#[derive(Debug, Default, Clone)]
pub struct Frame {
    variables: HashMap<String, Variable>,
    readonly_aliases: HashSet<String>,
    /// Whether `Me` is bound in this frame.
    ///
    /// Every assignment to a bare name has to consider that the name is a
    /// member of the enclosing instance. Outside a method there is no
    /// instance, and knowing that without a lookup is what keeps assigning to
    /// a local down to one.
    has_self: bool,
    /// What the running call will return, under the name it answers to.
    ///
    /// A frame belongs to one call, so there is at most one. Held in a map,
    /// every call cloned the slot name twice to write it and every assignment
    /// inside a function built a key to ask about it.
    return_slot: Option<(String, Value)>,
    module_key: Option<String>,
    class_context: Option<String>,
    with_stack: Vec<Value>,
    yielded_values: Option<Vec<Value>>,
    resume_next: bool,
    error_handler: Option<String>,
    handled_error_ip: Option<usize>,
}

impl Frame {
    pub(crate) fn mark_readonly_alias(&mut self, name: &str) {
        self.readonly_aliases.insert(key(name));
    }

    pub(crate) fn is_readonly_alias(&self, name: &str) -> bool {
        self.readonly_aliases.contains(&key(name))
    }

    fn reject_readonly_assignment(&self, name: &str, span: Span) -> Result<(), Diagnostic> {
        if self.is_readonly_alias(name) {
            Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::INVALID_ASSIGNMENT,
                format!("Cannot modify ByRef ReadOnly parameter '{name}'"),
                Some(span),
            ))
        } else {
            Ok(())
        }
    }
    pub(crate) fn set_return_slot(&mut self, slot: String, value: Value) {
        match &mut self.return_slot {
            Some((held, current)) if held == &slot => *current = value,
            _ => self.return_slot = Some((slot, value)),
        }
    }

    /// Writes the return slot belonging to `name`, without building its key.
    pub(crate) fn set_return_slot_for(&mut self, name: &str, value: Value) {
        match &mut self.return_slot {
            Some((slot, current)) if well_known::is_return_slot_for(slot, name) => *current = value,
            _ => self.return_slot = Some((return_slot_key(name), value)),
        }
    }

    pub(crate) fn get_return_slot(&self, slot: &str) -> Option<Value> {
        match &self.return_slot {
            Some((held, value)) if held == slot => Some(value.clone()),
            _ => None,
        }
    }

    /// Reports whether this frame holds the return slot that assigning to
    /// `name` would target.
    ///
    /// Every plain assignment asks this question, so short-circuit on the
    /// common case (a frame with no return slots at all) before paying for the
    /// `__return_<name>` key.
    /// Records a variable, keeping [`Frame::has_self`] in step.
    ///
    /// Every insertion goes through here so the flag cannot drift from what
    /// the map holds.
    /// Makes room for `extra` more variables in one go.
    pub(crate) fn reserve(&mut self, extra: usize) {
        self.variables.reserve(extra);
    }

    fn insert_variable(&mut self, key: String, variable: Variable) {
        if key == well_known::SELF_KEY {
            self.has_self = true;
        }
        self.variables.insert(key, variable);
    }

    /// Whether `Me` is bound here.
    pub(crate) fn has_self(&self) -> bool {
        self.has_self
    }

    /// Writes to a local if there is one, in a single lookup.
    ///
    /// Hands the value back when the name is not a local. The caller has other
    /// places to look, and returning the value means it does not have to be
    /// cloned for a path that is usually not taken.
    pub(crate) fn assign_if_local(
        &mut self,
        name: &str,
        value: Value,
        span: Span,
    ) -> Result<LocalAssign, Diagnostic> {
        self.reject_readonly_assignment(name, span)?;
        let Some(variable) = with_key(name, |k| self.variables.get_mut(k)) else {
            return Ok(LocalAssign::NoSuchLocal(value));
        };
        if variable.is_const {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::INVALID_ASSIGNMENT,
                format!("Constant '{}' cannot be assigned", name),
                Some(span),
            )
            .with_primary_label("assignment to constant")
            .with_help("remove the assignment or use a non-Const variable"));
        }
        let old = previous_for_termination(&variable.borrow());
        *variable.borrow_mut() = coerce_assignment(&variable.ty, value, span)?;
        Ok(LocalAssign::Written(old))
    }

    pub(crate) fn has_return_slot_for(&self, name: &str) -> bool {
        self.return_slot
            .as_ref()
            .is_some_and(|(slot, _)| well_known::is_return_slot_for(slot, name))
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
            .get(well_known::SELF_KEY)
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
        span: Span,
        interpreter: &Interpreter,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if let Some(existing) = self.variables.get(&key) {
            // The same declaration running again means we re-entered its block,
            // so rebind it fresh instead of rejecting it as a duplicate. A
            // captured variable belongs to an outer scope, so declaring the name
            // here shadows it rather than clashing with it.
            if existing.declared_at != Some(span) && !existing.captured {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                    format!("Variable '{}' is already declared", name),
                    Some(span),
                ));
            }
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

        self.insert_variable(
            key,
            Variable {
                name: name.to_string(),
                ty: ty.clone(),
                cell: VariableCell::Direct(Rc::new(RefCell::new(value))),
                dynamic_array,
                is_const: false,
                module_level: false,
                captured: false,
                declared_at: Some(span),
            },
        );
        Ok(())
    }

    /// Declares a parameter and gives it its argument in one step.
    ///
    /// Binding a parameter used to declare the name, which builds the default
    /// value for its type, and then assign the argument over it, which folded
    /// the name a second time and looked it up again. The default value is
    /// never read and the second lookup always finds what the first one just
    /// wrote, so a call was paying twice to bind each parameter once.
    pub(crate) fn declare_bound(
        &mut self,
        name: &str,
        ty: TypeName,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if let Some(existing) = self.variables.get(&key)
            && existing.declared_at != Some(span)
            && !existing.captured
        {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Variable '{}' is already declared", name),
                Some(span),
            ));
        }

        // A missing optional argument stays Missing, which is what tells
        // IsMissing about it; coercing it would turn it into a zero.
        let value = if matches!(value, Value::Missing) {
            value
        } else {
            coerce_assignment(&ty, value, span)?
        };
        self.insert_variable(
            key,
            Variable {
                name: name.to_string(),
                ty,
                cell: VariableCell::Direct(Rc::new(RefCell::new(value))),
                dynamic_array: false,
                is_const: false,
                module_level: false,
                captured: false,
                declared_at: Some(span),
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
        span: Span,
        interpreter: &Interpreter,
        static_frame: &mut Frame,
    ) -> Result<(), Diagnostic> {
        let key = key(name);
        if !static_frame.variables.contains_key(&key) {
            static_frame.declare(name, ty.clone(), array, span, interpreter)?;
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
        if let Some(existing) = self.variables.get(&key)
            && existing.declared_at != Some(span)
            && !existing.captured
        {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Variable '{}' is already declared", name),
                Some(span),
            ));
        }
        self.insert_variable(
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
                captured: false,
                declared_at: Some(span),
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
        is_const: bool,
        value: Option<Value>,
        span: Span,
        interpreter: &Interpreter,
    ) -> Result<(), Diagnostic> {
        self.declare(name, ty.clone(), array, span, interpreter)?;
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
                crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
                format!(
                    "ByRef argument type {} must match parameter type {}",
                    variable.ty.display_name(),
                    ty.display_name()
                ),
                Some(span),
            ));
        }

        self.insert_variable(key, variable);
        Ok(())
    }

    pub(crate) fn inherit_modules_from(&mut self, source: &Frame) -> Result<(), Diagnostic> {
        for (name, variable) in &source.variables {
            if variable.module_level && !self.variables.contains_key(name) {
                self.insert_variable(name.clone(), variable.clone());
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
        self.insert_variable(
            key,
            Variable {
                name: name.to_string(),
                ty: TypeName::User(class_name.to_string()),
                cell: VariableCell::Direct(Rc::new(RefCell::new(Value::Object(object)))),
                dynamic_array: false,
                is_const: false,
                module_level: false,
                captured: false,
                declared_at: Some(span),
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
        self.reject_readonly_assignment(name, span)?;
        let Some(variable) = with_key(name, |k| self.variables.get_mut(k)) else {
            return Err(self.unknown_variable(name, span));
        };
        if variable.is_const {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::INVALID_ASSIGNMENT,
                format!("Constant '{}' cannot be assigned", name),
                Some(span),
            )
            .with_primary_label("assignment to constant")
            .with_help("remove the assignment or use a non-Const variable"));
        }

        let old = previous_for_termination(&variable.borrow());
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

    /// The value held under a name, or `None` when there is no such variable.
    ///
    /// [`Frame::get`] builds a diagnostic when the name is unknown, and that
    /// diagnostic suggests a near-miss by comparing the name against every
    /// variable in scope. Callers that treat a miss as an ordinary answer, such
    /// as a call site checking whether a name holds a lambda, were paying for
    /// that on every hit as well as every miss.
    pub(crate) fn peek(&self, name: &str) -> Option<Value> {
        with_key(name, |k| self.variables.get(k)).map(|variable| variable.borrow().clone())
    }

    pub(crate) fn get(&self, name: &str, span: Span) -> Result<Value, Diagnostic> {
        with_key(name, |k| self.variables.get(k))
            .map(|variable| variable.borrow().clone())
            .ok_or_else(|| self.unknown_variable(name, span))
    }

    /// Binds a variable captured from the scope that defined a lambda.
    ///
    /// The cell is shared, not copied, so the lambda and the defining scope see
    /// each other's assignments.
    pub(crate) fn bind_captured(&mut self, captured: &crate::runtime::CapturedVariable) {
        self.insert_variable(
            key(&captured.name),
            Variable {
                name: captured.name.clone(),
                ty: captured.ty.clone(),
                cell: VariableCell::Direct(captured.cell.clone()),
                dynamic_array: false,
                is_const: false,
                module_level: false,
                captured: true,
                declared_at: None,
            },
        );
    }

    /// Lists this scope's variables so a lambda defined here can capture them.
    ///
    /// Only directly held variables are shared by reference; an alias to an
    /// array element or a record field is a view into another value, so it is
    /// captured by value instead.
    pub(crate) fn capture_environment(&self) -> Vec<crate::runtime::CapturedVariable> {
        self.variables
            .values()
            .map(|variable| crate::runtime::CapturedVariable {
                name: variable.name.clone(),
                ty: variable.ty.clone(),
                cell: match &variable.cell {
                    VariableCell::Direct(cell) => cell.clone(),
                    view => Rc::new(RefCell::new(view.borrow().clone())),
                },
            })
            .collect()
    }

    /// Returns the variable's value only when it is an object-like target that
    /// indexing could dispatch to a default member.
    ///
    /// Indexed assignment needs to know whether `name(i) = x` means "write an
    /// array slot" or "invoke a default property". Cloning the value to find out
    /// would copy the entire array on every element write, so clone only the
    /// object cases, where a clone is just a refcount bump.
    pub(crate) fn get_if_indexable_object(
        &self,
        name: &str,
        span: Span,
    ) -> Result<Option<Value>, Diagnostic> {
        let variable = self
            .variable_ref(name)
            .ok_or_else(|| self.unknown_variable(name, span))?;
        let current = variable.borrow();
        Ok(match &*current {
            Value::Object(_) => Some(current.clone()),
            _ => None,
        })
    }

    /// Borrows a variable in place, skipping the `Variable` clone that `variable`
    /// performs. Prefer this wherever the caller only needs to read or borrow.
    pub(crate) fn variable_ref(&self, name: &str) -> Option<&Variable> {
        with_key(name, |k| self.variables.get(k))
    }

    pub(crate) fn variable(&self, name: &str, span: Span) -> Result<Variable, Diagnostic> {
        with_key(name, |k| self.variables.get(k))
            .cloned()
            .ok_or_else(|| self.unknown_variable(name, span))
    }

    pub(crate) fn remove_variable(&mut self, name: &str) -> Option<Variable> {
        let removed = with_key(name, |k| self.variables.remove(k));
        if removed.is_some() && with_key(name, |k| k == well_known::SELF_KEY) {
            self.has_self = false;
        }
        removed
    }

    /// Whether a name is a variable that currently holds an array.
    ///
    /// `f(1)` is indexing when `f` is an array and a call when `f` holds
    /// something callable, and only the value tells the two apart.
    pub(crate) fn holds_array(&self, name: &str) -> bool {
        self.variable_ref(name)
            .is_some_and(|variable| matches!(&*variable.borrow(), Value::Array(_)))
    }

    pub(crate) fn has_variable(&self, name: &str) -> bool {
        with_key(name, |k| self.variables.contains_key(k))
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
        let variable = self
            .variable_ref(name)
            .ok_or_else(|| self.unknown_variable(name, span))?;
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
    /// True when this variable came from the scope that defined a lambda,
    /// rather than being declared in this one.
    ///
    /// A declaration inside the lambda shadows the captured variable instead of
    /// colliding with it, which is how an inner scope is expected to behave.
    pub(crate) captured: bool,
    /// Source span of the declaration that introduced this variable.
    ///
    /// Re-executing the same `Dim` (a declaration inside a loop body) must
    /// reinitialize the variable rather than report a duplicate, which is how
    /// VB.NET block scoping behaves. Comparing spans distinguishes that from a
    /// genuine second declaration elsewhere in the same scope.
    pub(crate) declared_at: Option<Span>,
}

impl Variable {
    pub fn borrow(&self) -> Ref<'_, Value> {
        self.cell.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, Value> {
        self.cell.borrow_mut()
    }
}

/// What [`Frame::assign_if_local`] did.
pub(crate) enum LocalAssign {
    /// Written. Carries what was there, which may need terminating.
    Written(Value),
    /// There is no local by that name; the value is handed back untouched.
    NoSuchLocal(Value),
}
