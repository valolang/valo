use super::*;
use crate::ContinueTarget;
use crate::runtime::Span;
use crate::runtime::builtins::{self, strip_vba_namespace};
use crate::runtime::overloads;
use crate::runtime::well_known;

#[derive(Clone, Copy)]
pub(super) struct ExprValidation<'a, 'ctx> {
    pub(super) symbols: &'a HashMap<String, VarType>,
    pub(super) types: &'a TypeRegistry,
    pub(super) signatures: &'a Signatures,
    pub(super) context: &'a Context<'ctx>,
    pub(super) options: Options,
}

impl<'a, 'ctx> ExprValidation<'a, 'ctx> {
    pub(super) fn new(
        symbols: &'a HashMap<String, VarType>,
        types: &'a TypeRegistry,
        signatures: &'a Signatures,
        context: &'a Context<'ctx>,
        options: Options,
    ) -> Self {
        Self {
            symbols,
            types,
            signatures,
            context,
            options,
        }
    }
}

pub(super) fn validate_assignment_target(
    target: &AssignTarget,
    value_type: &TypeName,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
    options: Options,
) -> Result<TypeName, Diagnostic> {
    match target {
        AssignTarget::Variable { name, span } => {
            let target_type = if let Some(target_type) = symbols.get(&key(name)).cloned() {
                target_type
            } else if let Some(owner_name) = context.current_class() {
                if let Some(class_sig) = types.get_class(owner_name)
                    && let Some(field_sig) = class_sig.fields.get(&key(name))
                {
                    if field_sig.is_shared || symbols.contains_key(well_known::SELF_KEY) {
                        VarType::Scalar(Visibility::Public, field_sig.ty.clone())
                    } else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Instance field '{}' cannot be accessed from a Shared method",
                                name
                            ),
                            Some(*span),
                        ));
                    }
                } else if let Some(property_type) = types
                    .get_class(owner_name)
                    .and_then(|class_sig| class_sig.properties.get(&key(name)))
                    .and_then(bare_property_type)
                {
                    // A class property is also reachable by its bare name from
                    // inside the class, the same way a field is.
                    VarType::Scalar(Visibility::Public, property_type)
                } else if let Some(type_sig) = types.get(owner_name)
                    && let Some(field_sig) = type_sig.fields.get(&key(name))
                {
                    VarType::Scalar(Visibility::Public, field_sig.ty.clone())
                } else {
                    return Err(unknown_variable(name, *span, symbols));
                }
            } else {
                if !options.explicit {
                    return Ok(TypeName::Variant);
                }
                return Err(unknown_variable(name, *span, symbols));
            };
            if target_type.is_const() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::INVALID_ASSIGNMENT,
                    format!("Constant '{}' cannot be assigned", name),
                    Some(*span),
                ));
            }
            let Some(target_type) = target_type.scalar_type() else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    format!("Array variable '{}' cannot be used as a scalar", name),
                    Some(*span),
                ));
            };
            Ok(target_type)
        }
        AssignTarget::ArrayElement {
            name,
            indices,
            span,
        } => {
            let var_type = if let Some(var_type) = symbols.get(&key(name)).cloned() {
                var_type
            } else if let Some(owner_name) = context.current_class() {
                if let Some(class_sig) = types.get_class(owner_name)
                    && let Some(field_sig) = class_sig.fields.get(&key(name))
                {
                    if field_sig.is_shared || symbols.contains_key(well_known::SELF_KEY) {
                        if let Some(ref array_decl) = field_sig.array {
                            VarType::Array(
                                Visibility::Public,
                                field_sig.ty.clone(),
                                matches!(array_decl, ArrayDecl::Dynamic),
                            )
                        } else if field_sig.ty.same_type(&TypeName::Variant) {
                            VarType::Scalar(Visibility::Public, TypeName::Variant)
                        } else {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                format!("Variable '{}' is not an array", name),
                                Some(*span),
                            ));
                        }
                    } else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Instance array '{}' cannot be accessed from a Shared method",
                                name
                            ),
                            Some(*span),
                        ));
                    }
                } else if let Some(type_sig) = types.get(owner_name)
                    && let Some(field_sig) = type_sig.fields.get(&key(name))
                {
                    if let Some(ref array_decl) = field_sig.array {
                        VarType::Array(
                            Visibility::Public,
                            field_sig.ty.clone(),
                            matches!(array_decl, ArrayDecl::Dynamic),
                        )
                    } else if field_sig.ty.same_type(&TypeName::Variant) {
                        VarType::Scalar(Visibility::Public, TypeName::Variant)
                    } else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            format!("Variable '{}' is not an array", name),
                            Some(*span),
                        ));
                    }
                } else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Variable '{}' is not declared", name),
                        Some(*span),
                    ));
                }
            } else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(*span),
                ));
            };
            let element_type = match var_type {
                VarType::Array(_, ty, _) => ty,
                VarType::Scalar(_, TypeName::User(class_name))
                | VarType::Optional(_, TypeName::User(class_name))
                | VarType::Const(_, TypeName::User(class_name))
                    if class_name.eq_ignore_ascii_case(well_known::OBJECT)
                        || class_name.eq_ignore_ascii_case(well_known::COLLECTION) =>
                {
                    for index in indices {
                        validate_expr(index, symbols, types, signatures, context, options)?;
                    }
                    return Ok(TypeName::Variant);
                }
                VarType::Scalar(_, TypeName::User(class_name))
                | VarType::Optional(_, TypeName::User(class_name))
                | VarType::Const(_, TypeName::User(class_name))
                    if types
                        .get_class(&class_name)
                        .and_then(|class| class.default_property.as_ref())
                        .is_some() =>
                {
                    for index in indices {
                        validate_expr(index, symbols, types, signatures, context, options)?;
                    }
                    return Ok(value_type.clone());
                }
                VarType::Scalar(_, TypeName::Variant)
                | VarType::Optional(_, TypeName::Variant)
                | VarType::Const(_, TypeName::Variant) => {
                    for index in indices {
                        validate_expr(index, symbols, types, signatures, context, options)?;
                    }
                    return Ok(TypeName::Variant);
                }
                VarType::Module(alias) => {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::INVALID_QUALIFIED_ACCESS,
                        format!("Module '{}' cannot be indexed", alias),
                        Some(*span),
                    ));
                }
                _ => {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        format!("Variable '{}' is not an array", name),
                        Some(*span),
                    ));
                }
            };
            for index in indices {
                ensure_assignable(
                    &TypeName::Integer,
                    &validate_expr(index, symbols, types, signatures, context, options)?,
                    types,
                    index.span,
                )?;
            }
            Ok(element_type)
        }
        AssignTarget::Member {
            object,
            field,
            span,
        } => {
            if let ExprKind::Variable(class_name) = &object.kind
                && !symbols.contains_key(&key(class_name))
                && let Some(class_sig) = types.get_class(class_name)
                && let Some(field_sig) = class_sig.fields.get(&key(field))
                && field_sig.is_shared
            {
                return Ok(field_sig.ty.clone());
            }
            let object_type = validate_expr(object, symbols, types, signatures, context, options)?;
            let current_class = member_access_class(object, &object_type)
                .or_else(|| context.current_class().map(str::to_string));
            member_assignment_type(
                &object_type,
                field,
                value_type,
                &[],
                types,
                *span,
                current_class.as_deref(),
            )
        }
        AssignTarget::MemberArrayElement {
            object,
            field,
            indices,
            span,
        } => {
            let object_type = validate_expr(object, symbols, types, signatures, context, options)?;
            let mut index_types = Vec::with_capacity(indices.len());
            for index in indices {
                index_types.push(Some(validate_expr(
                    index, symbols, types, signatures, context, options,
                )?));
            }
            let current_class = member_access_class(object, &object_type)
                .or_else(|| context.current_class().map(str::to_string));
            member_assignment_type(
                &object_type,
                field,
                value_type,
                &index_types,
                types,
                *span,
                current_class.as_deref(),
            )
        }
    }
}

/// Reports whether a symbol can legitimately appear on the left of `(...)`
/// as an index or default-member access rather than as a call.
/// Returns the value type of a property referenced by its bare name.
///
/// The type comes from the `Get` accessor's return type, falling back to the
/// value parameter of `Let`/`Set` for a write-only property.
/// Reports whether a type can take part in a bitwise `And`/`Or`/`Xor`.
///
/// VB.NET allows every integral width here, not just `Integer`, plus enums and
/// the runtime-typed Variant.
/// Returns the same access with its `?.` guard removed, or `None` when the
/// expression is not a null-conditional access.
///
/// The guard only changes whether the access runs, never what it means, so
/// validation checks the plain form and marks the result nullable.
fn without_conditional_access(expr: &Expr) -> Option<Expr> {
    let kind = match &expr.kind {
        ExprKind::MemberAccess {
            object,
            field,
            conditional: true,
        } => ExprKind::MemberAccess {
            object: object.clone(),
            field: field.clone(),
            conditional: false,
        },
        ExprKind::MemberCall {
            object,
            method,
            type_args,
            args,
            conditional: true,
        } => ExprKind::MemberCall {
            object: object.clone(),
            method: method.clone(),
            type_args: type_args.clone(),
            args: args.clone(),
            conditional: false,
        },
        _ => return None,
    };
    Some(Expr {
        kind,
        span: expr.span,
    })
}

fn is_bitwise_operand(ty: &TypeName, types: &TypeRegistry) -> bool {
    ty.is_integral() || ty.same_type(&TypeName::Variant) || is_enum_type(ty, types)
}

/// Returns the type a bitwise operation produces for the given operands.
fn wider_bitwise_result(left: &TypeName, right: &TypeName) -> TypeName {
    let rank = |ty: &TypeName| match ty {
        TypeName::Byte => 0,
        TypeName::Integer => 1,
        TypeName::Long | TypeName::UInt32 => 2,
        TypeName::Int64 | TypeName::UInt64 => 3,
        _ => 1,
    };
    if !left.is_integral() && !right.is_integral() {
        return TypeName::Integer;
    }
    if !right.is_integral() || rank(left) >= rank(right) {
        if left.is_integral() {
            return left.clone();
        }
        return right.clone();
    }
    right.clone()
}

fn bare_property_type(property: &ClassPropertySig) -> Option<TypeName> {
    if let Some(get) = property.get.first()
        && let Some(return_type) = &get.return_type
    {
        return Some(return_type.clone());
    }
    property
        .writer()
        .and_then(|accessor| accessor.params.last())
        .map(|param| param.ty.clone())
}

fn is_indexable_var_type(var_type: &VarType) -> bool {
    match var_type {
        VarType::Array(..) => true,
        VarType::Scalar(_, ty) | VarType::Optional(_, ty) | VarType::Const(_, ty) => {
            matches!(
                ty,
                TypeName::User(_) | TypeName::GenericInstance { .. } | TypeName::Variant
            )
        }
        _ => false,
    }
}

fn unknown_variable(name: &str, span: Span, symbols: &HashMap<String, VarType>) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
        format!("Variable '{}' is not declared", name),
        Some(span),
    )
    .with_primary_label("unknown variable")
    .with_help("declare the variable before using it")
    .with_name_suggestion(name, symbols.keys().map(String::as_str))
}

