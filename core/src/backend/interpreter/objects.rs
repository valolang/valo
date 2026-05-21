use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::{ArrayValue, Diagnostic, EventBinding, ObjectValue, Span, TypeName, Value};
use crate::{ArrayDecl, ClassMember, Expr, ExprKind, Function, Procedure, PropertyKind};

use super::arrays::{array_element_mut, read_array_element, redim_array, write_array_element};
use super::frame::Variable;
use super::properties::{RuntimeProperty, RuntimePropertyAccessor};
use super::records::{RuntimeField, read_field_member, write_member};
use super::values::{default_value, key};
use super::{Frame, Interpreter};

impl Interpreter {
    fn default_field_value(
        &self,
        ty: &TypeName,
        array: &Option<ArrayDecl>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if let Some(array) = array {
            let mut elements = Vec::new();
            let mut bounds = Vec::new();
            let mut is_dynamic = false;
            let allocated = match array {
                ArrayDecl::Fixed(fixed_bounds) => {
                    let mut total_len: usize = 1;
                    for bound in fixed_bounds {
                        total_len *= (bound.upper - bound.lower + 1) as usize;
                        bounds.push(*bound);
                    }
                    for _ in 0..total_len {
                        elements.push(default_value(ty, &self.types, &self.enums, span)?);
                    }
                    true
                }
                ArrayDecl::Dynamic => {
                    is_dynamic = true;
                    false
                }
            };

            return Ok(Value::Array(Rc::new(crate::runtime::ArrayValue {
                element_type: ty.clone(),
                elements,
                bounds,
                allocated,
                dynamic: is_dynamic,
            })));
        }

        default_value(ty, &self.types, &self.enums, span)
    }

