use std::collections::HashMap;

use crate::runtime::{Diagnostic, TypeName};
use crate::{
    AssignTarget, BinaryOp, CaseCompareOp, CaseItem, ClassMember, DoLoopCondition, ExitTarget,
    Expr, ExprKind, Function, Parameter, PassingMode, Procedure, Program, PropertyKind, Stmt,
    UnaryOp, Visibility,
};

use crate::semantics::context::Context;
use crate::semantics::symbols::{CallableSig, ParamSig, Signatures, VarType, key};
use crate::semantics::types::{
    ClassFieldSig, ClassMethodSig, ClassPropertySig, ClassSig, EnumSig, FieldSig,
    PropertyAccessorSig, TypeRegistry, TypeSig,
};

#[derive(Debug, Clone, Copy, Default)]
struct LoopContext {
    for_depth: usize,
    while_depth: usize,
    do_depth: usize,
}

impl LoopContext {
    fn in_for(mut self) -> Self {
        self.for_depth += 1;
        self
    }

    fn in_while(mut self) -> Self {
        self.while_depth += 1;
        self
    }

    fn in_do(mut self) -> Self {
        self.do_depth += 1;
        self
    }
}

pub fn validate(program: &Program) -> Result<(), Diagnostic> {
    let types = collect_types(program)?;
    let signatures = collect_signatures(program, &types)?;
    let Some(main) = program
        .procedures
        .iter()
        .find(|procedure| procedure.name.eq_ignore_ascii_case("main"))
    else {
        return Err(Diagnostic::new("Program must contain Sub Main()", None));
    };

    if !main.params.is_empty() {
        return Err(Diagnostic::new(
            "Sub Main() cannot have parameters",
            Some(main.span),
        ));
    }

    for procedure in &program.procedures {
        validate_procedure(procedure, &types, &signatures)?;
    }

    for function in &program.functions {
        validate_function(function, &types, &signatures)?;
    }
    for class_decl in &program.classes {
        validate_class(class_decl, &types, &signatures)?;
    }

    Ok(())
}