pub(super) fn validate_expr(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
    options: Options,
) -> Result<TypeName, Diagnostic> {
    // A `?.` access answers Nothing whenever its receiver is Nothing, so it is
    // checked as an ordinary access and its result is made nullable.
    if let Some(unconditional) = without_conditional_access(expr) {
        let inner = validate_expr(&unconditional, symbols, types, signatures, context, options)?;
        return Ok(match inner {
            TypeName::Nullable(_) => inner,
            other => TypeName::Nullable(Box::new(other)),
        });
    }

    match &expr.kind {
        ExprKind::String(_) => Ok(TypeName::String),
        ExprKind::Query {
            variable,
            source,
            clauses,
        } => {
            let element_type = super::validate_statements::enumerable_element_type(
                source, symbols, types, signatures, context, options,
            )?;

            // The range variable is in scope for the clauses and nowhere else,
            // so it goes into a copy of the symbols rather than the caller's.
            let mut scoped = symbols.clone();
            scoped.insert(
                key(variable),
                VarType::Scalar(Visibility::Public, element_type),
            );
            for clause in clauses {
                let inner = match clause {
                    crate::QueryClause::Where(condition) => Some(condition),
                    crate::QueryClause::OrderBy { key: sort_key, .. } => Some(sort_key),
                    crate::QueryClause::Take(count) | crate::QueryClause::Skip(count) => {
                        Some(count)
                    }
                    crate::QueryClause::Select(projection) => Some(projection),
                    crate::QueryClause::Distinct => None,
                };
                if let Some(inner) = inner {
                    validate_expr(inner, &scoped, types, signatures, context, options)?;
                }
            }
            Ok(TypeName::User(well_known::COLLECTION.to_string()))
        }
        ExprKind::TupleLiteral(elements) => {
            if elements.len() < 2 {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    "A tuple needs at least two elements",
                    Some(expr.span),
                ));
            }
            let mut seen: Vec<&str> = Vec::new();
            let mut tuple = Vec::with_capacity(elements.len());
            for element in elements {
                if let Some(name) = &element.name {
                    if seen.iter().any(|other| other.eq_ignore_ascii_case(name)) {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                            format!("Tuple element '{name}' is named twice"),
                            Some(element.value.span),
                        ));
                    }
                    seen.push(name);
                }
                tuple.push(crate::runtime::TupleElement {
                    name: element.name.clone(),
                    ty: validate_expr(
                        &element.value,
                        symbols,
                        types,
                        signatures,
                        context,
                        options,
                    )?,
                });
            }
            Ok(TypeName::Tuple(tuple))
        }
        ExprKind::Interpolated(parts) => {
            for part in parts {
                if let crate::InterpolationPart::Value { expr, .. } = part {
                    validate_expr(expr, symbols, types, signatures, context, options)?;
                }
            }
            Ok(TypeName::String)
        }
        ExprKind::Convert {
            expr: value_expr,
            target,
            kind,
        } => {
            validate_expr(value_expr, symbols, types, signatures, context, options)?;
            ensure_known_type(target, types, expr.span)?;
            // TryCast answers Nothing when the value is not of the target type,
            // so its result is only meaningful for reference types.
            if matches!(kind, crate::ConversionKind::Try) && !is_class_type(target, types) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "TryCast requires a reference type, found '{}'",
                        target.display_name()
                    ),
                    Some(expr.span),
                )
                .with_help("use CType for value types"));
            }
            Ok(types.canonical_type_name(target))
        }
        ExprKind::GetType(target) => {
            ensure_known_type(target, types, expr.span)?;
            Ok(TypeName::String)
        }
        ExprKind::NameOf(_) => Ok(TypeName::String),
        ExprKind::DateLiteral(_) => Ok(TypeName::Date),
        ExprKind::Integer(value) => {
            let val = *value;
            if val >= i16::MIN as i64 && val <= i16::MAX as i64 {
                Ok(TypeName::Integer)
            } else if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
                Ok(TypeName::Long)
            } else {
                Ok(TypeName::Int64)
            }
        }
        ExprKind::Long(_) => Ok(TypeName::Long),
        ExprKind::LongLong(_) => Ok(TypeName::Int64),
        ExprKind::Single(_) => Ok(TypeName::Single),
        ExprKind::Double(_) => Ok(TypeName::Double),
        ExprKind::Currency(_) => Ok(TypeName::Currency),
        ExprKind::Decimal(_) => Ok(TypeName::Decimal),
        ExprKind::Boolean(_) => Ok(TypeName::Boolean),
        ExprKind::Nothing | ExprKind::Empty | ExprKind::Null => Ok(TypeName::Variant),
        ExprKind::Missing => Ok(TypeName::Variant),
        ExprKind::NamedArg { .. } => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "Named arguments are only valid inside call argument lists",
            Some(expr.span),
        )),
        ExprKind::TypeOfIs {
            expr: object,
            class_name,
        } => {
            let class = types.get_class(class_name).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Class '{}' is not defined", class_name),
                    Some(expr.span),
                )
            })?;
            let object_type = validate_expr(object, symbols, types, signatures, context, options)?;
            if is_object_reference_expr(object, &object_type, types) {
                Ok(TypeName::Boolean)
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "TypeOf requires a class object; '{}' is a class",
                        class.name
                    ),
                    Some(object.span),
                ))
            }
        }
        ExprKind::Me => match symbols.get(well_known::SELF_KEY).cloned() {
            Some(VarType::Scalar(_, ty))
            | Some(VarType::Optional(_, ty))
            | Some(VarType::Const(_, ty)) => Ok(ty),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Me is only valid inside class methods",
                Some(expr.span),
            )),
        },
        ExprKind::MyBase | ExprKind::MyClass => match symbols.get(well_known::SELF_KEY).cloned() {
            Some(VarType::Scalar(_, ty))
            | Some(VarType::Optional(_, ty))
            | Some(VarType::Const(_, ty)) => Ok(ty),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "MyBase and MyClass are only valid inside class methods",
                Some(expr.span),
            )),
        },
        ExprKind::WithTarget => Ok(TypeName::Variant),
        ExprKind::New {
            class_name,
            args,
            initializer,
            member_initializer,
        } => {
            if let Some(init) = initializer {
                for item in init {
                    validate_expr(item, symbols, types, signatures, context, options)?;
                }
            }

            if let TypeName::User(name) = class_name
                && name.eq_ignore_ascii_case(well_known::COLLECTION)
            {
                return Ok(TypeName::User("Collection".to_string()));
            }
            let class_name = resolve_new_type_name(class_name, symbols, types, expr.span)?;
            ensure_known_type(&class_name, types, expr.span)?;
            let (base_name, bindings) = generic_bindings_for_type(&class_name, types);
            if let Some(type_sig) = types.get(&base_name) {
                if !type_sig.is_structure {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!(
                            "Type '{}' cannot be constructed with New; use Structure",
                            type_sig.name
                        ),
                        Some(expr.span),
                    ));
                }
                if let Some(candidates) = well_known::find_constructor(&type_sig.subs) {
                    let validation =
                        ExprValidation::new(symbols, types, signatures, context, options);
                    let init =
                        resolve_overload("Sub", "New", candidates, args, expr.span, validation)?
                            .substitute_generics(&bindings);
                    validate_arguments("Sub", &init, args, expr.span, validation)?;
                } else if !args.is_empty() {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        format!("Structure '{}' has no Sub New constructor", type_sig.name),
                        Some(expr.span),
                    ));
                }
                return Ok(types.canonical_type_name(&class_name));
            }
            // `New T()` inside `Of T As New`. Which type T stands for is only
            // known at the call, and that is where the type argument was
            // checked for a parameterless constructor.
            if types.can_construct_generic(&base_name) {
                if !args.is_empty() {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
                        format!("'{base_name}' is constrained to New, which takes no arguments"),
                        Some(expr.span),
                    ));
                }
                return Ok(TypeName::User(base_name));
            }
            let class_sig = types.get_class(&base_name).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!(
                        "Class or Structure '{}' is not defined",
                        class_name.display_name()
                    ),
                    Some(expr.span),
                )
            })?;
            if let Some(candidates) = well_known::find_constructor(&class_sig.subs) {
                // `New Box(Of String)("x")` must check the argument against
                // String, not against the class's unbound `T`.
                let validation = ExprValidation::new(symbols, types, signatures, context, options);
                let init = resolve_overload("Sub", "New", candidates, args, expr.span, validation)?
                    .substitute_generics(&bindings);
                validate_arguments("Sub", &init, args, expr.span, validation)?;
            } else if !args.is_empty() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("Class '{}' has no Initialize constructor", class_sig.name),
                    Some(expr.span),
                ));
            }

            if let Some(inits) = member_initializer {
                let class_sig = class_sig.clone();
                for init in inits {
                    let value_type =
                        validate_expr(&init.value, symbols, types, signatures, context, options)?;
                    let member_type = class_sig
                        .fields
                        .get(&key(&init.name))
                        .map(|field| field.ty.clone())
                        .or_else(|| {
                            class_sig
                                .properties
                                .get(&key(&init.name))
                                .and_then(bare_property_type)
                        })
                        .ok_or_else(|| {
                            Diagnostic::new(
                                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                format!(
                                    "Class '{}' has no field or property '{}'",
                                    class_sig.name, init.name
                                ),
                                Some(init.span),
                            )
                            .with_primary_label("unknown member")
                        })?;
                    ensure_assignable(
                        &member_type.substitute_generics(&bindings),
                        &value_type,
                        types,
                        init.value.span,
                    )?;
                }
            }

            Ok(types.canonical_type_name(&class_name))
        }
        ExprKind::Variable(name) => {
            if let Some(var_type) = symbols.get(&key(name)).cloned() {
                match var_type {
                    VarType::Scalar(_, ty)
                    | VarType::Optional(_, ty)
                    | VarType::Const(_, ty)
                    | VarType::FunctionReturn(ty) => {
                        return Ok(ty);
                    }
                    VarType::Array(..) => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            format!("Array variable '{}' cannot be used as a scalar", name),
                            Some(expr.span),
                        ));
                    }
                    VarType::Module(alias) => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::INVALID_QUALIFIED_ACCESS,
                            format!("Module '{}' cannot be used as an expression", alias),
                            Some(expr.span),
                        ));
                    }
                }
            }
            if name.eq_ignore_ascii_case(well_known::ERR) {
                return Ok(TypeName::Variant);
            }
            if name.eq_ignore_ascii_case("Erl") {
                return Ok(TypeName::Integer);
            }
            if name.eq_ignore_ascii_case("FreeFile") {
                return Ok(TypeName::Integer);
            }
            if name.eq_ignore_ascii_case("Timer") || name.eq_ignore_ascii_case("Rnd") {
                return Ok(TypeName::Double);
            }
            if name.eq_ignore_ascii_case("Now")
                || name.eq_ignore_ascii_case("Date")
                || name.eq_ignore_ascii_case("Time")
            {
                return Ok(TypeName::Date);
            }
            if name.eq_ignore_ascii_case(well_known::CONSOLE) {
                return Ok(TypeName::Variant);
            }
            if name.eq_ignore_ascii_case(well_known::VBA) {
                return Ok(TypeName::Variant);
            }
            if let Some(constant) = crate::runtime::vba::vba_constant(name) {
                return Ok(constant.type_name());
            }
            if let Some(candidates) = signatures.functions.get(&key(name)) {
                let validation = ExprValidation::new(symbols, types, signatures, context, options);
                let function =
                    resolve_overload("Function", name, candidates, &[], expr.span, validation)?;
                validate_arguments("Function", function, &[], expr.span, validation)?;

                return Ok(function.return_type.clone().expect("function return type"));
            }
            if let Some(owner_name) = context.current_class() {
                if let Some(class_sig) = types.get_class(owner_name) {
                    let member_key = key(name);
                    if let Some(field_sig) = class_sig.fields.get(&member_key)
                        && (field_sig.is_shared || symbols.contains_key(well_known::SELF_KEY))
                    {
                        if field_sig.array.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                format!("Array variable '{}' cannot be used as a scalar", name),
                                Some(expr.span),
                            ));
                        }
                        return Ok(field_sig.ty.clone());
                    }
                    if let Some(candidates) = class_sig.functions.get(&member_key)
                        && candidates
                            .iter()
                            .any(|sig| sig.is_shared || symbols.contains_key(well_known::SELF_KEY))
                    {
                        let validation =
                            ExprValidation::new(symbols, types, signatures, context, options);
                        let func_sig = resolve_overload(
                            "Function",
                            name,
                            candidates,
                            &[],
                            expr.span,
                            validation,
                        )?;
                        validate_arguments("Function", func_sig, &[], expr.span, validation)?;
                        return Ok(func_sig.return_type.clone().expect("function return type"));
                    }
                    if let Some(prop_sig) = class_sig.properties.get(&member_key)
                        && (prop_sig.is_shared || symbols.contains_key(well_known::SELF_KEY))
                        && let Some(get) = prop_sig.get.first()
                    {
                        let callable = CallableSig {
                            attributes: Vec::new(),
                            visibility: Visibility::Public,
                            name: prop_sig.name.clone(),
                            type_params: Vec::new(),
                            generic_constraints: Vec::new(),
                            is_shared: prop_sig.is_shared,
                            _is_iterator: get.is_iterator,
                            is_declare: false,
                            params: get.params.clone(),
                            return_type: get.return_type.clone(),
                        };
                        validate_arguments(
                            "Property",
                            &callable,
                            &[],
                            expr.span,
                            ExprValidation::new(symbols, types, signatures, context, options),
                        )?;
                        return Ok(get.return_type.clone().unwrap_or(TypeName::Variant));
                    }
                }
                if let Some(type_sig) = types.get(owner_name) {
                    let member_key = key(name);
                    if let Some(field_sig) = type_sig.fields.get(&member_key) {
                        if field_sig.array.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                format!("Array variable '{}' cannot be used as a scalar", name),
                                Some(expr.span),
                            ));
                        }
                        return Ok(field_sig.ty.clone());
                    }
                    if let Some(candidates) = type_sig.functions.get(&member_key) {
                        let validation =
                            ExprValidation::new(symbols, types, signatures, context, options);
                        let func_sig = resolve_overload(
                            "Function",
                            name,
                            candidates,
                            &[],
                            expr.span,
                            validation,
                        )?;
                        validate_arguments("Function", func_sig, &[], expr.span, validation)?;
                        return Ok(func_sig.return_type.clone().expect("function return type"));
                    }
                    if let Some(prop_sig) = type_sig.properties.get(&member_key)
                        && let Some(get) = prop_sig.get.first()
                    {
                        let callable = CallableSig {
                            attributes: Vec::new(),
                            visibility: Visibility::Public,
                            name: prop_sig.name.clone(),
                            type_params: Vec::new(),
                            generic_constraints: Vec::new(),
                            is_shared: prop_sig.is_shared,
                            _is_iterator: get.is_iterator,
                            is_declare: false,
                            params: get.params.clone(),
                            return_type: get.return_type.clone(),
                        };
                        validate_arguments(
                            "Property",
                            &callable,
                            &[],
                            expr.span,
                            ExprValidation::new(symbols, types, signatures, context, options),
                        )?;
                        return Ok(get.return_type.clone().unwrap_or(TypeName::Variant));
                    }
                }
            }
            // Without Option Explicit an unknown name is a fresh Variant, but a
            // name that matches a member of the enclosing type is that member, so
            // its real type has to be worked out before falling back.
            if !options.explicit {
                return Ok(TypeName::Variant);
            }
            if enum_member_value_type(name, types).is_some() {
                Ok(TypeName::Integer)
            } else if name.to_ascii_lowercase().starts_with("vb") {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("VBA runtime constant '{}' is unknown or unsupported", name),
                    Some(expr.span),
                ))
            } else if name.to_ascii_lowercase().starts_with("mso") {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!(
                        "Office/COM constant '{}' is not part of Valo core runtime constants",
                        name
                    ),
                    Some(expr.span),
                ))
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(expr.span),
                ))
            }
        }
        ExprKind::MemberAccess { object, field, .. } => {
            if let ExprKind::Variable(name) = &object.kind
                && name.eq_ignore_ascii_case(well_known::ERR)
            {
                if field.eq_ignore_ascii_case("Number") {
                    return Ok(TypeName::Integer);
                }
                if field.eq_ignore_ascii_case("Description")
                    || field.eq_ignore_ascii_case("Source")
                    || field.eq_ignore_ascii_case("HelpFile")
                {
                    return Ok(TypeName::String);
                }
                if field.eq_ignore_ascii_case("HelpContext") {
                    return Ok(TypeName::Integer);
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("Err has no member '{}'", field),
                    Some(expr.span),
                ));
            }
            if let ExprKind::Variable(name) = &object.kind
                && name.eq_ignore_ascii_case(well_known::VBA)
            {
                if let Some(constant) = crate::runtime::vba::vba_constant(field) {
                    return Ok(constant.type_name());
                }
                if field.eq_ignore_ascii_case("Timer") || field.eq_ignore_ascii_case("Rnd") {
                    return Ok(TypeName::Double);
                }
                if field.eq_ignore_ascii_case("Now")
                    || field.eq_ignore_ascii_case("Date")
                    || field.eq_ignore_ascii_case("Time")
                {
                    return Ok(TypeName::Date);
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Module 'VBA' has no member '{}'", field),
                    Some(expr.span),
                ));
            }
            if let ExprKind::Variable(enum_name) = &object.kind
                && let Some(enum_sig) = types.get_enum(enum_name)
            {
                if enum_sig.members.contains_key(&key(field)) {
                    return Ok(TypeName::Integer);
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("Enum '{}' has no member '{}'", enum_sig.name, field),
                    Some(expr.span),
                ));
            }
            if let ExprKind::Variable(class_name) = &object.kind
                && !symbols.contains_key(&key(class_name))
                && let Some(class_sig) = types.get_class(class_name)
            {
                if let Some(field_sig) = class_sig.fields.get(&key(field)) {
                    return Ok(field_sig.ty.clone());
                }
                if let Some(property_sig) = class_sig.properties.get(&key(field))
                    && let Some(get) = property_sig.get.first()
                {
                    return Ok(get.return_type.clone().unwrap_or(TypeName::Variant));
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!(
                        "Class '{}' has no Shared member '{}'",
                        class_sig.name, field
                    ),
                    Some(expr.span),
                ));
            }
            let object_type = validate_expr(object, symbols, types, signatures, context, options)?;
            // Late binding is `member_read_type`'s to allow or refuse, so the
            // dynamic cases go through it rather than short-circuiting here.
            if !options.strict
                && (object_type.same_type(&TypeName::Variant)
                    || matches!(&object_type, TypeName::User(name) if name.eq_ignore_ascii_case(well_known::OBJECT)))
            {
                return Ok(TypeName::Variant);
            }
            let current_class = member_access_class(object, &object_type);
            member_read_type(
                &object_type,
                field,
                types,
                options,
                expr.span,
                current_class.as_deref(),
            )
        }
        ExprKind::MemberCall {
            object,
            method,
            type_args: _,
            args,
            ..
        } => {
            if let ExprKind::Variable(name) = &object.kind
                && name.eq_ignore_ascii_case(well_known::VBA)
            {
                if let Some(ty) = validate_builtin_function(
                    method,
                    args,
                    expr.span,
                    ExprValidation::new(symbols, types, signatures, context, options),
                )? {
                    return Ok(ty);
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Module 'VBA' has no member '{}'", method),
                    Some(expr.span),
                ));
            }
            if let ExprKind::Variable(name) = &object.kind
                && name.eq_ignore_ascii_case(well_known::ERR)
            {
                if method.eq_ignore_ascii_case("Clear") && args.is_empty() {
                    return Ok(TypeName::Variant);
                }
                if method.eq_ignore_ascii_case("Raise") {
                    validate_err_raise_args(
                        args, symbols, types, signatures, expr.span, context, options,
                    )?;
                    return Ok(TypeName::Variant);
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    "Err only supports Clear() and Raise()",
                    Some(expr.span),
                ));
            }
            if let ExprKind::Variable(class_name) = &object.kind
                && !symbols.contains_key(&key(class_name))
                && let Some(class_sig) = types.get_class(class_name)
            {
                if let Some(candidates) = class_sig.functions.get(&key(method)) {
                    let validation =
                        ExprValidation::new(symbols, types, signatures, context, options);
                    let function = resolve_overload(
                        "Function", method, candidates, args, expr.span, validation,
                    )?;
                    validate_arguments("Function", function, args, expr.span, validation)?;
                    return Ok(function.return_type.clone().unwrap_or(TypeName::Variant));
                }
                if let Some(property_sig) = class_sig.properties.get(&key(method))
                    && let Some(get) = property_sig.get.first()
                {
                    let callable = CallableSig {
                        attributes: Vec::new(),
                        visibility: Visibility::Public,
                        name: property_sig.name.clone(),
                        type_params: Vec::new(),
                        generic_constraints: Vec::new(),
                        is_shared: property_sig.is_shared,
                        _is_iterator: get.is_iterator,
                        is_declare: false,
                        params: get.params.clone(),
                        return_type: get.return_type.clone(),
                    };
                    validate_arguments(
                        "Property",
                        &callable,
                        args,
                        expr.span,
                        ExprValidation::new(symbols, types, signatures, context, options),
                    )?;
                    return Ok(get.return_type.clone().unwrap_or(TypeName::Variant));
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!(
                        "Class '{}' has no Shared method '{}'",
                        class_sig.name, method
                    ),
                    Some(expr.span),
                ));
            }
            let object_type = validate_expr(object, symbols, types, signatures, context, options)?;
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
                context,
                options,
            )
        }
        ExprKind::Call {
            name,
            type_args,
            args,
        } => {
            if let Some(ty) = validate_builtin_function(
                name,
                args,
                expr.span,
                ExprValidation::new(symbols, types, signatures, context, options),
            )? {
                return Ok(ty);
            }
            if name.eq_ignore_ascii_case("IsMissing") {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
                        "IsMissing expects exactly one argument",
                        Some(expr.span),
                    ));
                }
                let ExprKind::Variable(param_name) = &args[0].kind else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        "IsMissing requires an optional parameter name",
                        Some(args[0].span),
                    ));
                };
                return match symbols.get(&key(param_name)) {
                    Some(VarType::Optional(Visibility::Public, _)) => Ok(TypeName::Boolean),
                    Some(_) => Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        "IsMissing is only valid for Optional parameters",
                        Some(args[0].span),
                    )),
                    None => Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Variable '{}' is not declared", param_name),
                        Some(args[0].span),
                    )),
                };
            }
            if name.eq_ignore_ascii_case("LBound") || name.eq_ignore_ascii_case("UBound") {
                if args.is_empty() || args.len() > 2 {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
                        format!("{} expects one array argument and optional dimension", name),
                        Some(expr.span),
                    ));
                }
                validate_array_expr(&args[0], symbols, types, signatures, context, options)?;
                if args.len() == 2 {
                    ensure_assignable(
                        &TypeName::Integer,
                        &validate_expr(&args[1], symbols, types, signatures, context, options)?,
                        types,
                        args[1].span,
                    )?;
                }
                return Ok(TypeName::Integer);
            }
            // A function's own name is in scope as its implicit return variable.
            // Calling it with arguments is recursion, not indexing, unless the
            // symbol really is an array or an indexable object.
            let recurses_through_own_name = symbols
                .get(&key(name))
                .is_some_and(|var_type| !is_indexable_var_type(var_type))
                && signatures.functions.contains_key(&key(name));
            if let Some(var_type) = symbols.get(&key(name)).cloned()
                && !recurses_through_own_name
            {
                match var_type {
                    // Indexing an array does not depend on its visibility. A
                    // `Private` one at module level used to fall past this and
                    // be reported as not an array at all.
                    VarType::Array(_, element_type, _) => {
                        for arg in args {
                            ensure_assignable(
                                &TypeName::Integer,
                                &validate_expr(arg, symbols, types, signatures, context, options)?,
                                types,
                                arg.span,
                            )?;
                        }
                        return Ok(element_type);
                    }
                    VarType::Scalar(_, TypeName::User(class_name))
                    | VarType::Optional(_, TypeName::User(class_name))
                    | VarType::Const(_, TypeName::User(class_name)) => {
                        // Calling a delegate is the point of having one, so it
                        // is checked like a call to the procedure it names.
                        if let Some(delegate) =
                            types.delegates.get(&key(class_name.as_str())).cloned()
                        {
                            let validation =
                                ExprValidation::new(symbols, types, signatures, context, options);
                            let kind = match delegate.return_type {
                                Some(_) => "Function",
                                None => "Sub",
                            };
                            validate_arguments(kind, &delegate, args, expr.span, validation)?;
                            return Ok(delegate.return_type.unwrap_or(TypeName::Variant));
                        }
                        if class_name.eq_ignore_ascii_case(well_known::OBJECT)
                            || class_name.eq_ignore_ascii_case(well_known::FUNC)
                        {
                            for arg in args {
                                validate_expr(arg, symbols, types, signatures, context, options)?;
                            }
                            return Ok(TypeName::Variant);
                        }
                        if let Some(type_sig) = types.get(&class_name)
                            && type_sig.is_structure
                            && let Some(default_prop_name) = &type_sig.default_property
                        {
                            return validate_method_call(
                                &TypeName::User(class_name.clone()),
                                default_prop_name,
                                args,
                                true,
                                expr.span,
                                symbols,
                                types,
                                signatures,
                                None,
                                context,
                                options,
                            );
                        }
                        if let Some(default_prop_name) = types
                            .get_class(&class_name)
                            .and_then(|c| c.default_property.as_ref())
                        {
                            return validate_method_call(
                                &TypeName::User(class_name.clone()),
                                default_prop_name,
                                args,
                                true,
                                expr.span,
                                symbols,
                                types,
                                signatures,
                                None,
                                context,
                                options,
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
                    v if v.is_variant() => {
                        for arg in args {
                            validate_expr(arg, symbols, types, signatures, context, options)?;
                        }
                        return Ok(TypeName::Variant);
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            format!(
                                "Variable '{}' of type '{}' is not an array",
                                name,
                                var_type.display_name()
                            ),
                            Some(expr.span),
                        ));
                    }
                }
            }
            if let Some(VarType::Scalar(Visibility::Public, TypeName::User(class_name))) =
                symbols.get(well_known::SELF_KEY).cloned()
                && let Some(class_sig) = types.get_class(&class_name)
                && let Some(field_sig) = class_sig.fields.get(&key(name))
            {
                if field_sig.array.is_some() {
                    for arg in args {
                        ensure_assignable(
                            &TypeName::Integer,
                            &validate_expr(arg, symbols, types, signatures, context, options)?,
                            types,
                            arg.span,
                        )?;
                    }
                    return Ok(field_sig.ty.clone());
                }
                if field_sig.ty.same_type(&TypeName::Variant) {
                    for arg in args {
                        validate_expr(arg, symbols, types, signatures, context, options)?;
                    }
                    return Ok(TypeName::Variant);
                }
            }

            let mut candidates = signatures.functions.get(&key(name)).cloned();
            if candidates.is_none()
                && let Some(owner_name) = context.current_class()
                && let Some(class_sig) = types.get_class(owner_name)
            {
                let member_key = key(name);
                if let Some(methods) = class_sig.functions.get(&member_key) {
                    if methods
                        .iter()
                        .any(|sig| sig.is_shared || symbols.contains_key(well_known::SELF_KEY))
                    {
                        candidates = Some(methods.clone());
                    }
                } else if let Some(prop_sig) = class_sig.properties.get(&member_key)
                    && let Some(get) = prop_sig.get.first()
                    && (prop_sig.is_shared || symbols.contains_key(well_known::SELF_KEY))
                {
                    candidates = Some(vec![CallableSig {
                        attributes: Vec::new(),
                        visibility: Visibility::Public,
                        name: prop_sig.name.clone(),
                        type_params: Vec::new(),
                        generic_constraints: Vec::new(),
                        is_shared: prop_sig.is_shared,
                        _is_iterator: get.is_iterator,
                        is_declare: false,
                        params: get.params.clone(),
                        return_type: get.return_type.clone(),
                    }]);
                }
            }

            let Some(candidates) = candidates else {
                if signatures.subs.contains_key(&key(name)) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!("Sub '{}' cannot be used as an expression", name),
                        Some(expr.span),
                    ));
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Function '{}' is not defined", name),
                    Some(expr.span),
                ));
            };

            let validation = ExprValidation::new(symbols, types, signatures, context, options);
            let function =
                resolve_overload("Function", name, &candidates, args, expr.span, validation)?
                    .clone();

            let inferred_type_args;
            let type_args = if type_args.is_empty() && !function.type_params.is_empty() {
                inferred_type_args = infer_callable_type_args(
                    &function,
                    args,
                    ExprValidation::new(symbols, types, signatures, context, options),
                    expr.span,
                )?;
                &inferred_type_args
            } else {
                type_args
            };
            let function = instantiate_callable(&function, type_args, expr.span, types)?;
            validate_arguments(
                "Function",
                &function,
                args,
                expr.span,
                ExprValidation::new(symbols, types, signatures, context, options),
            )?;
            Ok(function.return_type.clone().expect("function return type"))
        }
        ExprKind::Index { target, args } => {
            let _target_type = validate_expr(target, symbols, types, signatures, context, options)?;
            for arg in args {
                validate_expr(arg, symbols, types, signatures, context, options)?;
            }
            Ok(TypeName::Variant)
        }
        ExprKind::IIf {
            condition,
            true_expr,
            false_expr,
        } => {
            validate_expr(condition, symbols, types, signatures, context, options)?;
            let true_type = validate_expr(true_expr, symbols, types, signatures, context, options)?;
            let false_type =
                validate_expr(false_expr, symbols, types, signatures, context, options)?;
            if true_type.same_type(&false_type) {
                Ok(true_type)
            } else {
                Ok(TypeName::Variant)
            }
        }
        ExprKind::Binary { left, op, right } => {
            let left_type_raw = validate_expr(left, symbols, types, signatures, context, options)?;
            let right_type_raw =
                validate_expr(right, symbols, types, signatures, context, options)?;

            let is_nullable = matches!(left_type_raw, TypeName::Nullable(_))
                || matches!(right_type_raw, TypeName::Nullable(_));

            let left_type = if let TypeName::Nullable(inner) = &left_type_raw {
                (**inner).clone()
            } else {
                left_type_raw.clone()
            };

            let right_type = if let TypeName::Nullable(inner) = &right_type_raw {
                (**inner).clone()
            } else {
                right_type_raw.clone()
            };

            let operator_kind = match op {
                BinaryOp::Add => Some(crate::OperatorKind::Add),
                BinaryOp::Subtract => Some(crate::OperatorKind::Subtract),
                BinaryOp::Multiply => Some(crate::OperatorKind::Multiply),
                BinaryOp::Divide => Some(crate::OperatorKind::Divide),
                BinaryOp::IntegerDivide => Some(crate::OperatorKind::IntegerDivide),
                BinaryOp::Exponent => Some(crate::OperatorKind::Exponent),
                BinaryOp::Modulo => Some(crate::OperatorKind::Modulo),
                BinaryOp::LogicalAnd | BinaryOp::LogicalAndAlso => Some(crate::OperatorKind::And),
                BinaryOp::LogicalOr | BinaryOp::LogicalOrElse => Some(crate::OperatorKind::Or),
                BinaryOp::LogicalXor => Some(crate::OperatorKind::Xor),
                BinaryOp::Equal => Some(crate::OperatorKind::Equal),
                BinaryOp::NotEqual => Some(crate::OperatorKind::NotEqual),
                BinaryOp::Less => Some(crate::OperatorKind::Less),
                BinaryOp::Greater => Some(crate::OperatorKind::Greater),
                BinaryOp::LessEqual => Some(crate::OperatorKind::LessEqual),
                BinaryOp::GreaterEqual => Some(crate::OperatorKind::GreaterEqual),
                BinaryOp::Like => Some(crate::OperatorKind::Like),
                BinaryOp::Concat => Some(crate::OperatorKind::Concatenate),
                _ => None,
            };

            let wrap_nullable = |ty: TypeName| -> TypeName {
                if is_nullable {
                    TypeName::Nullable(Box::new(ty))
                } else {
                    ty
                }
            };

            if let Some(kind) = operator_kind
                && let Some(res_ty) =
                    find_overloaded_binary_operator(&left_type, kind, &right_type, types)
            {
                return Ok(wrap_nullable(res_ty));
            }

            match op {
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Exponent
                | BinaryOp::Divide
                | BinaryOp::IntegerDivide
                | BinaryOp::Modulo => {
                    if is_numeric_type(&left_type) && is_numeric_type(&right_type) {
                        let base_ty = if left_type.same_type(&TypeName::Double)
                            || right_type.same_type(&TypeName::Double)
                        {
                            TypeName::Double
                        } else if left_type.same_type(&TypeName::Single)
                            || right_type.same_type(&TypeName::Single)
                        {
                            TypeName::Single
                        } else {
                            TypeName::Int64
                        };
                        Ok(wrap_nullable(base_ty))
                    } else {
                        ensure_assignable(&TypeName::Int64, &left_type, types, left.span)?;
                        ensure_assignable(&TypeName::Int64, &right_type, types, right.span)?;
                        Ok(wrap_nullable(TypeName::Int64))
                    }
                }
                BinaryOp::Concat => Ok(wrap_nullable(TypeName::String)),
                // A shift keeps the left operand's type; the count only has to
                // be a whole number.
                BinaryOp::ShiftLeft | BinaryOp::ShiftRight => {
                    ensure_assignable(&TypeName::Int64, &left_type, types, left.span)?;
                    ensure_assignable(&TypeName::Int64, &right_type, types, right.span)?;
                    Ok(wrap_nullable(if left_type.is_integral() {
                        left_type
                    } else {
                        TypeName::Int64
                    }))
                }
                BinaryOp::LogicalAnd
                | BinaryOp::LogicalAndAlso
                | BinaryOp::LogicalOr
                | BinaryOp::LogicalOrElse
                | BinaryOp::LogicalXor
                | BinaryOp::LogicalEqv
                | BinaryOp::LogicalImp => {
                    if (left_type.same_type(&TypeName::Boolean)
                        || left_type.same_type(&TypeName::Variant))
                        && (right_type.same_type(&TypeName::Boolean)
                            || right_type.same_type(&TypeName::Variant))
                    {
                        Ok(TypeName::Boolean)
                    } else if is_bitwise_operand(&left_type, types)
                        && is_bitwise_operand(&right_type, types)
                    {
                        // A bitwise result keeps the wider operand's width, so
                        // masking a Long does not silently narrow to Integer.
                        Ok(wider_bitwise_result(&left_type, &right_type))
                    } else {
                        Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "Logical operators require Boolean or Integer operands",
                            Some(expr.span),
                        ))
                    }
                }
                BinaryOp::Equal | BinaryOp::NotEqual => Ok(TypeName::Boolean),
                BinaryOp::Like => {
                    ensure_assignable(&TypeName::String, &left_type, types, left.span)?;
                    ensure_assignable(&TypeName::String, &right_type, types, right.span)?;
                    Ok(TypeName::Boolean)
                }
                BinaryOp::Is | BinaryOp::IsNot => {
                    // Compare against the declared types: a `T?` operand was
                    // already unwrapped above, and `value Is Nothing` is the
                    // idiomatic emptiness test for a nullable.
                    if is_object_reference_expr(left, &left_type_raw, types)
                        && is_object_reference_expr(right, &right_type_raw, types)
                    {
                        Ok(TypeName::Boolean)
                    } else {
                        Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            format!(
                                "'{}' requires class object operands or Nothing",
                                match op {
                                    BinaryOp::Is => "Is",
                                    BinaryOp::IsNot => "IsNot",
                                    _ => unreachable!(),
                                }
                            ),
                            Some(expr.span),
                        ))
                    }
                }
                BinaryOp::Less
                | BinaryOp::Greater
                | BinaryOp::LessEqual
                | BinaryOp::GreaterEqual => {
                    // A Variant carries its type at runtime, so an ordering
                    // comparison involving one can only be resolved there.
                    if left_type.same_type(&TypeName::Variant)
                        || right_type.same_type(&TypeName::Variant)
                        || (is_numeric_type(&left_type) && is_numeric_type(&right_type))
                        || (left_type.same_type(&TypeName::String)
                            && right_type.same_type(&TypeName::String))
                    {
                        Ok(TypeName::Boolean)
                    } else {
                        Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "Comparison requires matching numeric or String operands",
                            Some(expr.span),
                        ))
                    }
                }
            }
        }
        ExprKind::Unary { op, expr: inner } => {
            let ty = validate_expr(inner, symbols, types, signatures, context, options)?;
            let operator_kind = match op {
                UnaryOp::Positive => Some(crate::OperatorKind::UnaryPlus),
                UnaryOp::Negate => Some(crate::OperatorKind::UnaryMinus),
                UnaryOp::LogicalNot => Some(crate::OperatorKind::Not),
            };
            if let Some(kind) = operator_kind
                && let Some(res_ty) = find_overloaded_unary_operator(kind, &ty, types)
            {
                return Ok(res_ty);
            }
            match op {
                UnaryOp::Positive | UnaryOp::Negate => {
                    if is_numeric_type(&ty) {
                        Ok(ty)
                    } else {
                        Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            format!(
                                "Unary '{}' requires a numeric expression",
                                if matches!(op, UnaryOp::Negate) {
                                    "-"
                                } else {
                                    "+"
                                }
                            ),
                            Some(inner.span),
                        ))
                    }
                }
                UnaryOp::LogicalNot => {
                    ensure_assignable(&TypeName::Boolean, &ty, types, inner.span)?;
                    Ok(TypeName::Boolean)
                }
            }
        }
        ExprKind::AddressOf(_) => {
            // Wait, AddressOf returns a FuncPtr or LongPtr!
            Ok(TypeName::FuncPtr)
        }
        ExprKind::Lambda { .. } => Ok(TypeName::User("Func".to_string())),
        ExprKind::Await(expr) => {
            if !context.allows_await() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Await is only allowed inside Async Sub or Async Function",
                    Some(expr.span),
                ));
            }
            validate_expr(expr, symbols, types, signatures, context, options)
        }
        ExprKind::PassingModeOverride { expr, .. } => {
            validate_expr(expr, symbols, types, signatures, context, options)
        }
    }
}

