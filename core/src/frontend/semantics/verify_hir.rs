//! Structural preconditions for ownership analysis and eventual MIR lowering.
use super::typed_hir::{
    ArgumentMode, Conversion, DoCondition, Expression, ExpressionKind, LocalId, LoopId, ScopeId,
    Statement, TypedBody, ValueCategory,
};
use crate::frontend::type_model::TypeName;
use crate::runtime::{Diagnostic, DiagnosticCode, Span};
use std::collections::HashSet;

pub fn verify_body(body: &TypedBody) -> Result<(), Diagnostic> {
    for (index, structure) in body.structures.iter().enumerate() {
        if !matches!(structure, TypeName::User(_))
            || body.structures[..index]
                .iter()
                .any(|prior| prior.same_type(structure))
        {
            return Err(invalid(
                "HIR Structure identity is invalid or duplicated",
                body.span,
            ));
        }
    }
    for field in &body.fields {
        if !body
            .structures
            .iter()
            .chain(&body.classes)
            .any(|owner| owner.same_type(&field.owner))
        {
            return Err(invalid(
                "HIR field owner is not a declared value or reference type",
                body.span,
            ));
        }
    }
    if body.root_scope != ScopeId(0)
        || body
            .scopes
            .first()
            .is_none_or(|scope| scope.parent.is_some())
    {
        return Err(invalid("HIR has no root scope", body.span));
    }
    for (index, scope) in body.scopes.iter().enumerate() {
        if scope.id != ScopeId(index) {
            return Err(invalid("Scope ID does not match its index", scope.span));
        }
        for local in &scope.locals {
            if body
                .locals
                .get(local.0)
                .is_none_or(|item| item.id != *local || item.scope != scope.id)
            {
                return Err(invalid(
                    "Local is registered in the wrong scope",
                    scope.span,
                ));
            }
        }
        for cleanup in &scope.cleanup {
            match cleanup {
                super::typed_hir::ScopeCleanup::ExplicitDispose { resource, method } => {
                    let Some(local) = body.locals.get(resource.0) else {
                        return Err(invalid("Using cleanup has an invalid resource", scope.span));
                    };
                    let Some(disposer) = body.disposers.get(method.0) else {
                        return Err(invalid(
                            "Using cleanup has an invalid Dispose method",
                            scope.span,
                        ));
                    };
                    if local.scope != scope.id
                        || local.storage != super::typed_hir::LocalStorage::Value
                        || !local.ty.same_type(&disposer.owner)
                    {
                        return Err(invalid(
                            "Using cleanup does not own its resource",
                            scope.span,
                        ));
                    }
                }
                super::typed_hir::ScopeCleanup::FinallyRegion { finally_scope } => {
                    let Some(finally) = body.scopes.get(finally_scope.0) else {
                        return Err(invalid("Finally handler has an invalid scope", scope.span));
                    };
                    if finally.parent != scope.parent || finally.id == scope.id {
                        return Err(invalid(
                            "Finally handler is not a sibling of Try",
                            scope.span,
                        ));
                    }
                }
            }
        }
    }
    for (index, local) in body.locals.iter().enumerate() {
        if local.id != LocalId(index)
            || body
                .scopes
                .get(local.scope.0)
                .is_none_or(|scope| !scope.locals.contains(&local.id))
        {
            return Err(invalid("Local has no matching scope entry", local.span));
        }
    }
    for (index, field) in body.fields.iter().enumerate() {
        if field.id.0 != index {
            return Err(invalid("Field ID does not match its index", body.span));
        }
    }
    for (index, disposer) in body.disposers.iter().enumerate() {
        if disposer.id.0 != index {
            return Err(invalid(
                "Dispose method ID does not match its index",
                body.span,
            ));
        }
    }
    let mut visited = vec![false; body.scopes.len()];
    visited[0] = true;
    verify_statements(body, &body.statements, body.root_scope, &[], &mut visited)?;
    let mut handler_scopes = HashSet::new();
    collect_handler_scopes(&body.statements, &mut handler_scopes);
    if body
        .scopes
        .iter()
        .any(|scope| !scope.cleanup.is_empty() && !handler_scopes.contains(&scope.id))
    {
        return Err(invalid("Cleanup handler has no owning region", body.span));
    }
    if visited.iter().any(|used| !used) {
        return Err(invalid("HIR contains an unreachable scope", body.span));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct LoopFrame {
    id: LoopId,
    parent: ScopeId,
}

fn verify_statements(
    body: &TypedBody,
    statements: &[Statement],
    current: ScopeId,
    loops: &[LoopFrame],
    visited: &mut [bool],
) -> Result<(), Diagnostic> {
    for statement in statements {
        verify_statement_expressions(body, statement, current)?;
        match statement {
            Statement::Initialize { target, span, .. } => {
                if body
                    .locals
                    .get(target.0)
                    .is_none_or(|local| local.scope != current)
                {
                    return Err(invalid(
                        "Initializer targets a local outside its declaration scope",
                        *span,
                    ));
                }
            }
            Statement::Store { .. } => {}
            Statement::ReturnVoid {
                exited_scopes,
                cleanup_chain,
                span,
            } if body.return_type != TypeName::Void => {
                return Err(invalid("Void return in a value-returning function", *span));
            }
            Statement::ReturnVoid {
                exited_scopes,
                cleanup_chain,
                span,
            }
            | Statement::Return {
                exited_scopes,
                cleanup_chain,
                span,
                ..
            } => {
                let expected = chain(body, current, None)?;
                if *exited_scopes != expected {
                    return Err(invalid("Return has incorrect scope exits", *span));
                }
                verify_cleanup_chain(body, exited_scopes, cleanup_chain, *span)?;
            }
            Statement::CallSub { .. }
            | Statement::CollectionAdd { .. }
            | Statement::CollectionRemove { .. } => {}
            Statement::If {
                then_scope,
                then_body,
                else_scope,
                else_body,
                span,
                ..
            } => {
                visit_child(body, *then_scope, current, *span, visited)?;
                verify_statements(body, then_body, *then_scope, loops, visited)?;
                visit_child(body, *else_scope, current, *span, visited)?;
                verify_statements(body, else_body, *else_scope, loops, visited)?;
            }
            Statement::TryFinally {
                try_scope,
                try_body,
                finally_scope,
                finally_body,
                span,
            } => {
                if try_scope == finally_scope {
                    return Err(invalid("Try and Finally cannot share a scope", *span));
                }
                visit_child(body, *try_scope, current, *span, visited)?;
                if body.scopes[try_scope.0].cleanup
                    != vec![super::typed_hir::ScopeCleanup::FinallyRegion {
                        finally_scope: *finally_scope,
                    }]
                {
                    return Err(invalid("Try scope has no matching Finally handler", *span));
                }
                verify_statements(body, try_body, *try_scope, loops, visited)?;
                visit_child(body, *finally_scope, current, *span, visited)?;
                verify_statements(body, finally_body, *finally_scope, loops, visited)?;
                if has_nonlocal_exit(finally_body) {
                    return Err(invalid(
                        "Typed Finally cannot transfer control outward",
                        *span,
                    ));
                }
            }
            Statement::TryCatch {
                try_scope,
                try_body,
                catch_scope,
                catch_local,
                catch_body,
                finally_scope,
                finally_body,
                span,
            } => {
                if try_scope == catch_scope
                    || finally_scope.is_some_and(|id| id == *try_scope || id == *catch_scope)
                {
                    return Err(invalid(
                        "Try, Catch and Finally must have distinct scopes",
                        *span,
                    ));
                }
                let expected = finally_scope
                    .map(|finally_scope| {
                        vec![super::typed_hir::ScopeCleanup::FinallyRegion { finally_scope }]
                    })
                    .unwrap_or_default();
                visit_child(body, *try_scope, current, *span, visited)?;
                if body.scopes[try_scope.0].cleanup != expected {
                    return Err(invalid("Try scope has an incorrect Finally handler", *span));
                }
                verify_statements(body, try_body, *try_scope, loops, visited)?;
                visit_child(body, *catch_scope, current, *span, visited)?;
                if body.scopes[catch_scope.0].cleanup != expected {
                    return Err(invalid(
                        "Catch scope has an incorrect Finally handler",
                        *span,
                    ));
                }
                if let Some(local) = catch_local
                    && body.locals.get(local.0).is_none_or(|local| {
                        local.scope != *catch_scope
                            || local.initial_state != super::typed_hir::InitialState::Initialized
                    })
                {
                    return Err(invalid("Catch local is not initialized on entry", *span));
                }
                verify_statements(body, catch_body, *catch_scope, loops, visited)?;
                if let Some(finally_scope) = finally_scope {
                    visit_child(body, *finally_scope, current, *span, visited)?;
                    verify_statements(body, finally_body, *finally_scope, loops, visited)?;
                    if has_nonlocal_exit(finally_body) {
                        return Err(invalid(
                            "Typed Finally cannot transfer control outward",
                            *span,
                        ));
                    }
                } else if !finally_body.is_empty() {
                    return Err(invalid("Try/Catch body has an unattached Finally", *span));
                }
            }
            Statement::UsingDispose {
                resource,
                method,
                body_scope,
                body: child,
                span,
            } => {
                visit_child(body, *body_scope, current, *span, visited)?;
                if body.scopes[body_scope.0].cleanup
                    != vec![super::typed_hir::ScopeCleanup::ExplicitDispose {
                        resource: *resource,
                        method: *method,
                    }]
                {
                    return Err(invalid(
                        "Using scope has no matching Dispose cleanup",
                        *span,
                    ));
                }
                if !matches!(child.first(), Some(Statement::Initialize { target, .. }) if target == resource)
                {
                    return Err(invalid(
                        "Using resource is not initialized at scope entry",
                        *span,
                    ));
                }
                verify_statements(body, child, *body_scope, loops, visited)?;
            }
            Statement::While {
                id,
                body_scope,
                body: child,
                span,
                ..
            }
            | Statement::Do {
                id,
                body_scope,
                body: child,
                span,
                ..
            }
            | Statement::For {
                id,
                body_scope,
                body: child,
                span,
                ..
            }
            | Statement::ForEach {
                id,
                body_scope,
                body: child,
                span,
                ..
            } => {
                visit_child(body, *body_scope, current, *span, visited)?;
                let mut nested = loops.to_vec();
                nested.push(LoopFrame {
                    id: *id,
                    parent: current,
                });
                verify_statements(body, child, *body_scope, &nested, visited)?;
            }
            Statement::ExitLoop {
                loop_id,
                exited_scopes,
                cleanup_chain,
                span,
            }
            | Statement::ContinueLoop {
                loop_id,
                exited_scopes,
                cleanup_chain,
                span,
            } => {
                let frame = loops
                    .iter()
                    .rev()
                    .find(|frame| frame.id == *loop_id)
                    .ok_or_else(|| invalid("Loop control has no enclosing loop", *span))?;
                let expected = chain(body, current, Some(frame.parent))?;
                if *exited_scopes != expected {
                    return Err(invalid("Loop control has incorrect scope exits", *span));
                }
                verify_cleanup_chain(body, exited_scopes, cleanup_chain, *span)?;
            }
        }
    }
    Ok(())
}

fn has_nonlocal_exit(statements: &[Statement]) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Return { .. }
        | Statement::ReturnVoid { .. }
        | Statement::ExitLoop { .. }
        | Statement::ContinueLoop { .. } => true,
        Statement::If {
            then_body,
            else_body,
            ..
        } => has_nonlocal_exit(then_body) || has_nonlocal_exit(else_body),
        Statement::While { body, .. }
        | Statement::Do { body, .. }
        | Statement::For { body, .. }
        | Statement::ForEach { body, .. } => has_nonlocal_exit(body),
        Statement::TryFinally {
            try_body,
            finally_body,
            ..
        } => has_nonlocal_exit(try_body) || has_nonlocal_exit(finally_body),
        Statement::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            has_nonlocal_exit(try_body)
                || has_nonlocal_exit(catch_body)
                || has_nonlocal_exit(finally_body)
        }
        Statement::UsingDispose { body, .. } => has_nonlocal_exit(body),
        Statement::Initialize { .. }
        | Statement::Store { .. }
        | Statement::CallSub { .. }
        | Statement::CollectionAdd { .. }
        | Statement::CollectionRemove { .. } => false,
    })
}