fn validate_class(
    class_decl: &crate::ClassDecl,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<(), Diagnostic> {
    for member in &class_decl.members {
        match member {
            ClassMember::Field(_) => {}
            ClassMember::Sub(method) => {
                let mut symbols = HashMap::new();
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&method.procedure.params, &mut symbols)?;
                validate_statements(
                    &method.procedure.body,
                    &mut symbols,
                    types,
                    signatures,
                    Context::MethodSub {
                        class_name: class_decl.name.clone(),
                    },
                    LoopContext::default(),
                )?;
            }
            ClassMember::Function(method) => {
                let mut symbols = HashMap::new();
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&method.function.params, &mut symbols)?;
                let mut saw_return = false;
                validate_statements(
                    &method.function.body,
                    &mut symbols,
                    types,
                    signatures,
                    Context::MethodFunction {
                        class_name: class_decl.name.clone(),
                        return_type: method.function.return_type.clone(),
                        saw_return: &mut saw_return,
                    },
                    LoopContext::default(),
                )?;
                if !saw_return {
                    return Err(Diagnostic::new(
                        format!("Function '{}' must return a value", method.function.name),
                        Some(method.function.span),
                    ));
                }
            }
            ClassMember::Property(property) => {
                let mut symbols = HashMap::new();
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&property.params, &mut symbols)?;
                match property.kind {
                    PropertyKind::Get => {
                        let return_type = property
                            .return_type
                            .clone()
                            .expect("property get return type");
                        let mut saw_return = false;
                        validate_statements(
                            &property.body,
                            &mut symbols,
                            types,
                            signatures,
                            Context::PropertyGet {
                                class_name: class_decl.name.clone(),
                                return_type,
                                saw_return: &mut saw_return,
                            },
                            LoopContext::default(),
                        )?;
                        if !saw_return {
                            return Err(Diagnostic::new(
                                format!("Property Get '{}' must return a value", property.name),
                                Some(property.span),
                            ));
                        }
                    }
                    PropertyKind::Let | PropertyKind::Set => {
                        validate_statements(
                            &property.body,
                            &mut symbols,
                            types,
                            signatures,
                            Context::PropertyLetSet {
                                class_name: class_decl.name.clone(),
                            },
                            LoopContext::default(),
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn collect_types(program: &Program) -> Result<TypeRegistry, Diagnostic> {
    let mut types = HashMap::new();

    for type_decl in &program.types {
        let type_key = key(&type_decl.name);
        if types.contains_key(&type_key) {
            return Err(Diagnostic::new(
                format!("Type '{}' is already defined", type_decl.name),
                Some(type_decl.span),
            ));
        }

        let mut fields = HashMap::new();
        for field in &type_decl.fields {
            let field_key = key(&field.name);
            if fields.contains_key(&field_key) {
                return Err(Diagnostic::new(
                    format!(
                        "Field '{}' is already declared in Type '{}'",
                        field.name, type_decl.name
                    ),
                    Some(field.span),
                ));
            }
            fields.insert(
                field_key,
                FieldSig {
                    ty: field.ty.clone(),
                },
            );
        }

        types.insert(
            type_key,
            TypeSig {
                name: type_decl.name.clone(),
                fields,
            },
        );
    }

    let mut enums = HashMap::new();
    for enum_decl in &program.enums {
        let enum_key = key(&enum_decl.name);
        if types.contains_key(&enum_key) || enums.contains_key(&enum_key) {
            return Err(Diagnostic::new(
                format!("Enum '{}' is already defined", enum_decl.name),
                Some(enum_decl.span),
            ));
        }
        let mut members = HashMap::new();
        let mut previous = -1;
        for member in &enum_decl.members {
            let member_key = key(&member.name);
            if members.contains_key(&member_key) {
                return Err(Diagnostic::new(
                    format!(
                        "Enum member '{}' is already declared in Enum '{}'",
                        member.name, enum_decl.name
                    ),
                    Some(member.span),
                ));
            }
            let value = if let Some(expr) = &member.value {
                eval_enum_const_expr(expr, &members)?
            } else {
                previous + 1
            };
            previous = value;
            members.insert(member_key, value);
        }
        enums.insert(
            enum_key,
            EnumSig {
                name: enum_decl.name.clone(),
                members,
            },
        );
    }

    let mut classes = HashMap::new();
    for class_decl in &program.classes {
        let class_key = key(&class_decl.name);
        if types.contains_key(&class_key)
            || enums.contains_key(&class_key)
            || classes.contains_key(&class_key)
        {
            return Err(Diagnostic::new(
                format!("Class '{}' is already defined", class_decl.name),
                Some(class_decl.span),
            ));
        }

        let mut fields = HashMap::new();
        let mut subs = HashMap::new();
        let mut functions = HashMap::new();
        let mut properties: HashMap<String, ClassPropertySig> = HashMap::new();
        for member in &class_decl.members {
            match member {
                ClassMember::Field(field) => {
                    let field_key = key(&field.name);
                    if fields.contains_key(&field_key) || properties.contains_key(&field_key) {
                        return Err(Diagnostic::new(
                            format!(
                                "Field '{}' conflicts with another member in Class '{}'",
                                field.name, class_decl.name
                            ),
                            Some(field.span),
                        ));
                    }
                    fields.insert(
                        field_key,
                        ClassFieldSig {
                            visibility: field.visibility,
                            ty: field.ty.clone(),
                        },
                    );
                }
                ClassMember::Sub(method) => {
                    let method_key = key(&method.procedure.name);
                    if subs.contains_key(&method_key)
                        || functions.contains_key(&method_key)
                        || properties.contains_key(&method_key)
                    {
                        return Err(Diagnostic::new(
                            format!(
                                "Method '{}' conflicts with another member in Class '{}'",
                                method.procedure.name, class_decl.name
                            ),
                            Some(method.procedure.span),
                        ));
                    }
                    subs.insert(
                        method_key,
                        ClassMethodSig {
                            visibility: method.visibility,
                            name: method.procedure.name.clone(),
                            params: params_to_sigs(&method.procedure.params),
                            return_type: None,
                        },
                    );
                }
                ClassMember::Function(method) => {
                    let method_key = key(&method.function.name);
                    if subs.contains_key(&method_key)
                        || functions.contains_key(&method_key)
                        || properties.contains_key(&method_key)
                    {
                        return Err(Diagnostic::new(
                            format!(
                                "Method '{}' conflicts with another member in Class '{}'",
                                method.function.name, class_decl.name
                            ),
                            Some(method.function.span),
                        ));
                    }
                    functions.insert(
                        method_key,
                        ClassMethodSig {
                            visibility: method.visibility,
                            name: method.function.name.clone(),
                            params: params_to_sigs(&method.function.params),
                            return_type: Some(method.function.return_type.clone()),
                        },
                    );
                }
                ClassMember::Property(property) => {
                    let property_key = key(&property.name);
                    if fields.contains_key(&property_key)
                        || subs.contains_key(&property_key)
                        || functions.contains_key(&property_key)
                    {
                        return Err(Diagnostic::new(
                            format!(
                                "Property '{}' conflicts with another member in Class '{}'",
                                property.name, class_decl.name
                            ),
                            Some(property.span),
                        ));
                    }
                    let property_sig =
                        properties
                            .entry(property_key)
                            .or_insert_with(|| ClassPropertySig {
                                name: property.name.clone(),
                                get: None,
                                let_: None,
                                set: None,
                            });
                    let target = match property.kind {
                        PropertyKind::Get => &mut property_sig.get,
                        PropertyKind::Let => &mut property_sig.let_,
                        PropertyKind::Set => &mut property_sig.set,
                    };
                    if target.is_some() {
                        return Err(Diagnostic::new(
                            format!(
                                "Property {:?} '{}' is already declared in Class '{}'",
                                property.kind, property.name, class_decl.name
                            ),
                            Some(property.span),
                        ));
                    }
                    *target = Some(PropertyAccessorSig {
                        visibility: property.visibility,
                        params: params_to_sigs(&property.params),
                        return_type: property.return_type.clone(),
                    });
                }
            }
        }
        classes.insert(
            class_key,
            ClassSig {
                name: class_decl.name.clone(),
                fields,
                subs,
                functions,
                properties,
            },
        );
    }

    let registry = TypeRegistry {
        types,
        enums,
        classes,
    };
    for type_decl in &program.types {
        for field in &type_decl.fields {
            ensure_known_type(&field.ty, &registry, field.span)?;
        }
    }
    for class_decl in &program.classes {
        for member in &class_decl.members {
            match member {
                ClassMember::Field(field) => ensure_known_type(&field.ty, &registry, field.span)?,
                ClassMember::Sub(method) => {
                    for param in &method.procedure.params {
                        ensure_known_type(&param.ty, &registry, param.span)?;
                    }
                }
                ClassMember::Function(method) => {
                    ensure_known_type(
                        &method.function.return_type,
                        &registry,
                        method.function.span,
                    )?;
                    for param in &method.function.params {
                        ensure_known_type(&param.ty, &registry, param.span)?;
                    }
                }
                ClassMember::Property(property) => match property.kind {
                    PropertyKind::Get => {
                        if !property.params.is_empty() {
                            return Err(Diagnostic::new(
                                format!("Property Get '{}' cannot have parameters", property.name),
                                Some(property.span),
                            ));
                        }
                        ensure_known_type(
                            property.return_type.as_ref().expect("get return type"),
                            &registry,
                            property.span,
                        )?;
                    }
                    PropertyKind::Let | PropertyKind::Set => {
                        if property.params.len() != 1 {
                            return Err(Diagnostic::new(
                                format!(
                                    "Property {:?} '{}' must have exactly one parameter",
                                    property.kind, property.name
                                ),
                                Some(property.span),
                            ));
                        }
                        let param = &property.params[0];
                        if param.mode != PassingMode::ByVal {
                            return Err(Diagnostic::new(
                                format!(
                                    "Property {:?} '{}' parameter must be ByVal",
                                    property.kind, property.name
                                ),
                                Some(param.span),
                            ));
                        }
                        ensure_known_type(&param.ty, &registry, param.span)?;
                        if property.kind == PropertyKind::Set
                            && !matches!(&param.ty, TypeName::User(name) if registry.get_class(name).is_some())
                        {
                            return Err(Diagnostic::new(
                                format!(
                                    "Property Set '{}' parameter must be a class type",
                                    property.name
                                ),
                                Some(param.span),
                            ));
                        }
                        if property.kind == PropertyKind::Let
                            && matches!(&param.ty, TypeName::User(name) if registry.get_class(name).is_some())
                        {
                            return Err(Diagnostic::new(
                                format!(
                                    "Property Let '{}' parameter cannot be a class type",
                                    property.name
                                ),
                                Some(param.span),
                            ));
                        }
                    }
                },
            }
        }
    }

    Ok(registry)
}

fn collect_signatures(program: &Program, types: &TypeRegistry) -> Result<Signatures, Diagnostic> {
    let mut subs = HashMap::new();
    let mut functions = HashMap::new();
    let mut names = HashMap::new();

    for type_decl in &program.types {
        names.insert(key(&type_decl.name), "Type");
    }
    for enum_decl in &program.enums {
        if let Some(existing) = names.insert(key(&enum_decl.name), "Enum") {
            return Err(Diagnostic::new(
                format!(
                    "Name '{}' conflicts with existing {}",
                    enum_decl.name, existing
                ),
                Some(enum_decl.span),
            ));
        }
    }
    for class_decl in &program.classes {
        names.insert(key(&class_decl.name), "Class");
    }

    for procedure in &program.procedures {
        for param in &procedure.params {
            ensure_known_type(&param.ty, types, param.span)?;
        }

        let name_key = key(&procedure.name);
        if let Some(existing) = names.insert(name_key.clone(), "Sub") {
            return Err(Diagnostic::new(
                format!(
                    "Name '{}' conflicts with existing {}",
                    procedure.name, existing
                ),
                Some(procedure.span),
            ));
        }

        subs.insert(
            name_key,
            CallableSig {
                visibility: Visibility::Public,
                name: procedure.name.clone(),
                params: params_to_sigs(&procedure.params),
                return_type: None,
            },
        );
    }

    for function in &program.functions {
        for param in &function.params {
            ensure_known_type(&param.ty, types, param.span)?;
        }
        ensure_known_type(&function.return_type, types, function.span)?;

        let name_key = key(&function.name);
        if let Some(existing) = names.insert(name_key.clone(), "Function") {
            return Err(Diagnostic::new(
                format!(
                    "Name '{}' conflicts with existing {}",
                    function.name, existing
                ),
                Some(function.span),
            ));
        }

        functions.insert(
            name_key,
            CallableSig {
                visibility: Visibility::Public,
                name: function.name.clone(),
                params: params_to_sigs(&function.params),
                return_type: Some(function.return_type.clone()),
            },
        );
    }

    Ok(Signatures { subs, functions })
}

fn eval_enum_const_expr(expr: &Expr, members: &HashMap<String, i64>) -> Result<i64, Diagnostic> {
    match &expr.kind {
        ExprKind::Integer(value) => Ok(*value),
        ExprKind::Variable(name) => members.get(&key(name)).copied().ok_or_else(|| {
            Diagnostic::new(
                format!("Enum member '{}' is not defined", name),
                Some(expr.span),
            )
        }),
        ExprKind::Unary {
            op: UnaryOp::Negate,
            expr,
        } => Ok(-eval_enum_const_expr(expr, members)?),
        ExprKind::Binary { left, op, right } => {
            let left = eval_enum_const_expr(left, members)?;
            let right = eval_enum_const_expr(right, members)?;
            match op {
                BinaryOp::Add => Ok(left + right),
                BinaryOp::Subtract => Ok(left - right),
                BinaryOp::Multiply => Ok(left * right),
                BinaryOp::Divide => {
                    if right == 0 {
                        Err(Diagnostic::new("Division by zero", Some(expr.span)))
                    } else {
                        Ok(left / right)
                    }
                }
                _ => Err(Diagnostic::new(
                    "Enum value expression must be numeric",
                    Some(expr.span),
                )),
            }
        }
        _ => Err(Diagnostic::new(
            "Enum value expression must be numeric",
            Some(expr.span),
        )),
    }
}

fn params_to_sigs(params: &[Parameter]) -> Vec<ParamSig> {
    params
        .iter()
        .map(|param| ParamSig {
            mode: param.mode,
            ty: param.ty.clone(),
        })
        .collect()
}

fn validate_procedure(
    procedure: &Procedure,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<(), Diagnostic> {
    let mut symbols = HashMap::new();
    add_parameters(&procedure.params, &mut symbols)?;
    validate_statements(
        &procedure.body,
        &mut symbols,
        types,
        signatures,
        Context::Sub,
        LoopContext::default(),
    )
}

fn validate_function(
    function: &Function,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<(), Diagnostic> {
    let mut symbols = HashMap::new();
    add_parameters(&function.params, &mut symbols)?;

    let mut saw_return = false;
    validate_statements(
        &function.body,
        &mut symbols,
        types,
        signatures,
        Context::Function {
            return_type: function.return_type.clone(),
            saw_return: &mut saw_return,
        },
        LoopContext::default(),
    )?;

    if !saw_return {
        return Err(Diagnostic::new(
            format!("Function '{}' must return a value", function.name),
            Some(function.span),
        ));
    }

    Ok(())
}

fn add_parameters(
    params: &[Parameter],
    symbols: &mut HashMap<String, VarType>,
) -> Result<(), Diagnostic> {
    for param in params {
        let param_key = key(&param.name);
        if symbols.contains_key(&param_key) {
            return Err(Diagnostic::new(
                format!("Parameter '{}' is already declared", param.name),
                Some(param.span),
            ));
        }
        symbols.insert(param_key, VarType::Scalar(param.ty.clone()));
    }
    Ok(())
}

fn validate_statements(
    statements: &[Stmt],
    symbols: &mut HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    mut context: Context<'_>,
    loop_context: LoopContext,
) -> Result<(), Diagnostic> {
    for stmt in statements {
        match stmt {
            Stmt::Dim {
                name,
                ty,
                array,
                span,
            } => {
                ensure_known_type(ty, types, *span)?;
                let key = key(name);
                if symbols.contains_key(&key) {
                    return Err(Diagnostic::new(
                        format!("Variable '{}' is already declared", name),
                        Some(*span),
                    ));
                }
                let var_type = if array.is_some() {
                    VarType::Array(ty.clone())
                } else {
                    VarType::Scalar(ty.clone())
                };
                symbols.insert(key, var_type);
            }
            Stmt::Assign { target, expr, span } => {
                let expr_type = validate_expr(expr, symbols, types, signatures)?;
                let target_type = validate_assignment_target(
                    target, &expr_type, symbols, types, signatures, &context,
                )?;
                ensure_assignable_expr(&target_type, &expr_type, expr, types, *span)?;
            }
            Stmt::SetAssign { target, expr, span } => {
                let expr_type = validate_expr(expr, symbols, types, signatures)?;
                let target_type = validate_assignment_target(
                    target, &expr_type, symbols, types, signatures, &context,
                )?;
                ensure_class_type(
                    &target_type,
                    types,
                    *span,
                    "Set target must be a class type",
                )?;
                ensure_assignable_expr(&target_type, &expr_type, expr, types, *span)?;
            }
            Stmt::ConsoleWriteLine { args, .. } => {
                for arg in args {
                    validate_expr(arg, symbols, types, signatures)?;
                }
            }
            Stmt::SubCall { name, args, span } => {
                validate_sub_call(name, args, *span, symbols, types, signatures)?;
            }
            Stmt::MemberSubCall {
                object,
                method,
                args,
                span,
            } => {
                let object_type = validate_expr(object, symbols, types, signatures)?;
                validate_method_call(
                    &object_type,
                    method,
                    args,
                    false,
                    *span,
                    symbols,
                    types,
                    signatures,
                    context.current_class(),
                )?;
            }
            Stmt::Return { expr, span } => match &mut context {
                Context::Sub | Context::MethodSub { .. } | Context::PropertyLetSet { .. } => {
                    return Err(Diagnostic::new(
                        "Return is only allowed inside Function or Property Get",
                        Some(*span),
                    ));
                }
                Context::Function {
                    return_type,
                    saw_return,
                }
                | Context::MethodFunction {
                    return_type,
                    saw_return,
                    ..
                }
                | Context::PropertyGet {
                    return_type,
                    saw_return,
                    ..
                } => {
                    let expr_type = validate_expr(expr, symbols, types, signatures)?;
                    ensure_assignable_expr(return_type, &expr_type, expr, types, *span)?;
                    **saw_return = true;
                }
            },
            Stmt::If {
                condition,
                then_body,
                elseif_branches,
                else_body,
                ..
            } => {
                ensure_assignable(
                    &TypeName::Boolean,
                    &validate_expr(condition, symbols, types, signatures)?,
                    condition.span,
                )?;
                validate_statements(
                    then_body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context,
                )?;
                for branch in elseif_branches {
                    ensure_assignable(
                        &TypeName::Boolean,
                        &validate_expr(&branch.condition, symbols, types, signatures)?,
                        branch.condition.span,
                    )?;
                    validate_statements(
                        &branch.body,
                        symbols,
                        types,
                        signatures,
                        context.reborrow(),
                        loop_context,
                    )?;
                }
                validate_statements(
                    else_body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context,
                )?;
            }
            Stmt::SelectCase {
                subject,
                branches,
                else_body,
                ..
            } => {
                let subject_type = validate_expr(subject, symbols, types, signatures)?;
                for branch in branches {
                    for item in &branch.items {
                        validate_case_item(item, &subject_type, symbols, types, signatures)?;
                    }
                    validate_statements(
                        &branch.body,
                        symbols,
                        types,
                        signatures,
                        context.reborrow(),
                        loop_context,
                    )?;
                }
                validate_statements(
                    else_body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context,
                )?;
            }
            Stmt::While {
                condition, body, ..
            } => {
                validate_expr(condition, symbols, types, signatures)?;
                validate_statements(
                    body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context.in_while(),
                )?;
            }
            Stmt::DoLoop {
                condition, body, ..
            } => {
                match condition {
                    DoLoopCondition::Infinite => {}
                    DoLoopCondition::PreWhile(condition)
                    | DoLoopCondition::PreUntil(condition)
                    | DoLoopCondition::PostWhile(condition)
                    | DoLoopCondition::PostUntil(condition) => {
                        ensure_assignable(
                            &TypeName::Boolean,
                            &validate_expr(condition, symbols, types, signatures)?,
                            condition.span,
                        )?;
                    }
                }
                validate_statements(
                    body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context.in_do(),
                )?;
            }
            Stmt::For {
                variable,
                start,
                end,
                step,
                next_variable,
                body,
                span,
            } => {
                let Some(ty) = symbols.get(&key(variable)) else {
                    return Err(Diagnostic::new(
                        format!("Variable '{}' is not declared", variable),
                        Some(*span),
                    ));
                };

                if !matches!(ty, VarType::Scalar(scalar) if scalar.same_type(&TypeName::Integer)) {
                    return Err(Diagnostic::new(
                        format!("For loop variable '{}' must be Integer", variable),
                        Some(*span),
                    ));
                }

                ensure_assignable(
                    &TypeName::Integer,
                    &validate_expr(start, symbols, types, signatures)?,
                    start.span,
                )?;
                ensure_assignable(
                    &TypeName::Integer,
                    &validate_expr(end, symbols, types, signatures)?,
                    end.span,
                )?;
                if let Some(step) = step {
                    ensure_assignable(
                        &TypeName::Integer,
                        &validate_expr(step, symbols, types, signatures)?,
                        step.span,
                    )?;
                }
                if let Some((next_variable, next_span)) = next_variable
                    && !next_variable.eq_ignore_ascii_case(variable)
                {
                    return Err(Diagnostic::new(
                        format!(
                            "Next variable '{}' does not match For variable '{}'",
                            next_variable, variable
                        ),
                        Some(*next_span),
                    ));
                }
                validate_statements(
                    body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context.in_for(),
                )?;
            }
            Stmt::ForEach {
                variable,
                iterable,
                next_variable,
                body,
                span,
            } => {
                let Some(loop_type) = symbols.get(&key(variable)).cloned() else {
                    return Err(Diagnostic::new(
                        format!("Variable '{}' is not declared", variable),
                        Some(*span),
                    ));
                };
                let VarType::Scalar(loop_type) = loop_type else {
                    return Err(Diagnostic::new(
                        format!("Array variable '{}' cannot be used as a scalar", variable),
                        Some(*span),
                    ));
                };
                let array_type = validate_array_expr(iterable, symbols, types, signatures)?;
                ensure_assignable(&loop_type, &array_type, *span)?;
                if let Some((next_variable, next_span)) = next_variable
                    && !next_variable.eq_ignore_ascii_case(variable)
                {
                    return Err(Diagnostic::new(
                        format!(
                            "Next variable '{}' does not match For Each variable '{}'",
                            next_variable, variable
                        ),
                        Some(*next_span),
                    ));
                }
                validate_statements(
                    body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context.in_for(),
                )?;
            }
            Stmt::ReDim {
                name,
                upper_bound,
                span,
                ..
            } => {
                let Some(var_type) = symbols.get(&key(name)).cloned() else {
                    return Err(Diagnostic::new(
                        format!("Variable '{}' is not declared", name),
                        Some(*span),
                    ));
                };
                if !matches!(var_type, VarType::Array(_)) {
                    return Err(Diagnostic::new(
                        "ReDim target must be a dynamic array",
                        Some(*span),
                    ));
                }
                ensure_assignable(
                    &TypeName::Integer,
                    &validate_expr(upper_bound, symbols, types, signatures)?,
                    upper_bound.span,
                )?;
            }
            Stmt::Exit { target, span } => {
                validate_exit(*target, *span, &context, loop_context)?;
            }
        }
    }

    Ok(())
}

fn validate_sub_call(
    name: &str,
    args: &[Expr],
    span: crate::runtime::Span,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<(), Diagnostic> {
    let Some(sub) = signatures.subs.get(&key(name)) else {
        if signatures.functions.contains_key(&key(name)) {
            return Err(Diagnostic::new(
                format!("Function '{}' cannot be called as a statement", name),
                Some(span),
            ));
        }
        return Err(Diagnostic::new(
            format!("Sub '{}' is not defined", name),
            Some(span),
        ));
    };

    validate_arguments("Sub", sub, args, symbols, types, signatures, span)
}

fn validate_assignment_target(
    target: &AssignTarget,
    value_type: &TypeName,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
) -> Result<TypeName, Diagnostic> {
    match target {
        AssignTarget::Variable { name, span } => {
            let Some(target_type) = symbols.get(&key(name)).cloned() else {
                return Err(Diagnostic::new(
                    format!("Variable '{}' is not declared", name),
                    Some(*span),
                ));
            };
            let VarType::Scalar(target_type) = target_type else {
                return Err(Diagnostic::new(
                    format!("Array variable '{}' cannot be used as a scalar", name),
                    Some(*span),
                ));
            };
            Ok(target_type)
        }
        AssignTarget::ArrayElement { name, index, span } => {
            let Some(var_type) = symbols.get(&key(name)).cloned() else {
                return Err(Diagnostic::new(
                    format!("Variable '{}' is not declared", name),
                    Some(*span),
                ));
            };
            let VarType::Array(element_type) = var_type else {
                return Err(Diagnostic::new(
                    format!("Variable '{}' is not an array", name),
                    Some(*span),
                ));
            };
            ensure_assignable(
                &TypeName::Integer,
                &validate_expr(index, symbols, types, signatures)?,
                index.span,
            )?;
            Ok(element_type)
        }
        AssignTarget::Member {
            object,
            field,
            span,
        } => {
            let object_type = validate_expr(object, symbols, types, signatures)?;
            let current_class = member_access_class(object, &object_type)
                .or_else(|| context.current_class().map(str::to_string));
            member_assignment_type(
                &object_type,
                field,
                value_type,
                types,
                *span,
                current_class.as_deref(),
            )
        }
    }
}

fn validate_expr(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<TypeName, Diagnostic> {
    match &expr.kind {
        ExprKind::String(_) => Ok(TypeName::String),
        ExprKind::Integer(_) => Ok(TypeName::Integer),
        ExprKind::Boolean(_) => Ok(TypeName::Boolean),
        ExprKind::Nothing => Ok(TypeName::Variant),
        ExprKind::Me => match symbols.get("me").cloned() {
            Some(VarType::Scalar(ty)) => Ok(ty),
            _ => Err(Diagnostic::new(
                "Me is only valid inside class methods",
                Some(expr.span),
            )),
        },
        ExprKind::New { class_name, args } => {
            let class_sig = types.get_class(class_name).ok_or_else(|| {
                Diagnostic::new(
                    format!("Class '{}' is not defined", class_name),
                    Some(expr.span),
                )
            })?;
            if let Some(init) = class_sig.subs.get("initialize") {
                validate_arguments("Sub", init, args, symbols, types, signatures, expr.span)?;
            } else if !args.is_empty() {
                return Err(Diagnostic::new(
                    format!("Class '{}' has no Initialize constructor", class_sig.name),
                    Some(expr.span),
                ));
            }
            Ok(TypeName::User(class_sig.name.clone()))
        }
        ExprKind::Variable(name) => match symbols.get(&key(name)).cloned() {
            Some(VarType::Scalar(ty)) => Ok(ty),
            Some(VarType::Array(_)) => Err(Diagnostic::new(
                format!("Array variable '{}' cannot be used as a scalar", name),
                Some(expr.span),
            )),
            None => {
                if enum_member_value_type(name, types).is_some() {
                    Ok(TypeName::Integer)
                } else {
                    Err(Diagnostic::new(
                        format!("Variable '{}' is not declared", name),
                        Some(expr.span),
                    ))
                }
            }
        },
        ExprKind::MemberAccess { object, field } => {
            if let ExprKind::Variable(enum_name) = &object.kind
                && let Some(enum_sig) = types.get_enum(enum_name)
            {
                if enum_sig.members.contains_key(&key(field)) {
                    return Ok(TypeName::Integer);
                }
                return Err(Diagnostic::new(
                    format!("Enum '{}' has no member '{}'", enum_sig.name, field),
                    Some(expr.span),
                ));
            }
            let object_type = validate_expr(object, symbols, types, signatures)?;
            let current_class = member_access_class(object, &object_type);
            member_read_type(
                &object_type,
                field,
                types,
                expr.span,
                current_class.as_deref(),
            )
        }
        ExprKind::MemberCall {
            object,
            method,
            args,
        } => {
            let object_type = validate_expr(object, symbols, types, signatures)?;
            validate_method_call(
                &object_type,
                method,
                args,
                true,
                expr.span,
                symbols,
                types,
                signatures,
                member_access_class(object, &object_type).as_deref(),
            )
        }
        ExprKind::Call { name, args } => {
            if name.eq_ignore_ascii_case("LBound") || name.eq_ignore_ascii_case("UBound") {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        format!("{} expects exactly one argument", name),
                        Some(expr.span),
                    ));
                }
                validate_array_expr(&args[0], symbols, types, signatures)?;
                return Ok(TypeName::Integer);
            }
            if let Some(var_type) = symbols.get(&key(name)).cloned() {
                let VarType::Array(element_type) = var_type else {
                    return Err(Diagnostic::new(
                        format!("Variable '{}' is not an array", name),
                        Some(expr.span),
                    ));
                };
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "Array access requires exactly one index",
                        Some(expr.span),
                    ));
                }
                ensure_assignable(
                    &TypeName::Integer,
                    &validate_expr(&args[0], symbols, types, signatures)?,
                    args[0].span,
                )?;
                return Ok(element_type);
            }

            let Some(function) = signatures.functions.get(&key(name)) else {
                if signatures.subs.contains_key(&key(name)) {
                    return Err(Diagnostic::new(
                        format!("Sub '{}' cannot be used as an expression", name),
                        Some(expr.span),
                    ));
                }
                return Err(Diagnostic::new(
                    format!("Function '{}' is not defined", name),
                    Some(expr.span),
                ));
            };

            validate_arguments(
                "Function", function, args, symbols, types, signatures, expr.span,
            )?;
            Ok(function.return_type.clone().expect("function return type"))
        }
        ExprKind::Binary { left, op, right } => {
            let left_type = validate_expr(left, symbols, types, signatures)?;
            let right_type = validate_expr(right, symbols, types, signatures)?;
            match op {
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Modulo => {
                    ensure_assignable(&TypeName::Integer, &left_type, left.span)?;
                    ensure_assignable(&TypeName::Integer, &right_type, right.span)?;
                    Ok(TypeName::Integer)
                }
                BinaryOp::Concat => Ok(TypeName::String),
                BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                    if left_type.same_type(&TypeName::Boolean)
                        && right_type.same_type(&TypeName::Boolean)
                    {
                        Ok(TypeName::Boolean)
                    } else if left_type.same_type(&TypeName::Integer)
                        && right_type.same_type(&TypeName::Integer)
                        || (is_enum_type(&left_type, types)
                            && right_type.same_type(&TypeName::Integer))
                        || (left_type.same_type(&TypeName::Integer)
                            && is_enum_type(&right_type, types))
                        || (is_enum_type(&left_type, types) && is_enum_type(&right_type, types))
                    {
                        Ok(TypeName::Integer)
                    } else {
                        Err(Diagnostic::new(
                            "Logical operators require Boolean or Integer operands",
                            Some(expr.span),
                        ))
                    }
                }
                BinaryOp::Equal | BinaryOp::NotEqual => Ok(TypeName::Boolean),
                BinaryOp::Is => {
                    if is_object_reference_expr(left, &left_type, types)
                        && is_object_reference_expr(right, &right_type, types)
                    {
                        Ok(TypeName::Boolean)
                    } else {
                        Err(Diagnostic::new(
                            "'Is' requires class object operands or Nothing",
                            Some(expr.span),
                        ))
                    }
                }
                BinaryOp::Less
                | BinaryOp::Greater
                | BinaryOp::LessEqual
                | BinaryOp::GreaterEqual => {
                    if left_type.same_type(&right_type)
                        && (left_type.same_type(&TypeName::Integer)
                            || left_type.same_type(&TypeName::String))
                    {
                        Ok(TypeName::Boolean)
                    } else {
                        Err(Diagnostic::new(
                            "Comparison requires matching Integer or String operands",
                            Some(expr.span),
                        ))
                    }
                }
            }
        }
        ExprKind::Unary { op, expr: inner } => match op {
            UnaryOp::Negate => {
                ensure_assignable(
                    &TypeName::Integer,
                    &validate_expr(inner, symbols, types, signatures)?,
                    inner.span,
                )?;
                Ok(TypeName::Integer)
            }
            UnaryOp::LogicalNot => {
                ensure_assignable(
                    &TypeName::Boolean,
                    &validate_expr(inner, symbols, types, signatures)?,
                    inner.span,
                )?;
                Ok(TypeName::Boolean)
            }
        },
    }
}

fn validate_array_expr(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    _types: &TypeRegistry,
    _signatures: &Signatures,
) -> Result<TypeName, Diagnostic> {
    match &expr.kind {
        ExprKind::Variable(name) => match symbols.get(&key(name)).cloned() {
            Some(VarType::Array(element_type)) => Ok(element_type),
            Some(VarType::Scalar(_)) => Err(Diagnostic::new(
                format!("Variable '{}' is not an array", name),
                Some(expr.span),
            )),
            None => Err(Diagnostic::new(
                format!("Variable '{}' is not declared", name),
                Some(expr.span),
            )),
        },
        _ => Err(Diagnostic::new("Expected array variable", Some(expr.span))),
    }
}

fn enum_member_value_type(name: &str, types: &TypeRegistry) -> Option<TypeName> {
    for enum_sig in types.enums.values() {
        if enum_sig.members.contains_key(&key(name)) {
            return Some(TypeName::Integer);
        }
    }
    None
}

fn validate_arguments(
    kind: &str,
    callable: &CallableSig,
    args: &[Expr],
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if args.len() != callable.params.len() {
        return Err(Diagnostic::new(
            format!(
                "{} '{}' expects {} argument(s), got {}",
                kind,
                callable.name,
                callable.params.len(),
                args.len()
            ),
            Some(span),
        ));
    }

    for (arg, param) in args.iter().zip(&callable.params) {
        match param.mode {
            PassingMode::ByVal => {
                let arg_type = validate_expr(arg, symbols, types, signatures)?;
                ensure_assignable_expr(&param.ty, &arg_type, arg, types, arg.span)?;
            }
            PassingMode::ByRef => {
                let ExprKind::Variable(name) = &arg.kind else {
                    return Err(Diagnostic::new(
                        "ByRef argument must be a variable",
                        Some(arg.span),
                    ));
                };
                let Some(arg_type) = symbols.get(&key(name)).cloned() else {
                    return Err(Diagnostic::new(
                        format!("Variable '{}' is not declared", name),
                        Some(arg.span),
                    ));
                };
                let expected = VarType::Scalar(param.ty.clone());
                if !arg_type.same_var_type(&expected) {
                    return Err(Diagnostic::new(
                        format!(
                            "ByRef argument type {} must match parameter type {}",
                            arg_type.display_name(),
                            expected.display_name()
                        ),
                        Some(arg.span),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn validate_method_call(
    object_type: &TypeName,
    method: &str,
    args: &[Expr],
    as_expression: bool,
    span: crate::runtime::Span,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    current_class: Option<&str>,
) -> Result<TypeName, Diagnostic> {
    let TypeName::User(class_name) = object_type else {
        return Err(Diagnostic::new(
            "Method call requires a class instance",
            Some(span),
        ));
    };
    let class_sig = types.get_class(class_name).ok_or_else(|| {
        Diagnostic::new(format!("Class '{}' is not defined", class_name), Some(span))
    })?;

    if as_expression {
        let Some(method_sig) = class_sig.functions.get(&key(method)) else {
            if class_sig.subs.contains_key(&key(method)) {
                return Err(Diagnostic::new(
                    format!("Sub method '{}' cannot be used as an expression", method),
                    Some(span),
                ));
            }
            return Err(Diagnostic::new(
                format!("Class '{}' has no method '{}'", class_sig.name, method),
                Some(span),
            ));
        };
        ensure_visible(
            method_sig.visibility,
            &class_sig.name,
            method,
            current_class,
            span,
        )?;
        validate_arguments(
            "Function", method_sig, args, symbols, types, signatures, span,
        )?;
        Ok(method_sig.return_type.clone().expect("function return"))
    } else {
        let Some(method_sig) = class_sig.subs.get(&key(method)) else {
            if class_sig.functions.contains_key(&key(method)) {
                return Err(Diagnostic::new(
                    format!(
                        "Function method '{}' cannot be called as a statement",
                        method
                    ),
                    Some(span),
                ));
            }
            return Err(Diagnostic::new(
                format!("Class '{}' has no method '{}'", class_sig.name, method),
                Some(span),
            ));
        };
        ensure_visible(
            method_sig.visibility,
            &class_sig.name,
            method,
            current_class,
            span,
        )?;
        validate_arguments("Sub", method_sig, args, symbols, types, signatures, span)?;
        Ok(TypeName::Variant)
    }
}

fn member_access_class(object: &Expr, object_type: &TypeName) -> Option<String> {
    if matches!(object.kind, ExprKind::Me) {
        if let TypeName::User(name) = object_type {
            return Some(name.clone());
        }
    }
    None
}

fn member_read_type(
    object_type: &TypeName,
    member: &str,
    types: &TypeRegistry,
    span: crate::runtime::Span,
    current_class: Option<&str>,
) -> Result<TypeName, Diagnostic> {
    let TypeName::User(type_name) = object_type else {
        return Err(Diagnostic::new(
            "Member access requires a user-defined Type value",
            Some(span),
        ));
    };

    if let Some(type_sig) = types.get(type_name) {
        return type_sig
            .fields
            .get(&key(member))
            .map(|field| field.ty.clone())
            .ok_or_else(|| {
                Diagnostic::new(
                    format!("Type '{}' has no field '{}'", type_sig.name, member),
                    Some(span),
                )
            });
    }

    let class_sig = types.get_class(type_name).ok_or_else(|| {
        Diagnostic::new(format!("Type '{}' is not defined", type_name), Some(span))
    })?;
    if let Some(field_sig) = class_sig.fields.get(&key(member)) {
        ensure_visible(
            field_sig.visibility,
            &class_sig.name,
            member,
            current_class,
            span,
        )?;
        return Ok(field_sig.ty.clone());
    }

    let property_sig = class_sig.properties.get(&key(member)).ok_or_else(|| {
        Diagnostic::new(
            format!(
                "Class '{}' has no field or property '{}'",
                class_sig.name, member
            ),
            Some(span),
        )
    })?;
    let get = property_sig.get.as_ref().ok_or_else(|| {
        Diagnostic::new(
            format!("Property '{}' has no Get accessor", property_sig.name),
            Some(span),
        )
    })?;
    ensure_visible(get.visibility, &class_sig.name, member, current_class, span)?;
    Ok(get.return_type.clone().expect("get return type"))
}

fn member_assignment_type(
    object_type: &TypeName,
    member: &str,
    value_type: &TypeName,
    types: &TypeRegistry,
    span: crate::runtime::Span,
    current_class: Option<&str>,
) -> Result<TypeName, Diagnostic> {
    let TypeName::User(type_name) = object_type else {
        return Err(Diagnostic::new(
            "Member assignment requires a user-defined Type value",
            Some(span),
        ));
    };

    if let Some(type_sig) = types.get(type_name) {
        return type_sig
            .fields
            .get(&key(member))
            .map(|field| field.ty.clone())
            .ok_or_else(|| {
                Diagnostic::new(
                    format!("Type '{}' has no field '{}'", type_sig.name, member),
                    Some(span),
                )
            });
    }

    let class_sig = types.get_class(type_name).ok_or_else(|| {
        Diagnostic::new(format!("Type '{}' is not defined", type_name), Some(span))
    })?;
    if let Some(field_sig) = class_sig.fields.get(&key(member)) {
        ensure_visible(
            field_sig.visibility,
            &class_sig.name,
            member,
            current_class,
            span,
        )?;
        return Ok(field_sig.ty.clone());
    }

    let property_sig = class_sig.properties.get(&key(member)).ok_or_else(|| {
        Diagnostic::new(
            format!(
                "Class '{}' has no field or property '{}'",
                class_sig.name, member
            ),
            Some(span),
        )
    })?;
    let accessor =
        if is_class_type(value_type, types) || value_type.same_type(&TypeName::Variant) {
            property_sig.set.as_ref().or(property_sig.let_.as_ref())
        } else {
            property_sig.let_.as_ref()
        }
        .ok_or_else(|| {
            Diagnostic::new(
                format!(
                    "Property '{}' has no Let or Set accessor",
                    property_sig.name
                ),
                Some(span),
            )
        })?;
    ensure_visible(
        accessor.visibility,
        &class_sig.name,
        member,
        current_class,
        span,
    )?;
    Ok(accessor.params[0].ty.clone())
}

fn ensure_visible(
    visibility: Visibility,
    owner_class: &str,
    member: &str,
    current_class: Option<&str>,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if visibility == Visibility::Public
        || current_class.is_some_and(|class_name| class_name.eq_ignore_ascii_case(owner_class))
    {
        Ok(())
    } else {
        Err(Diagnostic::new(
            format!("Member '{}' is Private in Class '{}'", member, owner_class),
            Some(span),
        ))
    }
}

fn ensure_known_type(
    ty: &TypeName,
    types: &TypeRegistry,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    match ty {
        TypeName::String | TypeName::Integer | TypeName::Boolean | TypeName::Variant => Ok(()),
        TypeName::User(name) => {
            if types.contains(name) {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    format!("Type '{}' is not defined", name),
                    Some(span),
                ))
            }
        }
    }
}

fn ensure_assignable_expr(
    target: &TypeName,
    source: &TypeName,
    source_expr: &Expr,
    types: &TypeRegistry,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if matches!(source_expr.kind, ExprKind::Nothing) {
        return ensure_class_type(target, types, span, "Nothing requires a class object type");
    }
    if is_enum_type(target, types) && source.same_type(&TypeName::Integer) {
        return Ok(());
    }
    ensure_assignable(target, source, span)
}

fn ensure_class_type(
    ty: &TypeName,
    types: &TypeRegistry,
    span: crate::runtime::Span,
    message: &str,
) -> Result<(), Diagnostic> {
    if is_class_type(ty, types) {
        Ok(())
    } else {
        Err(Diagnostic::new(message, Some(span)))
    }
}

fn is_object_reference_expr(expr: &Expr, ty: &TypeName, types: &TypeRegistry) -> bool {
    matches!(expr.kind, ExprKind::Nothing) || is_class_type(ty, types)
}

fn is_class_type(ty: &TypeName, types: &TypeRegistry) -> bool {
    matches!(ty, TypeName::User(name) if types.get_class(name).is_some())
}

fn is_enum_type(ty: &TypeName, types: &TypeRegistry) -> bool {
    matches!(ty, TypeName::User(name) if types.get_enum(name).is_some())
}

fn ensure_case_comparable(
    subject: &TypeName,
    value: &TypeName,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if subject.same_type(&TypeName::Variant)
        || value.same_type(&TypeName::Variant)
        || subject.same_type(value)
        || (matches!(subject, TypeName::User(_)) && value.same_type(&TypeName::Integer))
        || (subject.same_type(&TypeName::Integer) && matches!(value, TypeName::User(_)))
    {
        Ok(())
    } else {
        Err(Diagnostic::new(
            "Case expression type must match Select Case expression type",
            Some(span),
        ))
    }
}

fn validate_case_item(
    item: &CaseItem,
    subject_type: &TypeName,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<(), Diagnostic> {
    match item {
        CaseItem::Value(value) => {
            let value_type = validate_expr(value, symbols, types, signatures)?;
            ensure_case_comparable(subject_type, &value_type, value.span)
        }
        CaseItem::Range { start, end } => {
            let start_type = validate_expr(start, symbols, types, signatures)?;
            let end_type = validate_expr(end, symbols, types, signatures)?;
            ensure_case_comparable(subject_type, &start_type, start.span)?;
            ensure_case_comparable(subject_type, &end_type, end.span)?;
            ensure_case_orderable(subject_type, start.span)
        }
        CaseItem::Compare { op, expr } => {
            let expr_type = validate_expr(expr, symbols, types, signatures)?;
            ensure_case_comparable(subject_type, &expr_type, expr.span)?;
            if matches!(
                op,
                CaseCompareOp::Less
                    | CaseCompareOp::Greater
                    | CaseCompareOp::LessEqual
                    | CaseCompareOp::GreaterEqual
            ) {
                ensure_case_orderable(subject_type, expr.span)?;
            }
            Ok(())
        }
    }
}

fn ensure_case_orderable(ty: &TypeName, span: crate::runtime::Span) -> Result<(), Diagnostic> {
    if ty.same_type(&TypeName::Integer)
        || ty.same_type(&TypeName::String)
        || ty.same_type(&TypeName::Variant)
    {
        Ok(())
    } else {
        Err(Diagnostic::new(
            "Case range or comparison requires Integer or String operands",
            Some(span),
        ))
    }
}

fn validate_exit(
    target: ExitTarget,
    span: crate::runtime::Span,
    context: &Context<'_>,
    loop_context: LoopContext,
) -> Result<(), Diagnostic> {
    match target {
        ExitTarget::Sub => match context {
            Context::Sub | Context::MethodSub { .. } => Ok(()),
            _ => Err(Diagnostic::new(
                "Exit Sub is only valid inside Sub",
                Some(span),
            )),
        },
        ExitTarget::Function => match context {
            Context::Function { .. } | Context::MethodFunction { .. } => Ok(()),
            _ => Err(Diagnostic::new(
                "Exit Function is only valid inside Function",
                Some(span),
            )),
        },
        ExitTarget::For => {
            if loop_context.for_depth > 0 {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    "Exit For is only valid inside For",
                    Some(span),
                ))
            }
        }
        ExitTarget::While => {
            if loop_context.while_depth > 0 {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    "Exit While is only valid inside While",
                    Some(span),
                ))
            }
        }
        ExitTarget::Do => {
            if loop_context.do_depth > 0 {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    "Exit Do is only valid inside Do",
                    Some(span),
                ))
            }
        }
    }
}

fn ensure_assignable(
    target: &TypeName,
    source: &TypeName,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if target.same_type(&TypeName::Variant) || target.same_type(source) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            format!(
                "Cannot assign {} value to {} variable",
                source.display_name(),
                target.display_name()
            ),
            Some(span),
        ))
    }
}