fn resolve_new_type_name(
    ty: &TypeName,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    span: crate::runtime::Span,
) -> Result<TypeName, Diagnostic> {
    let TypeName::User(name) = ty else {
        return Ok(ty.clone());
    };
    let Some((qualifier, member)) = name.split_once('.') else {
        return Ok(ty.clone());
    };
    let Some(VarType::Module(module_name)) = symbols.get(&key(qualifier)) else {
        return Ok(ty.clone());
    };
    let qualified_name = format!("{module_name}.{member}");
    if types
        .get(&qualified_name)
        .is_some_and(|type_sig| !type_sig.is_structure)
    {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::INVALID_QUALIFIED_ACCESS,
            format!("Qualified type '{qualified_name}' cannot be constructed with New"),
            Some(span),
        ));
    }
    if types.contains(&qualified_name) {
        Ok(TypeName::User(qualified_name))
    } else {
        Ok(ty.clone())
    }
}

pub(super) fn validate_array_expr(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    _signatures: &Signatures,
    _context: &Context<'_>,
    options: Options,
) -> Result<TypeName, Diagnostic> {
    match &expr.kind {
        ExprKind::Variable(name) => match symbols.get(&key(name)).cloned() {
            Some(VarType::Array(_, element_type, _)) => Ok(element_type),
            Some(v) if v.is_variant() || v.scalar_type().is_some_and(|ty| matches!(ty, TypeName::User(ref name) if name.eq_ignore_ascii_case(well_known::FUNC) || name.eq_ignore_ascii_case(well_known::OBJECT))) => {
                Ok(TypeName::Variant)
            }
            Some(VarType::Scalar(_, _ty))
            | Some(VarType::Optional(_, _ty))
            | Some(VarType::Const(_, _ty)) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                format!("Variable '{}' is not an array", name),
                Some(expr.span),
            )),
            Some(VarType::Module(alias)) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::INVALID_QUALIFIED_ACCESS,
                format!("Module '{}' cannot be used as an array", alias),
                Some(expr.span),
            )),
            Some(VarType::FunctionReturn(_)) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                format!("Variable '{}' is not an array", name),
                Some(expr.span),
            )),
            None => {
                if let Some(VarType::Scalar(_, TypeName::User(class_name))) =
                    symbols.get(well_known::SELF_KEY).cloned()
                    && let Some(class_sig) = types.get_class(&class_name)
                    && let Some(field_sig) = class_sig.fields.get(&key(name))
                {
                    if field_sig.array.is_some() {
                        return Ok(field_sig.ty.clone());
                    }
                    if field_sig.ty.same_type(&TypeName::Variant) {
                        return Ok(TypeName::Variant);
                    }
                }
                if !options.explicit {
                    return Ok(TypeName::Variant);
                }
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(expr.span),
                ))
            }
        },
        _ => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::PARSE,
            "Expected array variable",
            Some(expr.span),
        )),
    }
}

