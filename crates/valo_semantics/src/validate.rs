use std::collections::HashMap;

use valo_parser::{
    BinaryOp, ClassMember, Expr, ExprKind, Function, Parameter, PassingMode, Procedure, Program,
    Stmt, UnaryOp, Visibility,
};
use valo_runtime::{Diagnostic, TypeName};

use crate::context::Context;
use crate::symbols::{CallableSig, ParamSig, Signatures, VarType, key};
use crate::types::{ClassFieldSig, ClassMethodSig, ClassSig, FieldSig, TypeRegistry, TypeSig};

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
    class_decl: &valo_parser::ClassDecl,
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
                )?;
                if !saw_return {
                    return Err(Diagnostic::new(
                        format!("Function '{}' must return a value", method.function.name),
                        Some(method.function.span),
                    ));
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

    let mut classes = HashMap::new();
    for class_decl in &program.classes {
        let class_key = key(&class_decl.name);
        if types.contains_key(&class_key) || classes.contains_key(&class_key) {
            return Err(Diagnostic::new(
                format!("Class '{}' is already defined", class_decl.name),
                Some(class_decl.span),
            ));
        }

        let mut fields = HashMap::new();
        let mut subs = HashMap::new();
        let mut functions = HashMap::new();
        for member in &class_decl.members {
            match member {
                ClassMember::Field(field) => {
                    let field_key = key(&field.name);
                    if fields.contains_key(&field_key) {
                        return Err(Diagnostic::new(
                            format!(
                                "Field '{}' is already declared in Class '{}'",
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
                    if subs.contains_key(&method_key) || functions.contains_key(&method_key) {
                        return Err(Diagnostic::new(
                            format!(
                                "Method '{}' is already declared in Class '{}'",
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
                    if subs.contains_key(&method_key) || functions.contains_key(&method_key) {
                        return Err(Diagnostic::new(
                            format!(
                                "Method '{}' is already declared in Class '{}'",
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
            }
        }
        classes.insert(
            class_key,
            ClassSig {
                name: class_decl.name.clone(),
                fields,
                subs,
                functions,
            },
        );
    }

    let registry = TypeRegistry { types, classes };
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
) -> Result<(), Diagnostic> {
    for stmt in statements {
        match stmt {
            Stmt::Dim {
                name,
                ty,
                array_size,
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
                let var_type = if array_size.is_some() {
                    VarType::Array(ty.clone())
                } else {
                    VarType::Scalar(ty.clone())
                };
                symbols.insert(key, var_type);
            }
            Stmt::Assign { name, expr, span } => {
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
                let expr_type = validate_expr(expr, symbols, types, signatures)?;
                ensure_assignable(&target_type, &expr_type, *span)?;
            }
            Stmt::ArrayAssign {
                name,
                index,
                expr,
                span,
            } => {
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
                let expr_type = validate_expr(expr, symbols, types, signatures)?;
                ensure_assignable(&element_type, &expr_type, *span)?;
            }
            Stmt::MemberAssign {
                target,
                field,
                expr,
                span,
            } => {
                let target_type = validate_expr(target, symbols, types, signatures)?;
                let current_class = member_access_class(target, &target_type);
                let field_type =
                    field_type(&target_type, field, types, *span, current_class.as_deref())?;
                let expr_type = validate_expr(expr, symbols, types, signatures)?;
                ensure_assignable(&field_type, &expr_type, *span)?;
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
                Context::Sub | Context::MethodSub { .. } => {
                    return Err(Diagnostic::new(
                        "Return is only allowed inside Function",
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
                } => {
                    let expr_type = validate_expr(expr, symbols, types, signatures)?;
                    ensure_assignable(return_type, &expr_type, *span)?;
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
                validate_statements(then_body, symbols, types, signatures, context.reborrow())?;
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
                    )?;
                }
                validate_statements(else_body, symbols, types, signatures, context.reborrow())?;
            }
            Stmt::While {
                condition, body, ..
            } => {
                validate_expr(condition, symbols, types, signatures)?;
                validate_statements(body, symbols, types, signatures, context.reborrow())?;
            }
            Stmt::For {
                variable,
                start,
                end,
                step,
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
                validate_statements(body, symbols, types, signatures, context.reborrow())?;
            }
        }
    }

    Ok(())
}

fn validate_sub_call(
    name: &str,
    args: &[Expr],
    span: valo_runtime::Span,
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
            None => Err(Diagnostic::new(
                format!("Variable '{}' is not declared", name),
                Some(expr.span),
            )),
        },
        ExprKind::MemberAccess { object, field } => {
            let object_type = validate_expr(object, symbols, types, signatures)?;
            let current_class = member_access_class(object, &object_type);
            field_type(
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
                    ensure_assignable(&TypeName::Boolean, &left_type, left.span)?;
                    ensure_assignable(&TypeName::Boolean, &right_type, right.span)?;
                    Ok(TypeName::Boolean)
                }
                BinaryOp::Equal | BinaryOp::NotEqual => Ok(TypeName::Boolean),
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

fn validate_arguments(
    kind: &str,
    callable: &CallableSig,
    args: &[Expr],
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    span: valo_runtime::Span,
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
                ensure_assignable(&param.ty, &arg_type, arg.span)?;
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
    span: valo_runtime::Span,
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

fn field_type(
    object_type: &TypeName,
    field: &str,
    types: &TypeRegistry,
    span: valo_runtime::Span,
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
            .get(&key(field))
            .map(|field| field.ty.clone())
            .ok_or_else(|| {
                Diagnostic::new(
                    format!("Type '{}' has no field '{}'", type_sig.name, field),
                    Some(span),
                )
            });
    }

    let class_sig = types.get_class(type_name).ok_or_else(|| {
        Diagnostic::new(format!("Type '{}' is not defined", type_name), Some(span))
    })?;
    let field_sig = class_sig.fields.get(&key(field)).ok_or_else(|| {
        Diagnostic::new(
            format!("Class '{}' has no field '{}'", class_sig.name, field),
            Some(span),
        )
    })?;
    ensure_visible(
        field_sig.visibility,
        &class_sig.name,
        field,
        current_class,
        span,
    )?;
    Ok(field_sig.ty.clone())
}

fn ensure_visible(
    visibility: Visibility,
    owner_class: &str,
    member: &str,
    current_class: Option<&str>,
    span: valo_runtime::Span,
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
    span: valo_runtime::Span,
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

fn ensure_assignable(
    target: &TypeName,
    source: &TypeName,
    span: valo_runtime::Span,
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