fn collect_handler_scopes(statements: &[Statement], found: &mut HashSet<ScopeId>) {
    for statement in statements {
        match statement {
            Statement::UsingDispose {
                body_scope, body, ..
            } => {
                found.insert(*body_scope);
                collect_handler_scopes(body, found);
            }
            Statement::If {
                then_body,
                else_body,
                ..
            } => {
                collect_handler_scopes(then_body, found);
                collect_handler_scopes(else_body, found);
            }
            Statement::TryFinally {
                try_scope,
                try_body,
                finally_body,
                ..
            } => {
                found.insert(*try_scope);
                collect_handler_scopes(try_body, found);
                collect_handler_scopes(finally_body, found);
            }
            Statement::TryCatch {
                try_scope,
                try_body,
                catch_scope,
                catch_body,
                finally_body,
                finally_scope,
                ..
            } => {
                if finally_scope.is_some() {
                    found.insert(*try_scope);
                    found.insert(*catch_scope);
                }
                collect_handler_scopes(try_body, found);
                collect_handler_scopes(catch_body, found);
                collect_handler_scopes(finally_body, found);
            }
            Statement::While { body, .. }
            | Statement::Do { body, .. }
            | Statement::For { body, .. }
            | Statement::ForEach { body, .. } => collect_handler_scopes(body, found),
            _ => {}
        }
    }
}