fn validate_builtin_function(
    name: &str,
    args: &[Expr],
    span: crate::runtime::Span,
    validation: ExprValidation<'_, '_>,
) -> Result<Option<TypeName>, Diagnostic> {
    let symbols = validation.symbols;
    let types = validation.types;
    let signatures = validation.signatures;
    let context = validation.context;
    let options = validation.options;

    let effective_name = strip_vba_namespace(name);
    let Some(builtin) = builtins::lookup(effective_name) else {
        return Ok(None);
    };

    builtin.check_arity(args.len(), span)?;

    // A few builtins constrain an argument beyond simply counting it.
    match builtin.name {
        "IsArray" => {
            // Answers whether the argument is an array, so a non-array argument
            // is a legitimate question rather than an error.
            if validate_array_expr(&args[0], symbols, types, signatures, context, options).is_err()
            {
                validate_expr(&args[0], symbols, types, signatures, context, options)?;
            }
        }
        "IsMissing" => {
            validate_is_missing_argument(&args[0], symbols)?;
        }
        // These take an array as their first argument, which the ordinary
        // expression rules would reject as "an array used as a scalar".
        "Filter" | "LBound" | "UBound" => {
            validate_array_expr(&args[0], symbols, types, signatures, context, options)?;
            for arg in &args[1..] {
                validate_expr(arg, symbols, types, signatures, context, options)?;
            }
        }
        _ => {
            for arg in args {
                validate_expr(arg, symbols, types, signatures, context, options)?;
            }
        }
    }

    Ok(Some(builtin.returns.type_name()))
}

pub(super) fn enum_member_value_type(name: &str, types: &TypeRegistry) -> Option<TypeName> {
    for enum_sig in types.enums.values() {
        if enum_sig.members.contains_key(&key(name)) {
            return Some(TypeName::Integer);
        }
    }
    None
}

/// Picks which of the procedures sharing a name a call means.
///
/// Typing the arguments is only worth doing when there is a choice to make, so
/// a name that means exactly one procedure returns it untouched, and the
/// argument checking that follows reports a mismatch far more precisely than
/// "nothing fits" ever could.
pub(super) fn resolve_overload<'a>(
    kind: &str,
    name: &str,
    candidates: &'a [CallableSig],
    args: &[Expr],
    span: Span,
    validation: ExprValidation<'_, '_>,
) -> Result<&'a CallableSig, Diagnostic> {
    if let [only] = candidates {
        return Ok(only);
    }

    // A named argument says which parameter it is for, so it selects by name
    // rather than by position. Candidates that have no such parameter are out.
    let named: Vec<&str> = args
        .iter()
        .filter_map(|arg| match &arg.kind {
            ExprKind::NamedArg { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let by_name: Vec<&CallableSig> = candidates
        .iter()
        .filter(|candidate| {
            named.iter().all(|wanted| {
                candidate
                    .params
                    .iter()
                    .any(|param| param.name.eq_ignore_ascii_case(wanted))
            })
        })
        .collect();
    if !named.is_empty() {
        return match by_name.as_slice() {
            [single] => Ok(single),
            [] => Err(no_overload_matches(kind, name, candidates, span)),
            _ => Err(ambiguous_overload(kind, name, &by_name, span)),
        };
    }

    let argument_types: Vec<Option<TypeName>> = args
        .iter()
        .map(|arg| {
            validate_expr(
                arg,
                validation.symbols,
                validation.types,
                validation.signatures,
                validation.context,
                validation.options,
            )
            .ok()
        })
        .collect();

    let shapes: Vec<Vec<overloads::ParamShape>> = candidates.iter().map(param_shapes).collect();
    match overloads::resolve(&shapes, &argument_types) {
        overloads::Resolution::Single(chosen) => Ok(&candidates[chosen]),
        overloads::Resolution::NoMatch => Err(no_overload_matches(kind, name, candidates, span)),
        overloads::Resolution::Ambiguous(tied) => {
            let tied: Vec<&CallableSig> =
                tied.into_iter().map(|index| &candidates[index]).collect();
            Err(ambiguous_overload(kind, name, &tied, span))
        }
    }
}

/// What overload resolution needs from a signature's parameters.
pub(super) fn param_shapes(sig: &CallableSig) -> Vec<overloads::ParamShape> {
    sig.params
        .iter()
        .map(|param| overloads::ParamShape {
            ty: param.ty.clone(),
            is_optional: param.is_optional,
            is_param_array: param.is_param_array,
        })
        .collect()
}

fn no_overload_matches(
    kind: &str,
    name: &str,
    candidates: &[CallableSig],
    span: Span,
) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
        format!("No overload of {kind} '{name}' accepts these arguments"),
        Some(span),
    )
    .with_available_items(
        "the overloads are",
        candidates
            .iter()
            .map(describe_signature)
            .collect::<Vec<_>>()
            .iter()
            .map(String::as_str),
    )
}

fn ambiguous_overload(kind: &str, name: &str, tied: &[&CallableSig], span: Span) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::AMBIGUOUS_OVERLOAD,
        format!("Call to {kind} '{name}' is ambiguous"),
        Some(span),
    )
    .with_available_items(
        "these fit equally well",
        tied.iter()
            .map(|sig| describe_signature(sig))
            .collect::<Vec<_>>()
            .iter()
            .map(String::as_str),
    )
    .with_help("convert an argument, or name the parameters, to say which is meant")
}

/// Renders a signature the way it was written, for listing in a diagnostic.
fn describe_signature(sig: &CallableSig) -> String {
    let params: Vec<String> = sig
        .params
        .iter()
        .map(|param| {
            let mut text = String::new();
            if param.is_param_array {
                text.push_str("ParamArray ");
            } else if param.is_optional {
                text.push_str("Optional ");
            }
            text.push_str(&param.name);
            text.push_str(" As ");
            text.push_str(&param.ty.display_name());
            text
        })
        .collect();

    match &sig.return_type {
        Some(return_type) => format!(
            "{}({}) As {}",
            sig.name,
            params.join(", "),
            return_type.display_name()
        ),
        None => format!("{}({})", sig.name, params.join(", ")),
    }
}

pub(super) fn validate_arguments(
    kind: &str,
    callable: &CallableSig,
    args: &[Expr],
    span: Span,
    validation: ExprValidation<'_, '_>,
) -> Result<(), Diagnostic> {
    let has_param_array = callable
        .params
        .last()
        .is_some_and(|param| param.is_param_array);
    let mut assigned = vec![false; callable.params.len()];
    let mut positional_index = 0;
    let mut saw_named = false;

    for arg in args {
        if let ExprKind::NamedArg { name, expr: value } = &arg.kind {
            saw_named = true;
            let Some(index) = callable
                .params
                .iter()
                .position(|param| param.name.eq_ignore_ascii_case(name))
            else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::NAMED_ARGUMENT,
                    format!(
                        "{} '{}' has no parameter named '{}'",
                        kind, callable.name, name
                    ),
                    Some(arg.span),
                ));
            };
            if assigned[index] {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::NAMED_ARGUMENT,
                    format!("Argument '{}' is specified more than once", name),
                    Some(arg.span),
                ));
            }
            let param = &callable.params[index];
            if param.is_param_array {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    "ParamArray arguments cannot be supplied by name",
                    Some(arg.span),
                ));
            }
            validate_argument_value(param, value, validation)?;
            assigned[index] = true;
            continue;
        }
        if saw_named {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
                "Positional arguments cannot appear after named arguments",
                Some(arg.span),
            ));
        }
        let Some(param) = callable
            .params
            .get(positional_index)
            .or_else(|| callable.params.last().filter(|param| param.is_param_array))
        else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
                format!(
                    "{} '{}' expects {} argument(s), got {}",
                    kind,
                    callable.name,
                    callable.params.len(),
                    args.len()
                ),
                Some(span),
            ));
        };
        validate_argument_value(param, arg, validation)?;
        if !param.is_param_array {
            assigned[positional_index] = true;
            positional_index += 1;
        }
    }

    let missing_required = callable
        .params
        .iter()
        .enumerate()
        .any(|(index, param)| !assigned[index] && !param.is_optional && !param.is_param_array);
    if missing_required || (!has_param_array && args.len() > callable.params.len()) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
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

    Ok(())
}

/// Checks that `IsMissing` is asked about an optional parameter.
///
/// Anything else is always present, so the question has no meaning and is far
/// more likely to be a mistake than an intent.
fn validate_is_missing_argument(
    arg: &Expr,
    symbols: &HashMap<String, VarType>,
) -> Result<(), Diagnostic> {
    let ExprKind::Variable(name) = &arg.kind else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "IsMissing is only valid for Optional parameters",
            Some(arg.span),
        ));
    };
    if let Some(var_type) = symbols.get(&key(name))
        && !matches!(var_type, VarType::Optional(Visibility::Public, _))
    {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "IsMissing is only valid for Optional parameters",
            Some(arg.span),
        ));
    }
    Ok(())
}
fn validate_argument_value(
    param: &ParamSig,
    arg: &Expr,
    validation: ExprValidation<'_, '_>,
) -> Result<(), Diagnostic> {
    if param.is_param_array {
        let arg_type = validate_expr(
            arg,
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.options,
        )?;
        ensure_assignable_expr(
            &TypeName::Variant,
            &arg_type,
            arg,
            validation.types,
            validation.options,
            arg.span,
        )?;
        return Ok(());
    }
    match param.mode {
        PassingMode::ByVal => {
            let arg_type = validate_expr(
                arg,
                validation.symbols,
                validation.types,
                validation.signatures,
                validation.context,
                validation.options,
            )?;
            ensure_assignable_expr(
                &param.ty,
                &arg_type,
                arg,
                validation.types,
                validation.options,
                arg.span,
            )
        }
        PassingMode::ByRef => {
            let arg_type = validate_expr(
                arg,
                validation.symbols,
                validation.types,
                validation.signatures,
                validation.context,
                validation.options,
            )?;
            ensure_assignable_expr(
                &param.ty,
                &arg_type,
                arg,
                validation.types,
                validation.options,
                arg.span,
            )
        }
    }
}