    pub(crate) fn raise_event(
        &mut self,
        name: &str,
        args: &[Expr],
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let source = frame.get("me", span)?;
        let Value::Object(source) = source else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "RaiseEvent is only valid inside class methods",
                Some(span),
            ));
        };
        let class_name = source.borrow().class_name.clone();
        let class = self
            .classes
            .get(&key(&class_name))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Class '{}' is not defined", class_name),
                    Some(span),
                )
            })?;
        if !class.events.contains_key(&key(name)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                format!("Class '{}' has no event '{}'", class.name, name),
                Some(span),
            ));
        }
        let mut values = Vec::new();
        for arg in args {
            values.push(self.eval_expr(arg, frame)?);
        }
        let bindings = source.borrow().event_bindings.clone();
        for binding in bindings {
            if binding.event_name.eq_ignore_ascii_case(name) {
                self.call_method_sub_values(
                    Value::Object(binding.target.clone()),
                    &binding.handler_name,
                    &values,
                    frame,
                    span,
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn new_object(
        &mut self,
        class_name: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let class_name = self.resolve_user_type_name(class_name, caller_frame, span)?;
        if let Some(type_def) = self.types.get(&key(&class_name)).cloned() {
            if !type_def.is_structure {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::INVALID_QUALIFIED_ACCESS,
                    format!("'{}' is not a Structure", class_name),
                    Some(span),
                ));
            }
            let record = default_value(
                &TypeName::User(type_def.name.clone()),
                &self.types,
                &self.enums,
                span,
            )?;
            if let Some(init) = type_def.subs.get("initialize").cloned() {
                let mut frame = Frame::default();
                frame.inherit_modules_from(caller_frame)?;
                if let Some((module_key, _)) = key(&type_def.name).split_once('.') {
                    frame.set_module_key(module_key.to_string());
                }
                frame.declare_const("tmp", TypeName::User(type_def.name.clone()), record, span)?;
                let variable = frame.variable("tmp", span)?;
                let mut init_frame = Frame::default();
                init_frame.inherit_modules_from(caller_frame)?;
                if let Some((module_key, _)) = key(&type_def.name).split_once('.') {
                    init_frame.set_module_key(module_key.to_string());
                }
                init_frame.declare_alias(
                    "me",
                    TypeName::User(type_def.name.clone()),
                    variable,
                    span,
                )?;
                self.bind_parameters(&init.params, args, caller_frame, &mut init_frame)?;
                self.scope_stack
                    .push(format!("{}.{}", type_def.name, init.name));
                let result = self.exec_block(&init.body, &mut init_frame);
                self.scope_stack.pop();
                match result? {
                    super::ControlFlow::Continue | super::ControlFlow::ExitSub => {}
                    super::ControlFlow::Return(_) => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::CONTROL_FLOW,
                            "Return is only allowed inside Function",
                            Some(init.span),
                        ));
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::CONTROL_FLOW,
                            "Exit statement escaped its block",
                            Some(span),
                        ));
                    }
                }
                return frame.get("tmp", span);
            }
            if !args.is_empty() {
                let mut constructed = record;
                let Value::Record(record_data) = &mut constructed else {
                    unreachable!();
                };
                let record_data = Rc::make_mut(record_data);
                let fields = &mut record_data.fields;
                if args.len() != type_def.fields.len() {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!("Structure '{}' has no Constructor", type_def.name),
                        Some(span),
                    ));
                }
                for (field, arg) in type_def.fields.iter().zip(args.iter()) {
                    let value = self.eval_expr(arg, caller_frame)?;
                    fields.insert(key(&field.name), value);
                }
                return Ok(constructed);
            }
            return Ok(record);
        }
        let class = self
            .classes
            .get(&key(&class_name))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::INVALID_QUALIFIED_ACCESS,
                    format!("'{}' is not a class", class_name),
                    Some(span),
                )
            })?;
        let mut fields = HashMap::new();
        for field in &class.fields {
            let field_ty = self.resolve_type_name(&field.ty, caller_frame, span)?;
            let value = self.default_field_value(&field_ty, &field.array, span)?;
            fields.insert(key(&field.name), value);
        }
        let object = Value::Object(Rc::new(RefCell::new(ObjectValue {
            class_name: class.name.clone(),
            fields,
            event_bindings: Vec::new(),
            terminated: false,
        })));
        if let Some(init) = class
            .subs
            .get("initialize")
            .or_else(|| class.subs.get("class_initialize"))
        {
            self.call_method_sub(object.clone(), &init.name, args, caller_frame, span)?;
        } else if !args.is_empty() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                format!("Class '{}' has no Initialize constructor", class.name),
                Some(span),
            ));
        }
        Ok(object)
    }

    pub(crate) fn initialize_shared_class_fields(&mut self, span: Span) -> Result<(), Diagnostic> {
        let classes: Vec<_> = self.classes.values().cloned().collect();
        for class in classes {
            if class.shared_fields.is_empty() {
                continue;
            }
            let mut fields = HashMap::new();
            for field in &class.shared_fields {
                let ty = self.resolve_type_name(&field.ty, &Frame::default(), span)?;
                let value = self.default_field_value(&ty, &field.array, span)?;
                fields.insert(key(&field.name), value);
            }
            self.shared_class_fields.insert(key(&class.name), fields);
        }
        Ok(())
    }

    pub(crate) fn read_member(
        &mut self,
        value: &Value,
        member: &str,
        frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if object_has_field(value, member) {
            return read_field_member(value, member, span);
        }
        if matches!(value, Value::Nothing) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Object reference is Nothing",
                Some(span),
            )
            .with_primary_label("attempted to access a member on Nothing")
            .with_help("assign an object before accessing its members"));
        }
        if matches!(value, Value::Object(_)) {
            return self.call_property_get(value.clone(), member, &[], frame, span);
        }
        match read_field_member(value, member, span) {
            Ok(value) => Ok(value),
            Err(error) if matches!(value, Value::Record(_)) => {
                if let Value::Record(record) = value
                    && self
                        .types
                        .get(&key(&record.type_name))
                        .is_some_and(|type_def| type_def.properties.contains_key(&key(member)))
                {
                    return self.call_record_property_get(value.clone(), member, &[], frame, span);
                }
                Err(error)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn read_shared_member(
        &mut self,
        class_name: &str,
        member: &str,
        frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let class_name = self.resolve_user_type_name(class_name, frame, span)?;
        if let Some(fields) = self.shared_class_fields.get(&key(&class_name))
            && let Some(value) = fields.get(&key(member))
        {
            return Ok(value.clone());
        }
        let class = self.classes.get(&key(&class_name)).ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Class '{}' is not defined", class_name),
                Some(span),
            )
        })?;
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!("Class '{}' has no Shared field '{}'", class.name, member),
            Some(span),
        ))
    }

    pub(crate) fn assign_member(
        &mut self,
        target: &Expr,
        member: &str,
        value: Value,
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match &target.kind {
            ExprKind::Variable(name) => {
                if let Ok(module_key) = self.resolve_module_qualifier(name, frame, span) {
                    if frame.module_key() != Some(module_key.as_str())
                        && !self
                            .public_values
                            .get(&module_key)
                            .is_some_and(|values| values.contains(&key(member)))
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::PRIVATE_ACCESS,
                            format!("Module member '{}.{}' is Private", name, member),
                            Some(span),
                        ));
                    }
                    let module_frame =
                        self.module_frames.get_mut(&module_key).ok_or_else(|| {
                            Diagnostic::new(
                                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                                format!("Module '{}' is not loaded", name),
                                Some(span),
                            )
                        })?;
                    let old = module_frame.assign(member, value, span)?;
                    return self.maybe_terminate(old, span);
                }
                let variable = frame.variable(name, target.span)?;
                self.assign_member_to_variable(variable, member, value, span)
            }
            ExprKind::Me => {
                let variable = frame.variable("me", target.span)?;
                self.assign_member_to_variable(variable, member, value, span)
            }
            ExprKind::Call { name, args } => {
                let mut indices = Vec::new();
                for arg in args {
                    indices.push(frame.simple_index_value(arg, span)?);
                }
                if !frame.has_variable(name) {
                    let owner = frame.get("me", span)?;
                    return self.assign_member_to_bare_class_field_array_element(
                        owner, name, &indices, member, value, span,
                    );
                }
                let variable = frame.variable(name, target.span)?;
                let mut root = variable.borrow_mut();
                let element = array_element_mut(&mut root, &indices, span)?;
                if object_has_field(element, member) || !matches!(element, Value::Object(_)) {
                    let old = write_member(element, member, value, span)?;
                    self.maybe_terminate(old, span)?;
                    return Ok(());
                }
                let object = element.clone();
                drop(root);
                self.call_property_set(object, member, value, span)
            }
            ExprKind::MemberAccess { .. } | ExprKind::MemberCall { .. } | ExprKind::New { .. } => {
                let target_value = self.eval_expr(target, frame)?;
                self.assign_member_to_value(target_value, member, value, span)
            }
            _ => {
                let target_value = self.eval_expr(target, frame)?;
                self.assign_member_to_value(target_value, member, value, span)
            }
        }
    }

    pub(crate) fn assign_member_to_variable(
        &mut self,
        variable: Variable,
        member: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let mut root = variable.borrow_mut();
        if object_has_field(&root, member) || !matches!(&*root, Value::Object(_)) {
            if let Value::Record(record) = &*root
                && !record.fields.contains_key(&key(member))
                && self
                    .types
                    .get(&key(&record.type_name))
                    .is_some_and(|type_def| type_def.properties.contains_key(&key(member)))
            {
                drop(root);
                return self.call_record_property_set(variable, member, value, span);
            }
            return self.write_object_member(&mut root, member, value, span);
        }
        let object = root.clone();
        drop(root);
        self.call_property_set(object, member, value, span)
    }

    pub(crate) fn assign_member_to_value(
        &mut self,
        mut target: Value,
        member: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if object_has_field(&target, member) || !matches!(target, Value::Object(_)) {
            return self.write_object_member(&mut target, member, value, span);
        }
        self.call_property_set(target, member, value, span)
    }

    pub(crate) fn assign_bare_class_field(
        &mut self,
        owner: Value,
        field: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let mut owner_value = owner;
        self.write_object_member(&mut owner_value, field, value, span)
    }

    pub(crate) fn read_bare_class_field_array_element(
        &mut self,
        owner: Value,
        field: &str,
        indices: &[i64],
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let field_value = read_field_member(&owner, field, span)?;
        read_array_element(&field_value, indices, span)
    }

    pub(crate) fn assign_bare_class_field_array_element(
        &mut self,
        owner: Value,
        field: &str,
        indices: &[i64],
        value: Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let Value::Object(object) = owner else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Class field assignment requires an object",
                Some(span),
            ));
        };
        let mut object = object.borrow_mut();
        let Some(slot) = object.fields.get_mut(&key(field)) else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Class '{}' has no field '{}'", object.class_name, field),
                Some(span),
            ));
        };
        write_array_element(slot, indices, value, span)
    }

    pub(crate) fn assign_member_to_bare_class_field_array_element(
        &mut self,
        owner: Value,
        field: &str,
        indices: &[i64],
        member: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let Value::Object(object) = owner else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Class field assignment requires an object",
                Some(span),
            ));
        };
        let mut object_ref = object.borrow_mut();
        let class_name = object_ref.class_name.clone();
        let Some(slot) = object_ref.fields.get_mut(&key(field)) else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Class '{}' has no field '{}'", class_name, field),
                Some(span),
            ));
        };
        let element = array_element_mut(slot, indices, span)?;
        if object_has_field(element, member) || !matches!(element, Value::Object(_)) {
            let old = write_member(element, member, value, span)?;
            drop(object_ref);
            self.maybe_terminate(old, span)?;
            return Ok(());
        }
        let target = element.clone();
        drop(object_ref);
        self.call_property_set(target, member, value, span)
    }

    pub(crate) fn erase_member_array(
        &mut self,
        object: &Value,
        field: &str,
        span: Span,
        _frame: &mut Frame,
    ) -> Result<(), Diagnostic> {
        let Value::Object(object_ref) = object else {
            if let Value::Nothing = object {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    "Object reference is Nothing",
                    Some(span),
                ));
            }
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Erase member target must be an object",
                Some(span),
            ));
        };
        let mut object_mut = object_ref.borrow_mut();
        let class_name = object_mut.class_name.clone();
        let Some(slot) = object_mut.fields.get_mut(&key(field)) else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Class '{}' has no field '{}'", class_name, field),
                Some(span),
            ));
        };
        let Value::Array(array) = slot else {
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
                *element = default_value(&array.element_type, &self.types, &self.enums, span)?;
            }
        }
        Ok(())
    }

    pub(crate) fn redim_target(
        &mut self,
        target: &crate::ReDimTarget,
        new_bounds: Vec<crate::runtime::ArrayBound>,
        preserve: bool,
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match target {
            crate::ReDimTarget::Variable { name, .. } => {
                if frame.has_variable(name) {
                    frame.redim_array(name, new_bounds, preserve, &self.types, &self.enums, span)
                } else {
                    let owner = frame.get("me", span)?;
                    self.redim_value_member(owner, name, new_bounds, preserve, span)
                }
            }
            crate::ReDimTarget::Member { object, field, .. } => {
                let target_value = self.eval_expr(object, frame)?;
                self.redim_value_member(target_value, field, new_bounds, preserve, span)
            }
        }
    }

    fn redim_value_member(
        &mut self,
        target: Value,
        field: &str,
        new_bounds: Vec<crate::runtime::ArrayBound>,
        preserve: bool,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let Value::Object(object) = target else {
            if matches!(target, Value::Nothing) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    "Object reference is Nothing",
                    Some(span),
                ));
            }
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "ReDim member target requires an object",
                Some(span),
            ));
        };
        let mut object = object.borrow_mut();
        let class_name = object.class_name.clone();
        let Some(slot) = object.fields.get_mut(&key(field)) else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Class '{}' has no field '{}'", class_name, field),
                Some(span),
            ));
        };
        if matches!(slot, Value::Empty | Value::Null | Value::Missing) {
            *slot = Value::Array(Rc::new(ArrayValue {
                element_type: TypeName::Variant,
                elements: Vec::new(),
                bounds: Vec::new(),
                allocated: false,
                dynamic: true,
            }));
        }
        redim_array(slot, new_bounds, preserve, &self.types, &self.enums, span)
    }

    fn write_object_member(
        &mut self,
        target: &mut Value,
        member: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let owner = match target {
            Value::Object(object) if object.borrow().fields.contains_key(&key(member)) => {
                Some(object.clone())
            }
            _ => None,
        };
        let old_value = write_member(target, member, value, span)?;
        if let Some(owner) = owner {
            let new_value = owner
                .borrow()
                .fields
                .get(&key(member))
                .cloned()
                .unwrap_or(Value::Nothing);
            self.rebind_withevents_field(owner, member, &old_value, &new_value);
        }
        self.maybe_terminate(old_value, span)?;
        Ok(())
    }
}

