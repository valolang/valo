//! Stable, source-independent textual view of the typed HIR.
use std::fmt::Write;

use super::typed_hir::{
    CallArgument, DoCondition, Expression, ExpressionKind, ScopeId, Statement, TypedBody,
};

pub fn format_ownership_report(report: &super::ownership::OwnershipReport) -> String {
    let mut output = String::new();
    for replacement in &report.replacements {
        writeln!(
            output,
            "replace local #{}: old {:?}",
            replacement.place.root.0, replacement.old_value
        )
        .unwrap();
    }
    for exit in &report.exits {
        writeln!(
            output,
            "exit {:?} {:?}:",
            exit.kind,
            exit.exited_scopes
                .iter()
                .map(|scope| scope.0)
                .collect::<Vec<_>>()
        )
        .unwrap();
        for (local, action) in &exit.locals {
            writeln!(output, "  local #{}: {action:?}", local.0).unwrap();
        }
        for (local, method, action) in &exit.disposals {
            writeln!(
                output,
                "  dispose local #{} via #{}: {action:?}",
                local.0, method.0
            )
            .unwrap();
        }
        for handler in &exit.handlers {
            writeln!(output, "  handler {handler:?}").unwrap();
        }
    }
    output
}

pub fn format_body(body: &TypedBody) -> String {
    let mut output = String::new();
    writeln!(
        output,
        "fn #{} {} -> {:?}",
        body.function.0, body.name, body.return_type
    )
    .unwrap();
    write_block(&mut output, body, body.root_scope, &body.statements, 0);
    output
}

fn write_block(
    output: &mut String,
    body: &TypedBody,
    scope_id: ScopeId,
    statements: &[Statement],
    depth: usize,
) {
    let scope = &body.scopes[scope_id.0];
    let indent = "  ".repeat(depth);
    writeln!(
        output,
        "{indent}scope #{} parent {}",
        scope_id.0,
        scope
            .parent
            .map(|id| format!("#{}", id.0))
            .unwrap_or_else(|| "none".into())
    )
    .unwrap();
    for local in &scope.locals {
        let local = &body.locals[local.0];
        writeln!(
            output,
            "{indent}  local #{} {}: {:?} {:?} {:?} copy={:?} drop={:?}",
            local.id.0,
            local.name,
            local.ty,
            local.storage,
            local.initial_state,
            local.properties.copy,
            local.properties.requires_drop
        )
        .unwrap();
    }
    for statement in statements {
        write_statement(output, body, statement, depth + 1);
    }
}

fn write_statement(output: &mut String, body: &TypedBody, statement: &Statement, depth: usize) {
    let indent = "  ".repeat(depth);
    match statement {
        Statement::Initialize { target, value, .. } => {
            writeln!(output, "{indent}init #{} = {}", target.0, expression(value)).unwrap()
        }
        Statement::Store { target, value, .. } => writeln!(
            output,
            "{indent}store {} = {}",
            expression(target),
            expression(value)
        )
        .unwrap(),
        Statement::Return {
            value,
            exited_scopes,
            cleanup_chain,
            ..
        } => writeln!(
            output,
            "{indent}return {} exits {} cleanup {:?}",
            expression(value),
            scopes(exited_scopes),
            cleanup_chain
        )
        .unwrap(),
        Statement::ReturnVoid {
            exited_scopes,
            cleanup_chain,
            ..
        } => writeln!(
            output,
            "{indent}return void exits {} cleanup {:?}",
            scopes(exited_scopes),
            cleanup_chain
        )
        .unwrap(),
        Statement::CallSub {
            function,
            arguments,
            ..
        } => writeln!(
            output,
            "{indent}call sub#{}({} args)",
            function.0,
            arguments.len()
        )
        .unwrap(),
        Statement::If {
            condition,
            then_scope,
            then_body,
            else_scope,
            else_body,
            ..
        } => {
            writeln!(output, "{indent}if {}", expression(condition)).unwrap();
            write_block(output, body, *then_scope, then_body, depth + 1);
            writeln!(output, "{indent}else").unwrap();
            write_block(output, body, *else_scope, else_body, depth + 1);
        }
        Statement::TryFinally {
            try_scope,
            try_body,
            finally_scope,
            finally_body,
            ..
        } => {
            writeln!(output, "{indent}try").unwrap();
            write_block(output, body, *try_scope, try_body, depth + 1);
            writeln!(output, "{indent}finally").unwrap();
            write_block(output, body, *finally_scope, finally_body, depth + 1);
        }
        Statement::TryCatch {
            try_scope,
            try_body,
            catch_scope,
            catch_local,
            catch_body,
            finally_scope,
            finally_body,
            ..
        } => {
            writeln!(output, "{indent}try").unwrap();
            write_block(output, body, *try_scope, try_body, depth + 1);
            writeln!(output, "{indent}catch local {catch_local:?}").unwrap();
            write_block(output, body, *catch_scope, catch_body, depth + 1);
            if let Some(finally_scope) = finally_scope {
                writeln!(output, "{indent}finally").unwrap();
                write_block(output, body, *finally_scope, finally_body, depth + 1);
            }
        }
        Statement::UsingDispose {
            resource,
            method,
            body_scope,
            body: using_body,
            ..
        } => {
            writeln!(
                output,
                "{indent}using dispose local #{} via #{}",
                resource.0, method.0
            )
            .unwrap();
            write_block(output, body, *body_scope, using_body, depth + 1);
        }
        Statement::While {
            id,
            condition,
            body_scope,
            body: loop_body,
            ..
        } => {
            writeln!(output, "{indent}while #{} {}", id.0, expression(condition)).unwrap();
            write_block(output, body, *body_scope, loop_body, depth + 1);
        }
        Statement::Do {
            id,
            condition,
            body_scope,
            body: loop_body,
            ..
        } => {
            let condition = match condition {
                DoCondition::Infinite => "infinite".into(),
                DoCondition::PreWhile(e) => format!("pre while {}", expression(e)),
                DoCondition::PreUntil(e) => format!("pre until {}", expression(e)),
                DoCondition::PostWhile(e) => format!("post while {}", expression(e)),
                DoCondition::PostUntil(e) => format!("post until {}", expression(e)),
            };
            writeln!(output, "{indent}do #{} {condition}", id.0).unwrap();
            write_block(output, body, *body_scope, loop_body, depth + 1);
        }
        Statement::For {
            id,
            variable,
            start,
            end,
            step,
            body_scope,
            body: loop_body,
            ..
        } => {
            writeln!(
                output,
                "{indent}for #{} local #{} from {} to {} step {}",
                id.0,
                variable.0,
                expression(start),
                expression(end),
                expression(step)
            )
            .unwrap();
            write_block(output, body, *body_scope, loop_body, depth + 1);
        }
        Statement::ForEach {
            id,
            variable,
            iterable,
            element_type,
            body_scope,
            body: loop_body,
            ..
        } => {
            writeln!(
                output,
                "{indent}foreach #{} local #{}: {:?} in {}",
                id.0,
                variable.0,
                element_type,
                expression(iterable)
            )
            .unwrap();
            write_block(output, body, *body_scope, loop_body, depth + 1);
        }
        Statement::ExitLoop {
            loop_id,
            exited_scopes,
            cleanup_chain,
            ..
        } => writeln!(
            output,
            "{indent}exit #{} exits {} cleanup {:?}",
            loop_id.0,
            scopes(exited_scopes),
            cleanup_chain
        )
        .unwrap(),
        Statement::ContinueLoop {
            loop_id,
            exited_scopes,
            cleanup_chain,
            ..
        } => writeln!(
            output,
            "{indent}continue #{} exits {} cleanup {:?}",
            loop_id.0,
            scopes(exited_scopes),
            cleanup_chain
        )
        .unwrap(),
    }
}