fn instantiate_callable(
    callable: &CallableSig,
    type_args: &[TypeName],
    span: Span,
    types: &TypeRegistry,
) -> Result<CallableSig, Diagnostic> {
    if callable.type_params.is_empty() {
        if !type_args.is_empty() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                format!("'{}' is not generic", callable.name),
                Some(span),
            ));
        }
        return Ok(callable.clone());
    }
    if callable.type_params.len() != type_args.len() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!(
                "Type parameter count mismatch for {}. Expected {}, received {}",
                callable.name,
                callable.type_params.len(),
                type_args.len()
            ),
            Some(span),
        ));
    }
    validate_generic_constraints(
        &callable.name,
        &callable.type_params,
        &callable.generic_constraints,
        type_args,
        types,
        span,
    )?;
    let bindings = callable
        .type_params
        .iter()
        .cloned()
        .zip(type_args.iter().cloned())
        .collect::<Vec<_>>();
    let mut instantiated = callable.clone();
    instantiated.type_params.clear();
    for param in &mut instantiated.params {
        param.ty = param.ty.substitute_generics(&bindings);
    }
    instantiated.return_type = instantiated
        .return_type
        .map(|ty| ty.substitute_generics(&bindings));
    Ok(instantiated)
}

fn infer_callable_type_args(
    callable: &CallableSig,
    args: &[Expr],
    validation: ExprValidation<'_, '_>,
    span: Span,
) -> Result<Vec<TypeName>, Diagnostic> {
    let mut inferred: Vec<Option<TypeName>> = vec![None; callable.type_params.len()];
    let mut positional_index = 0;
    for arg in args {
        let (param, arg_expr) = if let ExprKind::NamedArg { name, expr } = &arg.kind {
            let Some(param) = callable
                .params
                .iter()
                .find(|param| param.name.eq_ignore_ascii_case(name))
            else {
                continue;
            };
            (param, expr.as_ref())
        } else {
            let Some(param) = callable.params.get(positional_index) else {
                continue;
            };
            positional_index += 1;
            (param, arg)
        };
        let Some(arg_type) = infer_expr_type_for_generic(
            arg_expr,
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.options,
        )?
        else {
            continue;
        };
        collect_generic_type_inferences(
            &param.ty,
            &arg_type,
            &callable.type_params,
            &mut inferred,
            arg.span,
        )?;
    }

    inferred
        .into_iter()
        .enumerate()
        .map(|(index, ty)| {
            ty.ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Cannot infer type argument '{}' for '{}'",
                        callable.type_params[index], callable.name
                    ),
                    Some(span),
                )
                .with_help("specify the type argument explicitly with '(Of ...)'")
            })
        })
        .collect()
}

fn infer_expr_type_for_generic(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
    options: Options,
) -> Result<Option<TypeName>, Diagnostic> {
    match &expr.kind {
        ExprKind::String(_) => Ok(Some(TypeName::String)),
        ExprKind::DateLiteral(_) => Ok(Some(TypeName::Date)),
        ExprKind::Integer(value) => {
            let ty = if *value >= i16::MIN as i64 && *value <= i16::MAX as i64 {
                TypeName::Integer
            } else if *value >= i32::MIN as i64 && *value <= i32::MAX as i64 {
                TypeName::Long
            } else {
                TypeName::Int64
            };
            Ok(Some(ty))
        }
        ExprKind::Long(_) => Ok(Some(TypeName::Long)),
        ExprKind::LongLong(_) => Ok(Some(TypeName::Int64)),
        ExprKind::Single(_) => Ok(Some(TypeName::Single)),
        ExprKind::Double(_) => Ok(Some(TypeName::Double)),
        ExprKind::Currency(_) => Ok(Some(TypeName::Currency)),
        ExprKind::Decimal(_) => Ok(Some(TypeName::Decimal)),
        ExprKind::Boolean(_) => Ok(Some(TypeName::Boolean)),
        ExprKind::Variable(name) => Ok(symbols.get(&key(name)).and_then(VarType::scalar_type)),
        ExprKind::New { class_name, .. } => Ok(Some(types.canonical_type_name(class_name))),
        ExprKind::NamedArg { expr, .. } | ExprKind::PassingModeOverride { expr, .. } => {
            infer_expr_type_for_generic(expr, symbols, types, signatures, context, options)
        }
        ExprKind::Nothing | ExprKind::Empty | ExprKind::Null | ExprKind::Missing => {
            Ok(Some(TypeName::Variant))
        }
        ExprKind::AddressOf(_) => Ok(Some(TypeName::FuncPtr)),
        ExprKind::Me | ExprKind::MyBase | ExprKind::MyClass => {
            validate_expr(expr, symbols, types, signatures, context, options).map(Some)
        }
        _ => Ok(None),
    }
}

fn collect_generic_type_inferences(
    param_type: &TypeName,
    arg_type: &TypeName,
    type_params: &[String],
    inferred: &mut [Option<TypeName>],
    span: Span,
) -> Result<(), Diagnostic> {
    match param_type {
        TypeName::User(name) => {
            if let Some(index) = type_params
                .iter()
                .position(|param| param.eq_ignore_ascii_case(name))
            {
                merge_inferred_type(&mut inferred[index], arg_type.clone(), name, span)?;
            }
        }
        TypeName::Array(param_inner) => {
            if let TypeName::Array(arg_inner) = arg_type {
                collect_generic_type_inferences(
                    param_inner,
                    arg_inner,
                    type_params,
                    inferred,
                    span,
                )?;
            }
        }
        TypeName::GenericInstance {
            name: param_name,
            args: param_args,
        } => {
            if let TypeName::GenericInstance {
                name: arg_name,
                args: arg_args,
            } = arg_type
                && param_name.eq_ignore_ascii_case(arg_name)
            {
                for (param_arg, arg_arg) in param_args.iter().zip(arg_args.iter()) {
                    collect_generic_type_inferences(
                        param_arg,
                        arg_arg,
                        type_params,
                        inferred,
                        span,
                    )?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn merge_inferred_type(
    slot: &mut Option<TypeName>,
    inferred_type: TypeName,
    param_name: &str,
    span: Span,
) -> Result<(), Diagnostic> {
    if let Some(existing) = slot {
        if !existing.same_type(&inferred_type) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                format!(
                    "Conflicting inferred types for '{}': '{}' and '{}'",
                    param_name,
                    existing.display_name(),
                    inferred_type.display_name()
                ),
                Some(span),
            ));
        }
    } else {
        *slot = Some(inferred_type);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_method_call(
    object_type: &TypeName,
    method: &str,
    args: &[Expr],
    as_expression: bool,
    span: crate::runtime::Span,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    current_class: Option<&str>,
    context: &Context<'_>,
    options: Options,
) -> Result<TypeName, Diagnostic> {
    if object_type.same_type(&TypeName::Variant)
        || matches!(object_type, TypeName::User(name) if name.eq_ignore_ascii_case(well_known::OBJECT))
    {
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, options)?;
        }
        return Ok(TypeName::Variant);
    }
    // A constrained type parameter answers with whatever its bound has, the
    // same way reading a member of one does.
    if let TypeName::User(name) = object_type
        && let Some(bound) = types.bound_of(name).cloned()
    {
        return validate_method_call(
            &bound,
            method,
            args,
            as_expression,
            span,
            symbols,
            types,
            signatures,
            current_class,
            context,
            options,
        );
    }
    let (class_name, bindings) = generic_bindings_for_type(object_type, types);
    if !matches!(
        object_type,
        TypeName::User(_) | TypeName::GenericInstance { .. }
    ) {
        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, options),
        )? {
            return Ok(res_ty);
        }

        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Method call requires a class instance",
            Some(span),
        ));
    }
    let Some(class_sig) = types.get_class(&class_name) else {
        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, options),
        )? {
            return Ok(res_ty);
        }

        return validate_structure_method_call(
            object_type,
            method,
            args,
            as_expression,
            span,
            symbols,
            types,
            signatures,
            current_class,
            context,
            options,
        );
    };

    if as_expression {
        if let Some(method_candidates) = class_sig.functions.get(&key(method)) {
            let validation = ExprValidation::new(symbols, types, signatures, context, options);
            let method_sig = resolve_overload(
                "Function",
                method,
                method_candidates,
                args,
                span,
                validation,
            )?;
            ensure_visible(
                method_sig.visibility,
                &class_sig.name,
                method,
                current_class,
                span,
            )?;
            validate_arguments("Function", method_sig, args, span, validation)?;
            return Ok(method_sig
                .return_type
                .clone()
                .expect("function return")
                .substitute_generics(&bindings));
        }
        if let Some(get) = class_sig
            .properties
            .get(&key(method))
            .map(|p| {
                resolve_accessor(
                    method,
                    &p.get,
                    args,
                    span,
                    validation_for(symbols, types, signatures, context, options),
                )
            })
            .transpose()?
            .flatten()
        {
            ensure_visible(get.visibility, &class_sig.name, method, current_class, span)?;
            let return_type = get
                .return_type
                .clone()
                .expect("property return type")
                .substitute_generics(&bindings);

            // Case 1: The property itself takes these arguments
            if get.params.len() == args.len() {
                // Try to validate arguments for the property Get
                let dummy_sig = CallableSig {
                    attributes: Vec::new(),
                    visibility: get.visibility,
                    name: method.to_string(),
                    type_params: Vec::new(),
                    generic_constraints: Vec::new(),
                    is_shared: false,
                    _is_iterator: get.is_iterator,
                    is_declare: false,
                    params: get.params.clone(),
                    return_type: Some(return_type.clone()),
                };
                if validate_arguments(
                    "Property",
                    &dummy_sig,
                    args,
                    span,
                    ExprValidation::new(symbols, types, signatures, context, options),
                )
                .is_ok()
                {
                    return Ok(return_type);
                }
            }

            // Case 2: The property returns an object that has a default property
            let default_call = match &return_type {
                TypeName::User(inner_class_name) => types
                    .get_class(inner_class_name.as_str())
                    .and_then(|c| c.default_property.as_ref())
                    .map(|name| (return_type.clone(), name.clone())),
                _ => None,
            };

            if let Some((inner_type, default_prop_name)) = default_call {
                return validate_method_call(
                    &inner_type,
                    &default_prop_name,
                    args,
                    true,
                    span,
                    symbols,
                    types,
                    signatures,
                    None,
                    context,
                    options,
                );
            }
        }
        if class_sig.subs.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Sub method '{}' cannot be used as an expression", method),
                Some(span),
            ));
        }
        if class_sig.events.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Event '{}' cannot be called directly", method),
                Some(span),
            ));
        }
        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, options),
        )? {
            return Ok(res_ty);
        }

        Err(unknown_class_member(class_sig, method, as_expression, span))
    } else {
        if let Some(method_candidates) = class_sig.subs.get(&key(method)) {
            let validation = ExprValidation::new(symbols, types, signatures, context, options);
            let method_sig =
                resolve_overload("Sub", method, method_candidates, args, span, validation)?;
            ensure_visible(
                method_sig.visibility,
                &class_sig.name,
                method,
                current_class,
                span,
            )?;
            validate_arguments("Sub", method_sig, args, span, validation)?;
            return Ok(TypeName::Variant);
        }
        // Sub-style property call (e.g., obj.Prop = value or obj.Prop(idx) = value)
        // This is complex because MemberCall is usually for reads.
        // But some VBA code might use MemberCall as a statement for something that returns an object and then calls a default sub?
        // Actually MemberSubCall is used for subs.

        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, options),
        )? {
            return Ok(res_ty);
        }

        if class_sig.functions.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!(
                    "Function method '{}' cannot be called as a statement",
                    method
                ),
                Some(span),
            ));
        }
        if class_sig.events.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Event '{}' cannot be called directly", method),
                Some(span),
            ));
        }
        Err(unknown_class_member(class_sig, method, as_expression, span))
    }
}

fn unknown_class_member(
    class_sig: &ClassSig,
    method: &str,
    as_expression: bool,
    span: Span,
) -> Diagnostic {
    let noun = if as_expression {
        "method or property"
    } else {
        "method"
    };
    let candidates = class_member_names(class_sig);
    Diagnostic::new(
        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
        format!(
            "Method or property '{}' was not found on type '{}'",
            method, class_sig.name
        ),
        Some(span),
    )
    .with_primary_label(format!("unknown {noun}"))
    .with_name_suggestion(method, candidates.iter().map(String::as_str))
    .with_available_items("available members", candidates.iter().map(String::as_str))
}

fn class_member_names(class_sig: &ClassSig) -> Vec<String> {
    let mut names = Vec::new();
    names.extend(
        class_sig
            .subs
            .values()
            .flatten()
            .map(|sig| sig.name.clone()),
    );
    names.extend(
        class_sig
            .functions
            .values()
            .flatten()
            .map(|sig| sig.name.clone()),
    );
    names.extend(class_sig.properties.values().map(|sig| sig.name.clone()));
    names.extend(class_sig.events.values().map(|sig| sig.name.clone()));
    names
}