pub(crate) fn object_has_field(value: &Value, field: &str) -> bool {
    if let Value::Object(object) = value {
        return object.borrow().fields.contains_key(&key(field));
    }
    false
}

pub(crate) fn ensure_object(
    value: Value,
    span: Span,
) -> Result<Rc<RefCell<ObjectValue>>, Diagnostic> {
    match value {
        Value::Object(object) => Ok(object),
        Value::Nothing => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            "Object reference is Nothing",
            Some(span),
        )
        .with_primary_label("attempted to call a method on Nothing")
        .with_help("assign an object before calling its methods")),
        _ => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Method call requires an object",
            Some(span),
        )),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeClass {
    pub(crate) name: String,
    pub(crate) fields: Vec<RuntimeField>,
    pub(crate) shared_fields: Vec<RuntimeField>,
    pub(crate) constants: Vec<crate::ConstDecl>,
    pub(crate) events: HashMap<String, RuntimeEvent>,
    pub(crate) subs: HashMap<String, Procedure>,
    pub(crate) shared_subs: HashMap<String, Procedure>,
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) shared_functions: HashMap<String, Function>,
    pub(crate) iterator: Option<Function>,
    pub(crate) properties: HashMap<String, RuntimeProperty>,
    pub(crate) enumerator_member: Option<String>,
    pub(crate) default_member: Option<String>,
}