fn verify_statement_expressions(
    body: &TypedBody,
    statement: &Statement,
    current: ScopeId,
) -> Result<(), Diagnostic> {
    match statement {
        Statement::Initialize { target, value, .. } => {
            verify_expression(body, value, current)?;
            if body
                .locals
                .get(target.0)
                .is_none_or(|local| !local.ty.same_type(&value.ty))
            {
                return Err(invalid(
                    "Initializer type does not match its local",
                    value.span,
                ));
            }
            Ok(())
        }
        Statement::Return { value, .. } => {
            verify_expression(body, value, current)?;
            if !value.ty.same_type(&body.return_type) {
                return Err(invalid(
                    "Return type does not match the function",
                    value.span,
                ));
            }
            Ok(())
        }
        Statement::ReturnVoid { span, .. } => {
            if body.return_type == TypeName::Void {
                Ok(())
            } else {
                Err(invalid("Void return in a value-returning function", *span))
            }
        }
        Statement::CallSub {
            function,
            signature,
            arguments,
            span,
        } => {
            if signature.return_type != TypeName::Void {
                return Err(invalid("Sub call must have Void return", *span));
            }
            verify_expression(
                body,
                &Expression {
                    kind: ExpressionKind::Call {
                        function: *function,
                        signature: signature.clone(),
                        arguments: arguments.clone(),
                    },
                    ty: TypeName::Void,
                    category: ValueCategory::Value,
                    span: *span,
                },
                current,
            )
        }
        Statement::CollectionAdd {
            collection,
            item,
            before,
            span,
        } => {
            verify_expression(body, collection, current)?;
            verify_expression(body, item, current)?;
            if let Some(before) = before.as_ref() {
                verify_expression(body, before, current)?;
            }
            if !matches!(&collection.ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
                || item.ty != TypeName::Variant
                || before
                    .as_ref()
                    .as_ref()
                    .is_some_and(|value| value.ty != TypeName::Int64)
            {
                return Err(invalid("Collection.Add types are invalid", *span));
            }
            Ok(())
        }
        Statement::CollectionRemove {
            collection,
            index,
            span,
        } => {
            verify_expression(body, collection, current)?;
            verify_expression(body, index, current)?;
            if !matches!(&collection.ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
                || index.ty != TypeName::Int64
            {
                return Err(invalid("Collection.Remove types are invalid", *span));
            }
            Ok(())
        }
        Statement::Store { target, value, .. } => {
            verify_expression(body, target, current)?;
            verify_expression(body, value, current)?;
            if target.category != ValueCategory::Place || !target.ty.same_type(&value.ty) {
                return Err(invalid(
                    "Store target and value types do not match",
                    value.span,
                ));
            }
            Ok(())
        }
        Statement::If { condition, .. } | Statement::While { condition, .. } => {
            verify_expression(body, condition, current)?;
            boolean_condition(condition)
        }
        Statement::Do { condition, .. } => match condition {
            DoCondition::Infinite => Ok(()),
            DoCondition::PreWhile(expr)
            | DoCondition::PreUntil(expr)
            | DoCondition::PostWhile(expr)
            | DoCondition::PostUntil(expr) => {
                verify_expression(body, expr, current)?;
                boolean_condition(expr)
            }
        },
        Statement::For {
            variable,
            start,
            end,
            step,
            span,
            ..
        } => {
            verify_local(body, *variable, current, *span)?;
            let variable_type = &body.locals[variable.0].ty;
            if !variable_type.is_integral() {
                return Err(invalid("For variable is not integral", *span));
            }
            for expression in [start.as_ref(), end.as_ref(), step.as_ref()] {
                verify_expression(body, expression, current)?;
                if !expression.ty.same_type(variable_type) {
                    return Err(invalid(
                        "For bound or step has the wrong type",
                        expression.span,
                    ));
                }
            }
            Ok(())
        }
        Statement::ForEach {
            variable,
            iterable,
            element_type,
            span,
            ..
        } => {
            verify_local(body, *variable, current, *span)?;
            verify_expression(body, iterable, current)?;
            if !body.locals[variable.0].ty.same_type(element_type)
                || !(matches!(&iterable.ty, TypeName::Array(inner) if inner.same_type(element_type))
                    || (iterable.ty
                        == TypeName::User(crate::runtime::well_known::COLLECTION.into())
                        && *element_type == TypeName::Variant))
            {
                return Err(invalid(
                    "For Each element type does not match its array and variable",
                    *span,
                ));
            }
            Ok(())
        }
        Statement::TryFinally { .. }
        | Statement::TryCatch { .. }
        | Statement::UsingDispose { .. }
        | Statement::ExitLoop { .. }
        | Statement::ContinueLoop { .. } => Ok(()),
    }
}

fn boolean_condition(condition: &Expression) -> Result<(), Diagnostic> {
    if condition.ty.same_type(&TypeName::Boolean) {
        Ok(())
    } else {
        Err(invalid(
            "Branch or loop condition is not Boolean",
            condition.span,
        ))
    }
}

fn verify_expression(
    body: &TypedBody,
    expression: &Expression,
    current: ScopeId,
) -> Result<(), Diagnostic> {
    match &expression.kind {
        ExpressionKind::Constant(_) | ExpressionKind::ArrayInit { .. } => Ok(()),
        ExpressionKind::NewCollection => {
            if matches!(&expression.ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
            {
                Ok(())
            } else {
                Err(invalid(
                    "Collection allocation has wrong type",
                    expression.span,
                ))
            }
        }
        ExpressionKind::BoxDynamic(value) => {
            verify_expression(body, value, current)?;
            if expression.ty == TypeName::Variant && value.ty != TypeName::Variant {
                Ok(())
            } else {
                Err(invalid("Dynamic box types are invalid", expression.span))
            }
        }
        ExpressionKind::UnboxDynamic { value, target } => {
            verify_expression(body, value, current)?;
            if value.ty == TypeName::Variant && expression.ty.same_type(target) {
                Ok(())
            } else {
                Err(invalid("Dynamic unbox types are invalid", expression.span))
            }
        }
        ExpressionKind::CollectionCount(value) => {
            verify_expression(body, value, current)?;
            if expression.ty == TypeName::Int32
                && matches!(&value.ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
            {
                Ok(())
            } else {
                Err(invalid(
                    "Collection.Count types are invalid",
                    expression.span,
                ))
            }
        }
        ExpressionKind::CollectionItem { collection, index } => {
            verify_expression(body, collection, current)?;
            verify_expression(body, index, current)?;
            if expression.ty == TypeName::Variant
                && index.ty == TypeName::Int64
                && matches!(&collection.ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
            {
                Ok(())
            } else {
                Err(invalid(
                    "Collection.Item types are invalid",
                    expression.span,
                ))
            }
        }
        ExpressionKind::NewClass(ty) => {
            if body.classes.iter().any(|class| class.same_type(ty)) && expression.ty.same_type(ty) {
                Ok(())
            } else {
                Err(invalid(
                    "Class allocation has an unresolved type",
                    expression.span,
                ))
            }
        }
        ExpressionKind::StringLen(value) => {
            verify_expression(body, value, current)?;
            if !value.ty.same_type(&TypeName::String) || !expression.ty.same_type(&TypeName::Int32)
            {
                return Err(invalid(
                    "String length has incompatible types",
                    expression.span,
                ));
            }
            Ok(())
        }
        ExpressionKind::StringFormat { value, decimals } => {
            verify_expression(body, value, current)?;
            if !expression.ty.same_type(&TypeName::String)
                || !(value.ty.is_integral()
                    || matches!(
                        value.ty,
                        TypeName::Single | TypeName::Double | TypeName::Boolean
                    ))
                || decimals.is_some_and(|digits| digits > 6)
            {
                return Err(invalid(
                    "String interpolation format is invalid",
                    expression.span,
                ));
            }
            Ok(())
        }
        ExpressionKind::StringConcat { left, right }
        | ExpressionKind::StringCompare { left, right, .. } => {
            verify_expression(body, left, current)?;
            verify_expression(body, right, current)?;
            let expected = if matches!(expression.kind, ExpressionKind::StringConcat { .. }) {
                TypeName::String
            } else {
                TypeName::Boolean
            };
            if !left.ty.same_type(&TypeName::String)
                || !right.ty.same_type(&TypeName::String)
                || !expression.ty.same_type(&expected)
            {
                return Err(invalid(
                    "String operation has incompatible types",
                    expression.span,
                ));
            }
            Ok(())
        }
        ExpressionKind::ReferenceIdentity { left, right, .. } => {
            verify_expression(body, left, current)?;
            verify_expression(body, right, current)?;
            if !left.ty.same_type(&right.ty)
                || !expression.ty.same_type(&TypeName::Boolean)
                || !body.classes.iter().any(|class| class.same_type(&left.ty))
            {
                return Err(invalid(
                    "Reference identity operands are invalid",
                    expression.span,
                ));
            }
            Ok(())
        }
        ExpressionKind::Tuple(values) => {
            let TypeName::Tuple(elements) = &expression.ty else {
                return Err(invalid(
                    "Tuple constructor has a non-tuple type",
                    expression.span,
                ));
            };
            if values.len() != elements.len() {
                return Err(invalid(
                    "Tuple constructor arity differs from its type",
                    expression.span,
                ));
            }
            for (value, element) in values.iter().zip(elements) {
                verify_expression(body, value, current)?;
                if !value.ty.same_type(&element.ty) {
                    return Err(invalid("Tuple element has the wrong type", value.span));
                }
            }
            Ok(())
        }
        ExpressionKind::Place(place) => {
            let local = place.root;
            verify_local(body, local, current, expression.span)?;
            let mut place_type = body.locals[local.0].ty.clone();
            for projection in &place.projections {
                match projection {
                    super::typed_hir::Projection::Field(id) => {
                        let field = body
                            .fields
                            .get(id.0)
                            .ok_or_else(|| invalid("Invalid field identity", expression.span))?;
                        if field.id != *id || !field.owner.same_type(&place_type) {
                            return Err(invalid(
                                "Field projection has the wrong owner type",
                                expression.span,
                            ));
                        }
                        place_type = field.ty.clone();
                    }
                    super::typed_hir::Projection::TupleField(index) => {
                        let TypeName::Tuple(elements) = &place_type else {
                            return Err(invalid(
                                "Tuple-field projection requires a tuple place",
                                expression.span,
                            ));
                        };
                        let element = elements.get(*index).ok_or_else(|| {
                            invalid("Tuple-field index is out of range", expression.span)
                        })?;
                        place_type = element.ty.clone();
                    }
                    super::typed_hir::Projection::Index(index) => {
                        verify_expression(body, &index.index, current)?;
                        let TypeName::Array(element) = &place_type else {
                            return Err(invalid(
                                "Index projection requires an array place",
                                expression.span,
                            ));
                        };
                        if !index.index.ty.is_integral() || !element.same_type(&index.element_type)
                        {
                            return Err(invalid(
                                "Index projection has incompatible types",
                                expression.span,
                            ));
                        }
                        place_type = index.element_type.clone();
                    }
                }
            }
            if expression.category != ValueCategory::Place || !place_type.same_type(&expression.ty)
            {
                return Err(invalid(
                    "Place has an incorrect type or category",
                    expression.span,
                ));
            }
            Ok(())
        }
        ExpressionKind::Load(place)
        | ExpressionKind::BorrowMutable(place)
        | ExpressionKind::BorrowImmutable(place)
        | ExpressionKind::Move(place) => {
            verify_expression(body, place, current)?;
            if place.category != ValueCategory::Place || !place.ty.same_type(&expression.ty) {
                return Err(invalid(
                    "Load or borrow does not match its place",
                    expression.span,
                ));
            }
            Ok(())
        }
        ExpressionKind::Convert { value, conversion } => {
            verify_expression(body, value, current)?;
            if !(value.ty.is_integral() || matches!(value.ty, TypeName::Single | TypeName::Double))
                || !(expression.ty.is_integral()
                    || matches!(expression.ty, TypeName::Single | TypeName::Double))
                || (*conversion == Conversion::TruncateToInteger && !expression.ty.is_integral())
            {
                return Err(invalid(
                    "Numeric conversion has incompatible types",
                    expression.span,
                ));
            }
            Ok(())
        }
        ExpressionKind::Arithmetic {
            signature,
            left,
            right,
            ..
        } => {
            verify_expression(body, left, current)?;
            verify_expression(body, right, current)?;
            if !left.ty.same_type(&signature.operand_type)
                || !right.ty.same_type(&signature.operand_type)
                || !expression.ty.same_type(&signature.result_type)
            {
                return Err(invalid(
                    "Arithmetic types do not match the resolved signature",
                    expression.span,
                ));
            }
            Ok(())
        }
        ExpressionKind::Compare { left, right, .. } => {
            verify_expression(body, left, current)?;
            verify_expression(body, right, current)?;
            if !left.ty.same_type(&right.ty) || !expression.ty.same_type(&TypeName::Boolean) {
                return Err(invalid(
                    "Comparison operands or result have incompatible types",
                    expression.span,
                ));
            }
            Ok(())
        }
        ExpressionKind::Call {
            signature,
            arguments,
            ..
        } => {
            if signature.parameter_types.len() != arguments.len()
                || signature.parameter_modes.len() != arguments.len()
                || !expression.ty.same_type(&signature.return_type)
            {
                return Err(invalid(
                    "Call does not match its resolved signature",
                    expression.span,
                ));
            }
            for ((argument, parameter_type), parameter_mode) in arguments
                .iter()
                .zip(&signature.parameter_types)
                .zip(&signature.parameter_modes)
            {
                if argument.mode != *parameter_mode {
                    return Err(invalid(
                        "Call argument has the wrong resolved passing mode",
                        argument.value.span,
                    ));
                }
                if !argument.value.ty.same_type(parameter_type) {
                    return Err(invalid(
                        "Call argument has the wrong resolved type",
                        argument.value.span,
                    ));
                }
                let mode_matches = matches!(
                    (&argument.mode, &argument.value.kind),
                    (ArgumentMode::ByVal, ExpressionKind::Constant(_))
                        | (ArgumentMode::ByVal, ExpressionKind::Tuple(_))
                        | (ArgumentMode::ByVal, ExpressionKind::ArrayInit { .. })
                        | (ArgumentMode::ByVal, ExpressionKind::Load(_))
                        | (ArgumentMode::ByVal, ExpressionKind::Convert { .. })
                        | (ArgumentMode::ByVal, ExpressionKind::Arithmetic { .. })
                        | (ArgumentMode::ByVal, ExpressionKind::Compare { .. })
                        | (ArgumentMode::ByVal, ExpressionKind::StringConcat { .. })
                        | (ArgumentMode::ByVal, ExpressionKind::StringCompare { .. })
                        | (ArgumentMode::ByVal, ExpressionKind::StringLen(_))
                        | (ArgumentMode::ByVal, ExpressionKind::StringFormat { .. })
                        | (ArgumentMode::ByVal, ExpressionKind::Call { .. })
                        | (
                            ArgumentMode::BorrowMutable,
                            ExpressionKind::BorrowMutable(_)
                        )
                        | (
                            ArgumentMode::BorrowImmutable,
                            ExpressionKind::BorrowImmutable(_)
                        )
                        | (ArgumentMode::Move, ExpressionKind::Move(_))
                );
                if !mode_matches {
                    return Err(invalid(
                        "Call argument mode does not match its value category",
                        argument.value.span,
                    ));
                }
                verify_expression(body, &argument.value, current)?;
            }
            Ok(())
        }
    }
}

fn verify_local(
    body: &TypedBody,
    local: LocalId,
    mut current: ScopeId,
    span: Span,
) -> Result<(), Diagnostic> {
    let declared = body
        .locals
        .get(local.0)
        .ok_or_else(|| invalid("Local ID is out of range", span))?
        .scope;
    loop {
        if current == declared {
            return Ok(());
        }
        current = body
            .scopes
            .get(current.0)
            .and_then(|scope| scope.parent)
            .ok_or_else(|| invalid("Local is referenced outside its lexical scope", span))?;
    }
}

fn visit_child(
    body: &TypedBody,
    child: ScopeId,
    parent: ScopeId,
    span: Span,
    visited: &mut [bool],
) -> Result<(), Diagnostic> {
    if body
        .scopes
        .get(child.0)
        .is_none_or(|scope| scope.parent != Some(parent))
        || visited.get(child.0).is_none_or(|already| *already)
    {
        return Err(invalid(
            "HIR child scope has an invalid parent or is reused",
            span,
        ));
    }
    visited[child.0] = true;
    Ok(())
}

fn chain(
    body: &TypedBody,
    mut current: ScopeId,
    stop: Option<ScopeId>,
) -> Result<Vec<ScopeId>, Diagnostic> {
    let mut scopes = Vec::new();
    while Some(current) != stop {
        let scope = body
            .scopes
            .get(current.0)
            .ok_or_else(|| invalid("Scope ID is out of range", body.span))?;
        scopes.push(current);
        if let Some(parent) = scope.parent {
            current = parent;
        } else if stop.is_some() {
            return Err(invalid("Scope exit does not reach its target", body.span));
        } else {
            break;
        }
    }
    Ok(scopes)
}

fn verify_cleanup_chain(
    body: &TypedBody,
    exited_scopes: &[ScopeId],
    actual: &[super::typed_hir::CleanupStep],
    span: Span,
) -> Result<(), Diagnostic> {
    if actual != body.cleanup_chain(exited_scopes) {
        return Err(invalid("Transfer has an incorrect cleanup chain", span));
    }
    Ok(())
}

fn invalid(message: &str, span: Span) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::TYPE_MISMATCH, message, Some(span))
}