#[allow(clippy::too_many_arguments)]
fn validate_structure_method_call(
    object_type: &TypeName,
    method: &str,
    args: &[Expr],
    as_expression: bool,
    span: crate::runtime::Span,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    current_type: Option<&str>,
    context: &Context<'_>,
    options: Options,
) -> Result<TypeName, Diagnostic> {
    let (type_name, bindings) = generic_bindings_for_type(object_type, types);
    if let Some(interface_sig) = types.get_interface(&type_name) {
        if as_expression {
            if let Some(method_candidates) = interface_sig.functions.get(&key(method)) {
                let validation = ExprValidation::new(symbols, types, signatures, context, options);
                let method_sig = resolve_overload(
                    "Function",
                    method,
                    method_candidates,
                    args,
                    span,
                    validation,
                )?;
                validate_arguments("Function", method_sig, args, span, validation)?;
                return Ok(method_sig
                    .return_type
                    .clone()
                    .expect("function return")
                    .substitute_generics(&bindings));
            }
            if let Some(get) = interface_sig
                .properties
                .get(&key(method))
                .and_then(|p| p.getter())
            {
                let return_type = get
                    .return_type
                    .clone()
                    .expect("property return type")
                    .substitute_generics(&bindings);
                let dummy_sig = CallableSig {
                    attributes: Vec::new(),
                    visibility: get.visibility,
                    name: method.to_string(),
                    type_params: Vec::new(),
                    generic_constraints: Vec::new(),
                    is_shared: false,
                    _is_iterator: get.is_iterator,
                    is_declare: false,
                    params: get.params.clone(),
                    return_type: Some(return_type.clone()),
                };
                validate_arguments(
                    "Property",
                    &dummy_sig,
                    args,
                    span,
                    ExprValidation::new(symbols, types, signatures, context, options),
                )?;
                return Ok(return_type);
            }
            if interface_sig.subs.contains_key(&key(method)) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("Sub method '{}' cannot be used as an expression", method),
                    Some(span),
                ));
            }
        } else {
            if let Some(method_candidates) = interface_sig.subs.get(&key(method)) {
                let validation = ExprValidation::new(symbols, types, signatures, context, options);
                let method_sig =
                    resolve_overload("Sub", method, method_candidates, args, span, validation)?;
                validate_arguments("Sub", method_sig, args, span, validation)?;
                return Ok(TypeName::Variant);
            }
            if let Some(property_accessor) = interface_sig
                .properties
                .get(&key(method))
                .and_then(|p| p.writer())
            {
                let dummy_sig = CallableSig {
                    attributes: Vec::new(),
                    visibility: property_accessor.visibility,
                    name: method.to_string(),
                    type_params: Vec::new(),
                    generic_constraints: Vec::new(),
                    is_shared: false,
                    _is_iterator: property_accessor.is_iterator,
                    is_declare: false,
                    params: property_accessor.params.clone(),
                    return_type: None,
                };
                validate_arguments(
                    "Property",
                    &dummy_sig,
                    args,
                    span,
                    ExprValidation::new(symbols, types, signatures, context, options),
                )?;
                return Ok(TypeName::Variant);
            }
            if interface_sig.functions.contains_key(&key(method)) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("Function method '{}' cannot be used as a Sub", method),
                    Some(span),
                ));
            }
        }
    }

    let type_sig = types.get(&type_name).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Type '{}' is not defined", type_name),
            Some(span),
        )
    })?;
    // `record.Items(0)` indexes an array field; it only looks like a method call.
    if let Some(field_sig) = type_sig.fields.get(&key(method))
        && field_sig.array.is_some()
    {
        for arg in args {
            let index_type = validate_expr(arg, symbols, types, signatures, context, options)?;
            ensure_assignable(&TypeName::Int64, &index_type, types, arg.span)?;
        }
        return Ok(field_sig.ty.substitute_generics(&bindings));
    }

    if !type_sig.is_structure {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Method call requires a class or Structure instance",
            Some(span),
        ));
    }
    if as_expression {
        if let Some(method_candidates) = type_sig.functions.get(&key(method)) {
            let validation = ExprValidation::new(symbols, types, signatures, context, options);
            let method_sig = resolve_overload(
                "Function",
                method,
                method_candidates,
                args,
                span,
                validation,
            )?;
            ensure_visible(
                method_sig.visibility,
                &type_sig.name,
                method,
                current_type,
                span,
            )?;
            validate_arguments("Function", method_sig, args, span, validation)?;
            return Ok(method_sig
                .return_type
                .clone()
                .expect("function return")
                .substitute_generics(&bindings));
        }
        if let Some(get) = type_sig
            .properties
            .get(&key(method))
            .and_then(|p| p.getter())
        {
            ensure_visible(get.visibility, &type_sig.name, method, current_type, span)?;
            let return_type = get
                .return_type
                .clone()
                .expect("property return type")
                .substitute_generics(&bindings);
            let dummy_sig = CallableSig {
                attributes: Vec::new(),
                visibility: get.visibility,
                name: method.to_string(),
                type_params: Vec::new(),
                generic_constraints: Vec::new(),
                is_shared: false,
                _is_iterator: get.is_iterator,
                is_declare: false,
                params: get.params.clone(),
                return_type: Some(return_type.clone()),
            };
            validate_arguments(
                "Property",
                &dummy_sig,
                args,
                span,
                ExprValidation::new(symbols, types, signatures, context, options),
            )?;
            return Ok(return_type);
        }
        if type_sig.subs.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Sub method '{}' cannot be used as an expression", method),
                Some(span),
            ));
        }
        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, options),
        )? {
            return Ok(res_ty);
        }

        Err(unknown_structure_member(
            type_sig,
            method,
            as_expression,
            span,
        ))
    } else {
        if method.eq_ignore_ascii_case("New")
            || method.eq_ignore_ascii_case("Constructor")
            || method.eq_ignore_ascii_case("Initialize")
        {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Structure constructor cannot be called as a normal method",
                Some(span),
            ));
        }
        if let Some(method_candidates) = type_sig.subs.get(&key(method)) {
            let validation = ExprValidation::new(symbols, types, signatures, context, options);
            let method_sig =
                resolve_overload("Sub", method, method_candidates, args, span, validation)?;
            ensure_visible(
                method_sig.visibility,
                &type_sig.name,
                method,
                current_type,
                span,
            )?;
            validate_arguments("Sub", method_sig, args, span, validation)?;
            return Ok(TypeName::Variant);
        }
        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, options),
        )? {
            return Ok(res_ty);
        }

        if type_sig.functions.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!(
                    "Function method '{}' cannot be called as a statement",
                    method
                ),
                Some(span),
            ));
        }
        Err(unknown_structure_member(
            type_sig,
            method,
            as_expression,
            span,
        ))
    }
}

fn unknown_structure_member(
    type_sig: &TypeSig,
    method: &str,
    as_expression: bool,
    span: Span,
) -> Diagnostic {
    let noun = if as_expression {
        "method or property"
    } else {
        "method"
    };
    let candidates = structure_member_names(type_sig);
    Diagnostic::new(
        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
        format!(
            "Method or property '{}' was not found on type '{}'",
            method, type_sig.name
        ),
        Some(span),
    )
    .with_primary_label(format!("unknown {noun}"))
    .with_name_suggestion(method, candidates.iter().map(String::as_str))
    .with_available_items("available members", candidates.iter().map(String::as_str))
}

fn structure_member_names(type_sig: &TypeSig) -> Vec<String> {
    let mut names = Vec::new();
    names.extend(type_sig.subs.values().flatten().map(|sig| sig.name.clone()));
    names.extend(
        type_sig
            .functions
            .values()
            .flatten()
            .map(|sig| sig.name.clone()),
    );
    names.extend(type_sig.properties.values().map(|sig| sig.name.clone()));
    names
}

fn member_access_class(object: &Expr, object_type: &TypeName) -> Option<String> {
    if matches!(object.kind, ExprKind::Me)
        && let TypeName::User(name) = object_type
    {
        return Some(name.clone());
    }
    None
}

pub(super) fn member_read_type(
    object_type: &TypeName,
    member: &str,
    types: &TypeRegistry,
    options: Options,
    span: crate::runtime::Span,
    current_class: Option<&str>,
) -> Result<TypeName, Diagnostic> {
    if let TypeName::Nullable(inner) = object_type {
        if member.eq_ignore_ascii_case("Value") {
            return Ok((**inner).clone());
        }
        if member.eq_ignore_ascii_case("HasValue") {
            return Ok(TypeName::Boolean);
        }
        // A nullable class reference still reaches its members. This is what
        // makes the rest of a `?.` chain resolve: the guarded prefix has already
        // been typed as nullable, and the chain continues against the class.
        if is_class_type(inner, types) {
            return member_read_type(inner, member, types, options, span, current_class);
        }
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!("Nullable type has no member '{}'", member),
            Some(span),
        ));
    }
    if object_type.same_type(&TypeName::Variant)
        || matches!(object_type, TypeName::User(name) if name.eq_ignore_ascii_case(well_known::OBJECT))
    {
        // Reaching a member of a value whose type is only known at run time is
        // late binding: nothing here can say whether the member exists. That is
        // how COM and `CallByName` work, and it is exactly what `Option Strict`
        // is for turning off.
        if options.strict {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                format!(
                    "Option Strict does not allow reaching '{}' on {}",
                    member,
                    object_type.display_name()
                ),
                Some(span),
            )
            .with_primary_label("whether this member exists is only known when the program runs")
            .with_help(
                "declare the value with the type it holds, or use CType to say what it is",
            ));
        }
        return Ok(TypeName::Variant);
    }
    if let TypeName::Tuple(elements) = object_type {
        return tuple_member_type(elements, member, span);
    }
    // A constrained type parameter has whatever its bound has. That is what
    // the constraint is for; without this, `Of T As IShape` says something the
    // body cannot then use.
    if let TypeName::User(name) = object_type
        && let Some(bound) = types.bound_of(name).cloned()
    {
        return member_read_type(&bound, member, types, options, span, current_class);
    }
    let (type_name, bindings) = generic_bindings_for_type(object_type, types);
    if !matches!(
        object_type,
        TypeName::User(_) | TypeName::GenericInstance { .. }
    ) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Member access requires a user-defined Type value",
            Some(span),
        ));
    }

    if let Some(type_sig) = types.get(&type_name) {
        if let Some(field) = type_sig.fields.get(&key(member)) {
            ensure_visible(
                field.visibility,
                &type_sig.name,
                member,
                current_class,
                span,
            )?;
            return Ok(field.ty.substitute_generics(&bindings));
        }
        let Some(property_sig) = type_sig.properties.get(&key(member)) else {
            let message = if type_sig.is_structure {
                format!(
                    "Type '{}' has no field or property '{}'",
                    type_sig.name, member
                )
            } else {
                format!("Type '{}' has no field '{}'", type_sig.name, member)
            };
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                message,
                Some(span),
            ));
        };
        let get = property_sig.getter().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Property '{}' has no Get accessor", property_sig.name),
                Some(span),
            )
        })?;
        ensure_visible(get.visibility, &type_sig.name, member, current_class, span)?;
        return Ok(get
            .return_type
            .clone()
            .expect("get return type")
            .substitute_generics(&bindings));
    }

    let class_sig = types.get_class(&type_name).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Type '{}' is not defined", type_name),
            Some(span),
        )
    })?;
    if let Some(field_sig) = class_sig.fields.get(&key(member)) {
        ensure_visible(
            field_sig.visibility,
            &class_sig.name,
            member,
            current_class,
            span,
        )?;
        return Ok(field_sig.ty.substitute_generics(&bindings));
    }

    let property_sig = class_sig.properties.get(&key(member)).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!(
                "Class '{}' has no field or property '{}'",
                class_sig.name, member
            ),
            Some(span),
        )
    })?;
    let get = property_sig.getter().ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!("Property '{}' has no Get accessor", property_sig.name),
            Some(span),
        )
    })?;
    ensure_visible(get.visibility, &class_sig.name, member, current_class, span)?;
    Ok(get
        .return_type
        .clone()
        .expect("get return type")
        .substitute_generics(&bindings))
}

/// Picks which `Let` or `Set` accessor a write goes through.
///
/// An indexed property is written `Item(2) = "two"`, so the indices come first
/// and the value last. That whole shape is what chooses between overloads:
/// `Item(2) = "two"` and `Item("k") = "v"` can reach different accessors.
fn pick_writer<'a>(
    candidates: &'a [PropertyAccessorSig],
    index_types: &[Option<TypeName>],
    value_type: &TypeName,
    member: &str,
    span: crate::runtime::Span,
) -> Result<Option<&'a PropertyAccessorSig>, Diagnostic> {
    match candidates {
        [] => return Ok(None),
        [only] => return Ok(Some(only)),
        _ => {}
    }

    let shapes: Vec<Vec<overloads::ParamShape>> = candidates
        .iter()
        .map(|accessor| {
            accessor
                .params
                .iter()
                .map(|param| overloads::ParamShape {
                    ty: param.ty.clone(),
                    is_optional: param.is_optional,
                    is_param_array: param.is_param_array,
                })
                .collect()
        })
        .collect();
    let mut supplied: Vec<Option<TypeName>> = index_types.to_vec();
    supplied.push(Some(value_type.clone()));

    match overloads::resolve(&shapes, &supplied) {
        overloads::Resolution::Single(chosen) => Ok(Some(&candidates[chosen])),
        overloads::Resolution::NoMatch => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            format!("No accessor of property '{member}' accepts a write of this shape"),
            Some(span),
        )),
        overloads::Resolution::Ambiguous(_) => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::AMBIGUOUS_OVERLOAD,
            format!("Which accessor of property '{member}' is written to is ambiguous"),
            Some(span),
        )),
    }
}

/// The parameter a write to a property lands in.
///
/// It is the last one. A plain property has only that; an indexed one takes
/// its indices first and the value after them, which is what
/// `Property Let Item(index As Long, value As String)` declares.
fn assigned_parameter(accessor: &PropertyAccessorSig) -> Result<&ParamSig, Diagnostic> {
    accessor.params.last().ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::INVALID_DECLARATION,
            "A property Let or Set has to declare the value it is given",
            None,
        )
    })
}

fn member_assignment_type(
    object_type: &TypeName,
    member: &str,
    value_type: &TypeName,
    index_types: &[Option<TypeName>],
    types: &TypeRegistry,
    span: crate::runtime::Span,
    current_class: Option<&str>,
) -> Result<TypeName, Diagnostic> {
    if object_type.same_type(&TypeName::Variant) {
        return Ok(value_type.clone());
    }
    let (type_name, bindings) = generic_bindings_for_type(object_type, types);
    if !matches!(
        object_type,
        TypeName::User(_) | TypeName::GenericInstance { .. }
    ) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Member assignment requires a user-defined Type value",
            Some(span),
        ));
    }

    if let Some(type_sig) = types.get(&type_name) {
        if let Some(field) = type_sig.fields.get(&key(member)) {
            ensure_visible(
                field.visibility,
                &type_sig.name,
                member,
                current_class,
                span,
            )?;
            return Ok(field.ty.substitute_generics(&bindings));
        }
        let property_sig = type_sig.properties.get(&key(member)).ok_or_else(|| {
            let message = if type_sig.is_structure {
                format!(
                    "Type '{}' has no field or property '{}'",
                    type_sig.name, member
                )
            } else {
                format!("Type '{}' has no field '{}'", type_sig.name, member)
            };
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                message,
                Some(span),
            )
        })?;
        let candidates =
            if is_class_type(value_type, types) || value_type.same_type(&TypeName::Variant) {
                property_sig.writers()
            } else {
                &property_sig.let_
            };
        let accessor =
            pick_writer(candidates, index_types, value_type, member, span)?.ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!(
                        "Property '{}' has no Let or Set accessor",
                        property_sig.name
                    ),
                    Some(span),
                )
            })?;
        ensure_visible(
            accessor.visibility,
            &type_sig.name,
            member,
            current_class,
            span,
        )?;
        return Ok(assigned_parameter(accessor)?
            .ty
            .substitute_generics(&bindings));
    }

    let class_sig = types.get_class(&type_name).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Type '{}' is not defined", type_name),
            Some(span),
        )
    })?;
    if let Some(field_sig) = class_sig.fields.get(&key(member)) {
        ensure_visible(
            field_sig.visibility,
            &class_sig.name,
            member,
            current_class,
            span,
        )?;
        return Ok(field_sig.ty.substitute_generics(&bindings));
    }

    let property_sig = class_sig.properties.get(&key(member)).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!(
                "Class '{}' has no field or property '{}'",
                class_sig.name, member
            ),
            Some(span),
        )
    })?;
    let candidates = if is_class_type(value_type, types) || value_type.same_type(&TypeName::Variant)
    {
        property_sig.writers()
    } else {
        &property_sig.let_
    };
    let accessor =
        pick_writer(candidates, index_types, value_type, member, span)?.ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
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
    Ok(assigned_parameter(accessor)?
        .ty
        .substitute_generics(&bindings))
}

fn ensure_visible(
    visibility: Visibility,
    owner_class: &str,
    member: &str,
    current_class: Option<&str>,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if visibility == Visibility::Public
        || visibility == Visibility::Friend
        || visibility == Visibility::ProtectedFriend
        || current_class.is_some_and(|class_name| class_name.eq_ignore_ascii_case(owner_class))
    {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_IS_PRIVATE,
            format!(
                "Member '{}' is {} in Class '{}'",
                member,
                if visibility == Visibility::Protected {
                    "Protected"
                } else {
                    "Private"
                },
                owner_class
            ),
            Some(span),
        )
        .with_primary_label("member is not accessible here")
        .with_help("access this member from an allowed class scope or make it Public"))
    }
}