impl From<&crate::ClassDecl> for RuntimeClass {
    fn from(value: &crate::ClassDecl) -> Self {
        let mut fields = Vec::new();
        let mut shared_fields = Vec::new();
        let mut constants = Vec::new();
        let mut events = HashMap::new();
        let mut subs = HashMap::new();
        let mut shared_subs = HashMap::new();
        let mut functions = HashMap::new();
        let mut shared_functions = HashMap::new();
        let mut iterator = None;
        let mut properties = HashMap::new();
        let mut enumerator_member = None;
        let mut default_member = None;
        for member in &value.members {
            match member {
                ClassMember::Field(field) => {
                    let target = if field.is_shared {
                        &mut shared_fields
                    } else {
                        &mut fields
                    };
                    target.push(RuntimeField {
                        name: field.name.clone(),
                        ty: field
                            .ty
                            .clone()
                            .unwrap_or(crate::runtime::TypeName::Variant),
                        array: field.array.clone(),
                        initializer: field.initializer.clone(),
                        with_events: field.with_events,
                    });
                }
                ClassMember::Fields(class_fields) => {
                    for field in class_fields {
                        let target = if field.is_shared {
                            &mut shared_fields
                        } else {
                            &mut fields
                        };
                        target.push(RuntimeField {
                            name: field.name.clone(),
                            ty: field
                                .ty
                                .clone()
                                .unwrap_or(crate::runtime::TypeName::Variant),
                            array: field.array.clone(),
                            initializer: field.initializer.clone(),
                            with_events: field.with_events,
                        });
                    }
                }
                ClassMember::Const(const_decl) => constants.push(const_decl.clone()),
                ClassMember::Event(event) => {
                    events.insert(
                        key(&event.name),
                        RuntimeEvent {
                            name: event.name.clone(),
                        },
                    );
                }
                ClassMember::Sub(method) => {
                    if method.is_shared {
                        shared_subs.insert(key(&method.procedure.name), method.procedure.clone());
                    } else {
                        subs.insert(key(&method.procedure.name), method.procedure.clone());
                    }
                }
                ClassMember::Function(method) => {
                    if method.is_enumerator {
                        enumerator_member = Some(method.function.name.clone());
                    }
                    if method.function.is_iterator && method.function.params.is_empty() {
                        iterator = Some(method.function.clone());
                    }
                    if method.is_shared {
                        shared_functions
                            .insert(key(&method.function.name), method.function.clone());
                    } else {
                        functions.insert(key(&method.function.name), method.function.clone());
                    }
                }
                ClassMember::Iterator(method) => {
                    iterator = Some(method.function.clone());
                }
                ClassMember::Property(property) => {
                    if property.is_default {
                        default_member = Some(property.name.clone());
                    }
                    if property.is_enumerator {
                        enumerator_member = Some(property.name.clone());
                    }
                    if property.is_iterator
                        && property.params.is_empty()
                        && property.kind == PropertyKind::Get
                    {
                        iterator = Some(crate::Function {
                            visibility: property.visibility,
                            name: property.name.clone(),
                            is_iterator: true,
                            params: property.params.clone(),
                            return_type: property.return_type.clone().expect("get returns"),
                            return_slot: None,
                            body: property.body.clone(),
                            span: property.span,
                        });
                    }
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
            shared_fields,
            constants,
            events,
            subs,
            shared_subs,
            functions,
            shared_functions,
            iterator,
            properties,
            enumerator_member,
            default_member,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeEvent {
    pub(crate) name: String,
}

impl Interpreter {
    pub(crate) fn rebind_withevents_field(
        &mut self,
        owner: Rc<RefCell<ObjectValue>>,
        field: &str,
        old_value: &Value,
        value: &Value,
    ) {
        let owner_class_name = owner.borrow().class_name.clone();
        let Some(owner_class) = self.classes.get(&key(&owner_class_name)).cloned() else {
            return;
        };
        let Some(field_sig) = owner_class
            .fields
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(field) && candidate.with_events)
        else {
            return;
        };
        if let Value::Object(source) = old_value {
            source.borrow_mut().event_bindings.retain(|binding| {
                !(Rc::ptr_eq(&binding.target, &owner)
                    && binding
                        .handler_name
                        .to_ascii_lowercase()
                        .starts_with(&format!("{}_", field_sig.name.to_ascii_lowercase())))
            });
        }
        let Value::Object(source) = value else {
            return;
        };
        let source_class_name = source.borrow().class_name.clone();
        let Some(source_class) = self.classes.get(&key(&source_class_name)) else {
            return;
        };
        let mut bindings = Vec::new();
        for event in source_class.events.values() {
            let handler_name = format!("{}_{}", field_sig.name, event.name);
            if owner_class.subs.contains_key(&key(&handler_name)) {
                bindings.push(EventBinding {
                    event_name: event.name.clone(),
                    target: owner.clone(),
                    handler_name,
                });
            }
        }
        source.borrow_mut().event_bindings.extend(bindings);
    }
}