fn expression(expression: &Expression) -> String {
    let content = match &expression.kind {
        ExpressionKind::Constant(value) => format!("{value:?}"),
        ExpressionKind::StringConcat { left, right } => format!(
            "string.concat({}, {})",
            self::expression(left),
            self::expression(right)
        ),
        ExpressionKind::StringCompare {
            operation,
            left,
            right,
            text,
        } => format!(
            "string.cmp.{operation:?}.text={text}({}, {})",
            self::expression(left),
            self::expression(right)
        ),
        ExpressionKind::StringLen(value) => format!("string.len({})", self::expression(value)),
        ExpressionKind::StringFormat { value, decimals } => {
            format!("string.format.{decimals:?}({})", self::expression(value))
        }
        ExpressionKind::Tuple(values) => format!(
            "tuple({})",
            values
                .iter()
                .map(self::expression)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ExpressionKind::ArrayInit { lower, upper } => format!("array[{lower}..={upper}]"),
        ExpressionKind::Place(place) => {
            let mut rendered = format!("local #{}", place.root.0);
            for projection in &place.projections {
                match projection {
                    super::typed_hir::Projection::Field(id) => {
                        rendered.push_str(&format!(".field#{}", id.0))
                    }
                    super::typed_hir::Projection::TupleField(id) => {
                        rendered.push_str(&format!(".tuple#{id}"))
                    }
                    super::typed_hir::Projection::Index(index) => {
                        rendered.push_str(&format!("[{}]", self::expression(&index.index)))
                    }
                }
            }
            rendered
        }
        ExpressionKind::Load(place) => format!("load({})", self::expression(place)),
        ExpressionKind::BorrowMutable(place) => format!("borrow_mut({})", self::expression(place)),
        ExpressionKind::BorrowImmutable(place) => {
            format!("borrow_readonly({})", self::expression(place))
        }
        ExpressionKind::Move(place) => format!("move({})", self::expression(place)),
        ExpressionKind::Convert { value, conversion } => {
            format!("convert.{conversion:?}({})", self::expression(value))
        }
        ExpressionKind::Arithmetic {
            operation,
            left,
            right,
            ..
        } => format!(
            "{operation:?}({}, {})",
            self::expression(left),
            self::expression(right)
        ),
        ExpressionKind::Compare {
            operation,
            left,
            right,
        } => format!(
            "cmp.{operation:?}({}, {})",
            self::expression(left),
            self::expression(right)
        ),
        ExpressionKind::Call {
            function,
            signature,
            arguments,
        } => {
            let args = arguments
                .iter()
                .map(call_argument)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "call #{}({args}) -> {:?}",
                function.0, signature.return_type
            )
        }
    };
    format!("{content}:{:?}", expression.ty)
}

fn call_argument(argument: &CallArgument) -> String {
    format!("{:?} {}", argument.mode, expression(&argument.value))
}

fn scopes(ids: &[ScopeId]) -> String {
    format!(
        "[{}]",
        ids.iter()
            .map(|id| format!("#{}", id.0))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