/// The type of `tuple.Member`.
///
/// Every element answers to its position (`Item1`, `Item2`), and a named one
/// also answers to its name.
fn tuple_member_type(
    elements: &[crate::runtime::TupleElement],
    member: &str,
    span: crate::runtime::Span,
) -> Result<TypeName, Diagnostic> {
    for (index, element) in elements.iter().enumerate() {
        let positional = crate::runtime::TupleElement::positional_name(index);
        let answers_to = positional.eq_ignore_ascii_case(member)
            || element
                .name
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case(member));
        if answers_to {
            return Ok(element.ty.clone());
        }
    }

    let names: Vec<String> = elements
        .iter()
        .enumerate()
        .map(|(index, element)| match &element.name {
            Some(name) => name.clone(),
            None => crate::runtime::TupleElement::positional_name(index),
        })
        .collect();
    Err(Diagnostic::new(
        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
        format!("A tuple has no element '{member}'"),
        Some(span),
    )
    .with_available_items("its elements are", names.iter().map(String::as_str)))
}

/// Picks which accessor of a property a use site means.
///
/// Reading `Item(1)` and `Item("a")` can reach different getters, the way two
/// procedures sharing a name can. With one accessor there is nothing to choose,
/// and it is returned untouched so the argument checking that follows reports
/// what is actually wrong.
pub(super) fn resolve_accessor<'a>(
    name: &str,
    candidates: &'a [PropertyAccessorSig],
    args: &[Expr],
    span: crate::runtime::Span,
    validation: ExprValidation<'_, '_>,
) -> Result<Option<&'a PropertyAccessorSig>, Diagnostic> {
    match candidates {
        [] => return Ok(None),
        [only] => return Ok(Some(only)),
        _ => {}
    }

    let shapes: Vec<Vec<overloads::ParamShape>> = candidates
        .iter()
        .map(|accessor| {
            accessor
                .params
                .iter()
                .map(|param| overloads::ParamShape {
                    ty: param.ty.clone(),
                    is_optional: param.is_optional,
                    is_param_array: param.is_param_array,
                })
                .collect()
        })
        .collect();
    let argument_types: Vec<Option<TypeName>> = args
        .iter()
        .map(|arg| {
            validate_expr(
                arg,
                validation.symbols,
                validation.types,
                validation.signatures,
                validation.context,
                validation.options,
            )
            .ok()
        })
        .collect();

    match overloads::resolve(&shapes, &argument_types) {
        overloads::Resolution::Single(chosen) => Ok(Some(&candidates[chosen])),
        overloads::Resolution::NoMatch => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            format!("No accessor of property '{name}' accepts these arguments"),
            Some(span),
        )),
        overloads::Resolution::Ambiguous(_) => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::AMBIGUOUS_OVERLOAD,
            format!("Which accessor of property '{name}' is meant is ambiguous"),
            Some(span),
        )),
    }
}

/// Bundles the pieces validation carries, where the call site has them loose.
fn validation_for<'a, 'ctx>(
    symbols: &'a HashMap<String, VarType>,
    types: &'a TypeRegistry,
    signatures: &'a Signatures,
    context: &'a Context<'ctx>,
    options: Options,
) -> ExprValidation<'a, 'ctx> {
    ExprValidation::new(symbols, types, signatures, context, options)
}

pub(super) fn ensure_known_type(
    ty: &TypeName,
    types: &TypeRegistry,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    match ty {
        TypeName::String
        | TypeName::Byte
        | TypeName::Integer
        | TypeName::Long
        | TypeName::Int64
        | TypeName::UInt32
        | TypeName::UInt64
        | TypeName::Single
        | TypeName::Double
        | TypeName::Currency
        | TypeName::Decimal
        | TypeName::Boolean
        | TypeName::Date
        | TypeName::Variant
        | TypeName::Ptr
        | TypeName::FuncPtr => Ok(()),
        TypeName::Tuple(elements) => {
            for element in elements {
                ensure_known_type(&element.ty, types, span)?;
            }
            Ok(())
        }
        TypeName::User(name) => {
            if types.generic_params.contains(&key(name))
                || name.eq_ignore_ascii_case(well_known::OBJECT)
                || name.eq_ignore_ascii_case(well_known::COLLECTION)
                || name.contains('.')
                || is_builtin_vba_enum_type(name)
            {
                Ok(())
            } else if let Some(sig) = types.get(name)
                && !sig.type_params.is_empty()
            {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Generic type '{}' requires {} type argument(s)",
                        sig.name,
                        sig.type_params.len()
                    ),
                    Some(span),
                ))
            } else if let Some(sig) = types.get_class(name)
                && !sig.type_params.is_empty()
            {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Generic type '{}' requires {} type argument(s)",
                        sig.name,
                        sig.type_params.len()
                    ),
                    Some(span),
                ))
            } else if let Some(sig) = types.get_interface(name)
                && !sig.type_params.is_empty()
            {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Generic type '{}' requires {} type argument(s)",
                        sig.name,
                        sig.type_params.len()
                    ),
                    Some(span),
                ))
            } else if types.contains(name) {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Type '{}' is not defined", name),
                    Some(span),
                ))
            }
        }
        TypeName::GenericInstance { name, args } => {
            let expected = types
                .get(name)
                .map(|sig| (&sig.name, &sig.type_params, &sig.generic_constraints))
                .or_else(|| {
                    types
                        .get_class(name)
                        .map(|sig| (&sig.name, &sig.type_params, &sig.generic_constraints))
                })
                .or_else(|| {
                    types
                        .get_interface(name)
                        .map(|sig| (&sig.name, &sig.type_params, &sig.generic_constraints))
                });
            let Some((canonical, type_params, generic_constraints)) = expected else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Type '{}' is not defined", name),
                    Some(span),
                ));
            };
            let expected_count = type_params.len();
            if expected_count != args.len() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Type parameter count mismatch for {}. Expected {}, received {}",
                        canonical,
                        expected_count,
                        args.len()
                    ),
                    Some(span),
                )
                .with_help(format!("received {}", ty.display_name())));
            }
            for arg in args {
                ensure_known_type(arg, types, span)?;
            }
            validate_generic_constraints(
                canonical,
                type_params,
                generic_constraints,
                args,
                types,
                span,
            )?;
            Ok(())
        }
        TypeName::Enum(name) => {
            if types.enums.contains_key(&key(name)) {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Enum '{}' is not defined", name),
                    Some(span),
                ))
            }
        }
        TypeName::Array(inner) => ensure_known_type(inner, types, span),
        TypeName::Nullable(inner) => ensure_known_type(inner, types, span),
    }
}

fn is_builtin_vba_enum_type(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "vbvartype"
            | "vbcomparemethod"
            | "vbcalltype"
            | "vbmsgboxstyle"
            | "vbmsgboxresult"
            | "vbfileattribute"
            | "vbdatetimeformat"
            | "vbdayofweek"
            | "vbfirstweekofyear"
            | "vbstrconv"
            | "vbtristate"
            | "vbappwinstyle"
    )
}

fn validate_generic_constraints(
    owner: &str,
    type_params: &[String],
    constraints: &[crate::GenericParamConstraint],
    type_args: &[TypeName],
    types: &TypeRegistry,
    span: Span,
) -> Result<(), Diagnostic> {
    for constraint in constraints {
        let Some(index) = type_params
            .iter()
            .position(|param| param.eq_ignore_ascii_case(&constraint.name))
        else {
            continue;
        };
        let arg = &type_args[index];
        if constraint.require_class && !is_reference_type(arg, types) {
            return Err(generic_constraint_error(
                owner,
                &constraint.name,
                arg,
                "must be a reference type",
                span,
            ));
        }
        if constraint.require_structure && !is_value_type(arg, types) {
            return Err(generic_constraint_error(
                owner,
                &constraint.name,
                arg,
                "must be a value type",
                span,
            ));
        }
        if constraint.require_new && !has_parameterless_constructor(arg, types) {
            return Err(generic_constraint_error(
                owner,
                &constraint.name,
                arg,
                "must have a public parameterless constructor",
                span,
            ));
        }
        for bound in &constraint.bounds {
            if !satisfies_type_bound(arg, bound, types) {
                return Err(generic_constraint_error(
                    owner,
                    &constraint.name,
                    arg,
                    &format!("must inherit from or implement '{}'", bound.display_name()),
                    span,
                ));
            }
        }
    }
    Ok(())
}

fn generic_constraint_error(
    owner: &str,
    param: &str,
    arg: &TypeName,
    requirement: &str,
    span: Span,
) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
        format!(
            "Type argument '{}' for '{}.{}' {}",
            arg.display_name(),
            owner,
            param,
            requirement
        ),
        Some(span),
    )
}

fn is_reference_type(ty: &TypeName, types: &TypeRegistry) -> bool {
    match ty {
        TypeName::String | TypeName::Array(_) => true,
        TypeName::User(name) => {
            name.eq_ignore_ascii_case(well_known::OBJECT)
                || name.eq_ignore_ascii_case(well_known::COLLECTION)
                || types.get_class(name).is_some()
                || types.get_interface(name).is_some()
        }
        TypeName::GenericInstance { name, .. } => {
            types.get_class(name).is_some() || types.get_interface(name).is_some()
        }
        _ => false,
    }
}

fn is_value_type(ty: &TypeName, types: &TypeRegistry) -> bool {
    match ty {
        TypeName::Byte
        | TypeName::Integer
        | TypeName::Long
        | TypeName::Int64
        | TypeName::UInt32
        | TypeName::UInt64
        | TypeName::Single
        | TypeName::Double
        | TypeName::Currency
        | TypeName::Decimal
        | TypeName::Boolean
        | TypeName::Date
        | TypeName::Ptr
        | TypeName::FuncPtr
        | TypeName::Enum(_) => true,
        TypeName::User(name) => {
            types.get(name).is_some_and(|sig| sig.is_structure) || types.get_enum(name).is_some()
        }
        TypeName::GenericInstance { name, .. } => {
            types.get(name).is_some_and(|sig| sig.is_structure)
        }
        TypeName::String | TypeName::Variant | TypeName::Array(_) => false,
        // A tuple is copied when it is assigned, the way a Structure is.
        TypeName::Tuple(_) => true,
        TypeName::Nullable(inner) => is_value_type(inner, types),
    }
}

fn has_parameterless_constructor(ty: &TypeName, types: &TypeRegistry) -> bool {
    if is_value_type(ty, types) {
        return true;
    }
    match ty {
        TypeName::User(name) if name.eq_ignore_ascii_case(well_known::OBJECT) => true,
        TypeName::User(name) => types
            .get_class(name)
            .is_some_and(class_has_public_default_new),
        TypeName::GenericInstance { name, .. } => types
            .get_class(name)
            .is_some_and(class_has_public_default_new),
        _ => false,
    }
}

fn class_has_public_default_new(class: &ClassSig) -> bool {
    if class.inheritance == crate::ClassInheritance::MustInherit {
        return false;
    }

    // An explicit constructor (Sub New or Class_Initialize) replaces the
    // default one. `New Thing()` still works if any overload of it is public
    // and takes nothing.
    match well_known::find_constructor(&class.subs) {
        Some(overloads) => overloads
            .iter()
            .any(|init| init.visibility == Visibility::Public && init.params.is_empty()),
        None => true,
    }
}

fn satisfies_type_bound(arg: &TypeName, bound: &TypeName, types: &TypeRegistry) -> bool {
    if arg.same_type(bound) {
        return true;
    }
    match (arg, bound) {
        (_, TypeName::User(name)) if name.eq_ignore_ascii_case(well_known::OBJECT) => {
            is_reference_type(arg, types)
        }
        (TypeName::User(arg_name), TypeName::User(bound_name))
        | (TypeName::GenericInstance { name: arg_name, .. }, TypeName::User(bound_name)) => {
            // A bound is satisfied by inheriting from it or by implementing it.
            // The diagnostic has always said both; only the first was checked.
            class_inherits_from(arg_name, bound_name, types)
                || class_implements_interface(arg_name, bound, types)
        }
        _ => false,
    }
}

fn class_inherits_from(class_name: &str, bound_name: &str, types: &TypeRegistry) -> bool {
    let Some(class) = types.get_class(class_name) else {
        return false;
    };
    let Some(base) = &class.base_class else {
        return false;
    };
    let (TypeName::User(base_name)
    | TypeName::GenericInstance {
        name: base_name, ..
    }) = base
    else {
        return false;
    };
    base_name.eq_ignore_ascii_case(bound_name) || class_inherits_from(base_name, bound_name, types)
}

fn generic_bindings_for_type(
    ty: &TypeName,
    types: &TypeRegistry,
) -> (String, Vec<(String, TypeName)>) {
    match ty {
        TypeName::GenericInstance { name, args } => {
            let params = types
                .get(name)
                .map(|sig| sig.type_params.clone())
                .or_else(|| types.get_class(name).map(|sig| sig.type_params.clone()))
                .or_else(|| types.get_interface(name).map(|sig| sig.type_params.clone()))
                .unwrap_or_default();
            (
                name.clone(),
                params.into_iter().zip(args.iter().cloned()).collect(),
            )
        }
        TypeName::User(name) => (name.clone(), Vec::new()),
        _ => (ty.display_name(), Vec::new()),
    }
}

/// Rejects, under `Option Strict`, a conversion that can lose something.
///
/// Off, which is the default and what VBA source expects, these all convert
/// silently: `Dim n As Integer = 3.9` rounds, and `Dim n As Long = "7"` parses.
/// On, each has to be asked for with `CInt`, `CLng`, and the rest, so that
/// where a value changes shape is written down.
///
/// Widening is untouched either way: nothing is lost putting an `Integer` in a
/// `Long`, so nothing is gained by making it explicit.
fn ensure_strict_conversion(
    target: &TypeName,
    source: &TypeName,
    options: Options,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if !options.strict {
        return Ok(());
    }

    let reason = if source.same_type(&TypeName::Variant) && !target.same_type(&TypeName::Variant) {
        "a Variant holds anything, so what it holds is only known when the program runs"
    } else if is_numeric_type(target) && source.same_type(&TypeName::String) {
        "a String has to be parsed to be a number, and it may not be one"
    } else if target.same_type(&TypeName::String) && is_numeric_type(source) {
        "a number has to be formatted to be a String"
    } else if is_numeric_type(target) && is_numeric_type(source) && narrows(source, target) {
        "the value does not fit, and would be rounded or overflow"
    } else {
        return Ok(());
    };

    Err(Diagnostic::new(
        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
        format!(
            "Option Strict does not allow {} to become {}",
            source.display_name(),
            target.display_name()
        ),
        Some(span),
    )
    .with_primary_label(reason)
    .with_help(match conversion_function(target) {
        Some(name) => format!("convert it explicitly, with {name} or CType"),
        None => "convert it explicitly, with CType".to_string(),
    }))
}

/// The conversion that asks for this type by name.
fn conversion_function(ty: &TypeName) -> Option<&'static str> {
    Some(match ty {
        TypeName::Byte => "CByte",
        TypeName::Integer => "CInt",
        TypeName::Long => "CLng",
        TypeName::Int64 => "CLngLng",
        TypeName::Single => "CSng",
        TypeName::Double => "CDbl",
        TypeName::Currency => "CCur",
        TypeName::Decimal => "CDec",
        TypeName::Boolean => "CBool",
        TypeName::Date => "CDate",
        TypeName::String => "CStr",
        _ => return None,
    })
}

/// Whether reaching `to` from `from` can lose something.
fn narrows(from: &TypeName, to: &TypeName) -> bool {
    fn rank(ty: &TypeName) -> Option<u8> {
        Some(match ty {
            TypeName::Byte => 0,
            TypeName::Integer => 1,
            TypeName::Long => 2,
            TypeName::Int64 => 3,
            TypeName::Decimal => 4,
            TypeName::Single => 5,
            TypeName::Double => 6,
            _ => return None,
        })
    }
    match (rank(from), rank(to)) {
        (Some(from), Some(to)) => from > to,
        // Currency, Date, and the unsigned widths sit off the ladder; a
        // conversion between one of those and anything else is not obviously
        // lossless, so it is spelled out.
        _ => !from.same_type(to),
    }
}

pub(super) fn ensure_assignable_expr(
    target: &TypeName,
    source: &TypeName,
    source_expr: &Expr,
    types: &TypeRegistry,
    options: Options,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    ensure_strict_conversion(target, source, options, span)?;
    if matches!(source_expr.kind, ExprKind::Nothing) {
        if target.same_type(&TypeName::Variant) || matches!(target, TypeName::Nullable(_)) {
            return Ok(());
        }
        return ensure_class_type(target, types, span, "Nothing requires a class object type");
    }
    if is_enum_type(target, types) && is_numeric_type(source) {
        return Ok(());
    }

    if let TypeName::User(class_name) = &source
        && let Some(class_sig) = types.get_class(class_name)
        && let Some(default_prop_name) = &class_sig.default_property
        && let Some(prop_sig) = class_sig.properties.get(&key(default_prop_name))
        && let Some(get) = prop_sig.get.first()
        && get.params.is_empty()
        && let Some(prop_type) = &get.return_type
        && ensure_assignable(target, prop_type, types, span).is_ok()
    {
        return Ok(());
    }

    if let (TypeName::User(src_name), _) = (source, target)
        && (class_inherits_from(src_name, &target.display_name(), types)
            || class_implements_interface(src_name, target, types))
    {
        return Ok(());
    }

    if let (TypeName::GenericInstance { name: src_name, .. }, _) = (source, target)
        && (class_inherits_from(src_name, &target.display_name(), types)
            || class_implements_interface(src_name, target, types))
    {
        return Ok(());
    }

    ensure_assignable(target, source, types, span)
}

fn class_implements_interface(
    class_name: &str,
    interface_ty: &TypeName,
    types: &TypeRegistry,
) -> bool {
    let Some(class) = types.get_class(class_name) else {
        return false;
    };
    for impl_ty in &class.implements {
        if impl_ty.same_type(interface_ty) {
            return true;
        }
    }
    if let Some(base) = &class.base_class {
        let base_name = match base {
            TypeName::User(name) => name,
            TypeName::GenericInstance { name, .. } => name,
            _ => return false,
        };
        if class_implements_interface(base_name, interface_ty, types) {
            return true;
        }
    }
    false
}

pub(super) fn ensure_class_type(
    ty: &TypeName,
    types: &TypeRegistry,
    span: crate::runtime::Span,
    message: &str,
) -> Result<(), Diagnostic> {
    if is_class_type(ty, types) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            message,
            Some(span),
        ))
    }
}

pub(super) fn is_object_reference_expr(expr: &Expr, ty: &TypeName, types: &TypeRegistry) -> bool {
    matches!(expr.kind, ExprKind::Nothing)
        || is_class_type(ty, types)
        || ty.same_type(&TypeName::Variant)
        // A nullable holds Nothing when it has no value, so `value Is Nothing`
        // is the idiomatic emptiness test for `T?`.
        || matches!(ty, TypeName::Nullable(_))
}

pub(super) fn is_class_type(ty: &TypeName, types: &TypeRegistry) -> bool {
    match ty {
        TypeName::User(name) => {
            types.get_class(name).is_some()
                || types.get_interface(name).is_some()
                || name.eq_ignore_ascii_case(well_known::OBJECT)
                || name.eq_ignore_ascii_case(well_known::COLLECTION)
        }
        TypeName::GenericInstance { name, .. } => {
            types.get_class(name).is_some() || types.get_interface(name).is_some()
        }
        _ => false,
    }
}

fn find_overloaded_binary_operator(
    left: &TypeName,
    op: crate::OperatorKind,
    right: &TypeName,
    types: &TypeRegistry,
) -> Option<TypeName> {
    // Try left operand
    if let TypeName::User(name) = left {
        if let Some(class) = types.get_class(name)
            && let Some(operator) = class.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
        if let Some(type_sig) = types.get(name)
            && let Some(operator) = type_sig.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
    }

    // Try right operand
    if let TypeName::User(name) = right {
        if let Some(class) = types.get_class(name)
            && let Some(operator) = class.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
        if let Some(type_sig) = types.get(name)
            && let Some(operator) = type_sig.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
    }

    None
}

fn find_overloaded_unary_operator(
    op: crate::OperatorKind,
    ty: &TypeName,
    types: &TypeRegistry,
) -> Option<TypeName> {
    if let TypeName::User(name) = ty {
        if let Some(class) = types.get_class(name)
            && let Some(operator) = class.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
        if let Some(type_sig) = types.get(name)
            && let Some(operator) = type_sig.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
    }
    None
}

pub(super) fn is_enum_type(ty: &TypeName, types: &TypeRegistry) -> bool {
    matches!(ty, TypeName::User(name) if types.get_enum(name).is_some())
}

pub(super) fn ensure_case_comparable(
    subject: &TypeName,
    value: &TypeName,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if subject.same_type(&TypeName::Variant)
        || value.same_type(&TypeName::Variant)
        || subject.same_type(value)
        || (is_numeric_type(subject) && is_numeric_type(value))
        || (matches!(subject, TypeName::User(_)) && is_numeric_type(value))
        || (is_numeric_type(subject) && matches!(value, TypeName::User(_)))
    {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::SELECT_CASE,
            "Case expression type must match Select Case expression type",
            Some(span),
        )
        .with_primary_label("case expression has an incompatible type"))
    }
}

pub(super) fn validate_case_item(
    item: &CaseItem,
    subject_type: &TypeName,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
    options: Options,
) -> Result<(), Diagnostic> {
    match item {
        CaseItem::Value(value) => {
            let value_type = validate_expr(value, symbols, types, signatures, context, options)?;
            ensure_case_comparable(subject_type, &value_type, value.span)
        }
        CaseItem::Range { start, end } => {
            let start_type = validate_expr(start, symbols, types, signatures, context, options)?;
            let end_type = validate_expr(end, symbols, types, signatures, context, options)?;
            ensure_case_comparable(subject_type, &start_type, start.span)?;
            ensure_case_comparable(subject_type, &end_type, end.span)?;
            ensure_case_orderable(subject_type, start.span)
        }
        CaseItem::Compare { op, expr } => {
            let expr_type = validate_expr(expr, symbols, types, signatures, context, options)?;
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
    if is_numeric_type(ty) || ty.same_type(&TypeName::String) || ty.same_type(&TypeName::Variant) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::SELECT_CASE,
            "Case range or comparison requires numeric or String operands",
            Some(span),
        )
        .with_primary_label("range or comparison is not orderable"))
    }
}

/// Checks that `Continue For`, `Continue While`, and `Continue Do` each appear
/// inside a loop of the matching kind.
pub(super) fn validate_continue(
    target: ContinueTarget,
    span: crate::runtime::Span,
    loop_context: LoopContext,
) -> Result<(), Diagnostic> {
    let inside = match target {
        ContinueTarget::For => loop_context.for_depth > 0,
        ContinueTarget::While => loop_context.while_depth > 0,
        ContinueTarget::Do => loop_context.do_depth > 0,
    };
    if inside {
        return Ok(());
    }
    let keyword = target.keyword();
    Err(Diagnostic::new(
        crate::runtime::DiagnosticCode::CONTROL_FLOW,
        format!("Continue {keyword} is only valid inside {keyword}"),
        Some(span),
    )
    .with_primary_label(format!("invalid Continue {keyword}")))
}

pub(super) fn validate_exit(
    target: ExitTarget,
    span: crate::runtime::Span,
    context: &Context<'_>,
    loop_context: LoopContext,
) -> Result<(), Diagnostic> {
    match target {
        ExitTarget::Sub => match context {
            Context::Sub { .. } | Context::MethodSub { .. } => Ok(()),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Sub is only valid inside Sub",
                Some(span),
            )
            .with_primary_label("invalid Exit Sub")
            .with_help("use Exit Sub only inside a Sub body")),
        },
        ExitTarget::Function => match context {
            Context::Function { .. } | Context::MethodFunction { .. } => Ok(()),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Function is only valid inside Function",
                Some(span),
            )
            .with_primary_label("invalid Exit Function")
            .with_help("use Exit Function only inside a Function body")),
        },
        ExitTarget::Property => match context {
            Context::PropertyGet { .. } | Context::PropertyLetSet { .. } => Ok(()),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Property is only valid inside Property",
                Some(span),
            )
            .with_primary_label("invalid Exit Property")
            .with_help("use Exit Property only inside a Property Get, Let, or Set body")),
        },
        ExitTarget::For => {
            if loop_context.for_depth > 0 {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit For is only valid inside For",
                    Some(span),
                )
                .with_primary_label("invalid Exit For"))
            }
        }
        ExitTarget::While => {
            if loop_context.while_depth > 0 {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit While is only valid inside While",
                    Some(span),
                )
                .with_primary_label("invalid Exit While"))
            }
        }
        ExitTarget::Do => {
            if loop_context.do_depth > 0 {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit Do is only valid inside Do",
                    Some(span),
                )
                .with_primary_label("invalid Exit Do"))
            }
        }
    }
}

pub(super) fn is_numeric_type(ty: &TypeName) -> bool {
    match ty {
        TypeName::Byte
        | TypeName::Integer
        | TypeName::Long
        | TypeName::Int64
        | TypeName::UInt32
        | TypeName::UInt64
        | TypeName::Single
        | TypeName::Double
        | TypeName::Currency
        | TypeName::Decimal
        | TypeName::Date => true,
        TypeName::Nullable(inner) => is_numeric_type(inner),
        _ => false,
    }
}

/// Whether one tuple fits another, element by element.
///
/// Elements convert the way anything else does, so `(1, 2)` fits
/// `(Long, Double)`. Names do not take part: they are how an element is read
/// back, not what makes one tuple type different from another.
fn fits_tuple(
    target: &TypeName,
    source: &TypeName,
    types: &TypeRegistry,
    span: crate::runtime::Span,
) -> bool {
    let (TypeName::Tuple(target), TypeName::Tuple(source)) = (target, source) else {
        return false;
    };
    target.len() == source.len()
        && target
            .iter()
            .zip(source)
            .all(|(target, source)| ensure_assignable(&target.ty, &source.ty, types, span).is_ok())
}

/// Whether a callable value fits a delegate-typed place.
///
/// A delegate names a shape, and what actually flows into one is a lambda or
/// the address of a procedure. Neither carries the delegate's name, so the fit
/// is decided by what the value is, not by what it is called. Checking that the
/// shape *matches* happens where the value is still an expression, in
/// [`ensure_delegate_shape`]; by the time a type name is all that is left, the
/// parameters are gone.
fn fits_delegate(target: &TypeName, source: &TypeName, types: &TypeRegistry) -> bool {
    let TypeName::User(name) = target else {
        return false;
    };
    types.delegates.contains_key(&key(name))
        && matches!(source, TypeName::FuncPtr | TypeName::Variant)
}

pub(super) fn ensure_assignable(
    target: &TypeName,
    source: &TypeName,
    types: &TypeRegistry,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if fits_delegate(target, source, types)
        || fits_tuple(target, source, types, span)
        || target.same_type(&TypeName::Variant)
        || source.same_type(&TypeName::Variant)
        || target.same_type(source)
        || (is_numeric_type(target) && is_numeric_type(source))
        || (matches!(target, TypeName::Ptr | TypeName::FuncPtr) && is_numeric_type(source))
        || (is_numeric_type(target) && matches!(source, TypeName::Ptr | TypeName::FuncPtr))
        || (matches!(target, TypeName::Ptr | TypeName::FuncPtr)
            && matches!(source, TypeName::Ptr | TypeName::FuncPtr))
        || matches!(target, TypeName::User(name) if name.rsplit('.').next().is_some_and(|name| name.eq_ignore_ascii_case(well_known::OBJECT)))
            && matches!(source, TypeName::User(_))
        || is_inherited_class_assignable(target, source)
        || (matches!(target, TypeName::Nullable(_))
            && (matches!(source, TypeName::Variant) || matches!(source, TypeName::User(_))))
        || (matches!(source, TypeName::Nullable(_)) && matches!(target, TypeName::Variant))
        || (if let (TypeName::Nullable(t_inner), TypeName::Nullable(s_inner)) = (target, source) {
            ensure_assignable(t_inner, s_inner, types, span).is_ok()
        } else {
            false
        })
        || (if let TypeName::Nullable(inner) = target {
            ensure_assignable(inner, source, types, span).is_ok()
        } else {
            false
        })
    {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!(
                "Cannot assign {} value to {} variable",
                source.display_name(),
                target.display_name()
            ),
            Some(span),
        )
        .with_primary_label(format!(
            "expected {}, found {}",
            target.display_name(),
            source.display_name()
        ))
        .with_help("change the variable type or assign a value with the expected type"))
    }
}

fn is_inherited_class_assignable(target: &TypeName, source: &TypeName) -> bool {
    matches!((target, source), (TypeName::User(_), TypeName::User(_)))
}

fn validate_err_raise_args(
    args: &[Expr],
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    span: crate::runtime::Span,
    context: &Context<'_>,
    options: Options,
) -> Result<(), Diagnostic> {
    if args.is_empty() || args.len() > 5 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            "Err.Raise expects 1 to 5 arguments",
            Some(span),
        ));
    }
    let expected = [
        TypeName::Integer,
        TypeName::String,
        TypeName::String,
        TypeName::String,
        TypeName::Integer,
    ];
    for (index, arg) in args.iter().enumerate() {
        let actual = validate_expr(arg, symbols, types, signatures, context, options)?;
        ensure_assignable(&expected[index], &actual, types, arg.span)?;
    }
    Ok(())
}

fn resolve_extension_method(
    object_type: &TypeName,
    method: &str,
    args: &[Expr],
    as_expression: bool,
    span: crate::runtime::Span,
    validation: ExprValidation<'_, '_>,
) -> Result<Option<TypeName>, Diagnostic> {
    let type_key = object_type.display_name().to_lowercase();
    if let Some(methods) = validation.signatures.extension_methods.get(&type_key) {
        for sig in methods {
            if sig.name.eq_ignore_ascii_case(method) {
                if sig.params.is_empty() {
                    continue;
                }

                let mut shifted_sig = sig.clone();
                shifted_sig.params.remove(0);

                validate_arguments(
                    if as_expression { "Function" } else { "Sub" },
                    &shifted_sig,
                    args,
                    span,
                    validation,
                )?;

                if as_expression {
                    return Ok(Some(sig.return_type.clone().unwrap_or(TypeName::Variant)));
                } else {
                    return Ok(Some(TypeName::Variant));
                }
            }
        }
    }
    Ok(None)
}
