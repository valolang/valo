//! HIR-to-MIR lowering. Only verified HIR is accepted; unsupported features
//! produce a controlled error instead of a partially valid control-flow graph.
use std::collections::HashMap;

use super::ir as m;
use super::verify;
use crate::frontend::semantics::arithmetic::ArithmeticOp;
use crate::frontend::semantics::typed_hir as h;
use crate::frontend::type_model::TypeName;
use crate::runtime::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowerError {
    Unsupported { feature: &'static str, span: Span },
    InvalidHir(String),
    InvalidMir(String),
    Analysis(String),
}

pub fn lower_module(bodies: &[h::TypedBody]) -> Result<m::Module, LowerError> {
    bodies
        .iter()
        .map(lower_body)
        .collect::<Result<Vec<_>, _>>()
        .map(|functions| m::Module {
            functions,
            entry: None,
        })
}

pub fn lower_body(body: &h::TypedBody) -> Result<m::Function, LowerError> {
    crate::frontend::semantics::verify_hir::verify_body(body)
        .map_err(|error| LowerError::InvalidHir(error.message.to_string()))?;
    crate::frontend::semantics::ownership::check_body(body)
        .map_err(|error| LowerError::InvalidHir(error.message.to_string()))?;
    let mut builder = Builder::new(body);
    builder.lower_statements(&body.statements)?;
    if builder.current.is_some() {
        return Err(LowerError::Unsupported {
            feature: "function fallthrough without Return",
            span: body.span,
        });
    }
    super::drop_elaboration::elaborate(&mut builder.function).map_err(LowerError::Analysis)?;
    verify::verify(&builder.function).map_err(LowerError::InvalidMir)?;
    super::analysis::ownership::analyze(&builder.function).map_err(LowerError::Analysis)?;
    Ok(builder.function)
}

#[derive(Clone, Copy)]
struct LoopFrame {
    id: h::LoopId,
    exit: m::BlockId,
    continue_at: m::BlockId,
}

struct Builder<'a> {
    body: &'a h::TypedBody,
    function: m::Function,
    current: Option<m::BlockId>,
    loops: Vec<LoopFrame>,
    finally_bodies: HashMap<h::ScopeId, &'a [h::Statement]>,
}

impl<'a> Builder<'a> {
    fn new(body: &'a h::TypedBody) -> Self {
        let mut finally_bodies = HashMap::new();
        collect_finally(&body.statements, &mut finally_bodies);
        Self {
            body,
            function: m::Function {
                id: body.function,
                name: body.name.clone(),
                symbol_name: body.symbol_name.clone(),
                return_type: body.return_type.clone(),
                locals: body
                    .locals
                    .iter()
                    .map(|local| m::Local {
                        id: m::LocalId(local.id.0),
                        ty: local.ty.clone(),
                        properties: local.properties,
                        storage: local.storage,
                        parameter_index: local.parameter_index,
                        span: local.span,
                    })
                    .collect(),
                structures: body.structures.clone(),
                fields: body
                    .fields
                    .iter()
                    .map(|field| m::Field {
                        id: field.id,
                        owner: field.owner.clone(),
                        ty: field.ty.clone(),
                    })
                    .collect(),
                disposers: body
                    .disposers
                    .iter()
                    .map(|method| method.owner.clone())
                    .collect(),
                temps: Vec::new(),
                blocks: vec![m::BasicBlock {
                    id: m::BlockId(0),
                    label: Some("entry".into()),
                    instructions: Vec::new(),
                    terminator: None,
                }],
                entry: m::BlockId(0),
                span: body.span,
            },
            current: Some(m::BlockId(0)),
            loops: Vec::new(),
            finally_bodies,
        }
    }

    fn block(&mut self, label: &str) -> m::BlockId {
        let id = m::BlockId(self.function.blocks.len());
        self.function.blocks.push(m::BasicBlock {
            id,
            label: Some(label.into()),
            instructions: Vec::new(),
            terminator: None,
        });
        id
    }

    fn temp(&mut self, ty: TypeName) -> m::TempId {
        let id = m::TempId(self.function.temps.len());
        self.function.temps.push(ty);
        id
    }

    fn emit(
        &mut self,
        kind: m::InstructionKind,
        ty: Option<TypeName>,
        span: Span,
    ) -> Option<m::TempId> {
        let result = ty.map(|ty| self.temp(ty));
        let current = self.current.expect("emission into a live MIR block");
        self.function.blocks[current.0]
            .instructions
            .push(m::Instruction { result, kind, span });
        result
    }

    fn value(&mut self, kind: m::InstructionKind, ty: TypeName, span: Span) -> m::TempId {
        self.emit(kind, Some(ty), span)
            .expect("value instruction has a result")
    }

    fn terminate(&mut self, kind: m::TerminatorKind, span: Span) {
        let current = self.current.take().expect("terminating a live MIR block");
        assert!(self.function.blocks[current.0].terminator.is_none());
        self.function.blocks[current.0].terminator = Some(m::Terminator { kind, span });
    }

    fn goto(&mut self, target: m::BlockId, span: Span) {
        self.terminate(m::TerminatorKind::Goto(target), span);
    }

    fn lower_statements(&mut self, statements: &[h::Statement]) -> Result<(), LowerError> {
        for statement in statements {
            if self.current.is_none() {
                break;
            }
            self.lower_statement(statement)?;
        }
        Ok(())
    }

    fn lower_statement(&mut self, statement: &h::Statement) -> Result<(), LowerError> {
        match statement {
            h::Statement::Initialize {
                target,
                value,
                span,
            } => {
                let value = self.expr(value)?;
                self.store(self.local_place(*target), value, *span);
            }
            h::Statement::Store {
                target,
                value,
                span,
            } => {
                let place = self.place_expr(target)?;
                let value = self.expr(value)?;
                if place.ty == TypeName::String && place.projections.is_empty() {
                    self.emit(m::InstructionKind::Replace { place, value }, None, *span);
                } else {
                    self.store(place, value, *span);
                }
            }
            h::Statement::Return {
                value,
                exited_scopes,
                cleanup_chain,
                span,
                ..
            } => {
                let value = self.expr(value)?;
                self.exit_cleanup(exited_scopes, cleanup_chain, *span)?;
                self.terminate(m::TerminatorKind::Return(value), *span);
            }
            h::Statement::ReturnVoid {
                exited_scopes,
                cleanup_chain,
                span,
                ..
            } => {
                self.exit_cleanup(exited_scopes, cleanup_chain, *span)?;
                self.terminate(m::TerminatorKind::ReturnVoid, *span);
            }
            h::Statement::CallSub {
                function,
                signature,
                arguments,
                span,
            } => {
                let lowered = self.call_arguments(arguments)?;
                self.emit(
                    m::InstructionKind::Call {
                        target: m::CallTarget::Function(*function),
                        arguments: lowered,
                        parameter_types: signature.parameter_types.clone(),
                        parameter_modes: signature.parameter_modes.clone(),
                        return_type: None,
                    },
                    None,
                    *span,
                );
            }
            h::Statement::If {
                condition,
                then_scope,
                then_body,
                else_scope,
                else_body,
                span,
                ..
            } => {
                self.lower_if(
                    condition,
                    *then_scope,
                    then_body,
                    *else_scope,
                    else_body,
                    *span,
                )?;
            }
            h::Statement::While {
                id,
                condition,
                body_scope,
                body,
                span,
                ..
            } => {
                self.lower_while(*id, condition, *body_scope, body, *span)?;
            }
            h::Statement::Do {
                id,
                condition,
                body_scope,
                body,
                span,
                ..
            } => {
                self.lower_do(*id, condition, *body_scope, body, *span)?;
            }
            h::Statement::For { .. } => self.lower_for(statement)?,
            h::Statement::ForEach { .. } => self.lower_foreach(statement)?,
            h::Statement::ExitLoop {
                loop_id,
                exited_scopes,
                cleanup_chain,
                span,
                ..
            }
            | h::Statement::ContinueLoop {
                loop_id,
                exited_scopes,
                cleanup_chain,
                span,
                ..
            } => {
                let frame = self
                    .loops
                    .iter()
                    .rev()
                    .find(|frame| frame.id == *loop_id)
                    .copied()
                    .ok_or_else(|| LowerError::InvalidHir("loop target is not active".into()))?;
                let target = if matches!(statement, h::Statement::ExitLoop { .. }) {
                    frame.exit
                } else {
                    frame.continue_at
                };
                self.exit_cleanup(exited_scopes, cleanup_chain, *span)?;
                self.goto(target, *span);
            }
            h::Statement::TryFinally {
                try_scope,
                try_body,
                span,
                ..
            } => {
                self.lower_statements(try_body)?;
                if self.current.is_some() {
                    self.exit_cleanup(
                        &[*try_scope],
                        &self.body.cleanup_chain(&[*try_scope]),
                        *span,
                    )?;
                }
            }
            h::Statement::UsingDispose {
                body_scope,
                body,
                span,
                ..
            } => {
                self.lower_statements(body)?;
                if self.current.is_some() {
                    self.exit_cleanup(
                        &[*body_scope],
                        &self.body.cleanup_chain(&[*body_scope]),
                        *span,
                    )?;
                }
            }
            h::Statement::TryCatch { span, .. } => {
                return Err(LowerError::Unsupported {
                    feature: "native Catch dispatch and exception edges",
                    span: *span,
                });
            }
        }
        Ok(())
    }

    fn lower_if(
        &mut self,
        condition: &h::Expression,
        then_scope: h::ScopeId,
        then_body: &[h::Statement],
        else_scope: h::ScopeId,
        else_body: &[h::Statement],
        span: Span,
    ) -> Result<(), LowerError> {
        let condition = self.expr(condition)?;
        let yes = self.block("if.then");
        let no = self.block("if.else");
        let join = self.block("if.join");
        self.terminate(
            m::TerminatorKind::Branch {
                condition,
                then_block: yes,
                else_block: no,
            },
            span,
        );
        self.current = Some(yes);
        self.lower_statements(then_body)?;
        let then_live = self.current.is_some();
        if then_live {
            self.exit_cleanup(&[then_scope], &[], span)?;
            self.goto(join, span);
        }
        self.current = Some(no);
        self.lower_statements(else_body)?;
        let else_live = self.current.is_some();
        if else_live {
            self.exit_cleanup(&[else_scope], &[], span)?;
            self.goto(join, span);
        }
        if then_live || else_live {
            self.current = Some(join);
        } else {
            self.function.blocks[join.0].terminator = Some(m::Terminator {
                kind: m::TerminatorKind::Unreachable,
                span,
            });
            self.current = None;
        }
        Ok(())
    }

    fn lower_while(
        &mut self,
        id: h::LoopId,
        condition: &h::Expression,
        body_scope: h::ScopeId,
        body: &[h::Statement],
        span: Span,
    ) -> Result<(), LowerError> {
        let test = self.block("while.test");
        let loop_body = self.block("while.body");
        let exit = self.block("while.exit");
        self.goto(test, span);
        self.current = Some(test);
        let condition = self.expr(condition)?;
        self.terminate(
            m::TerminatorKind::Branch {
                condition,
                then_block: loop_body,
                else_block: exit,
            },
            span,
        );
        self.loops.push(LoopFrame {
            id,
            exit,
            continue_at: test,
        });
        self.current = Some(loop_body);
        self.lower_statements(body)?;
        if self.current.is_some() {
            self.exit_cleanup(&[body_scope], &[], span)?;
            self.goto(test, span);
        }
        self.loops.pop();
        self.current = Some(exit);
        Ok(())
    }

    fn lower_do(
        &mut self,
        id: h::LoopId,
        condition: &h::DoCondition,
        body_scope: h::ScopeId,
        body: &[h::Statement],
        span: Span,
    ) -> Result<(), LowerError> {
        let test = self.block("do.test");
        let loop_body = self.block("do.body");
        let exit = self.block("do.exit");
        let post = matches!(
            condition,
            h::DoCondition::PostWhile(_) | h::DoCondition::PostUntil(_)
        );
        self.goto(if post { loop_body } else { test }, span);
        self.loops.push(LoopFrame {
            id,
            exit,
            continue_at: test,
        });
        self.current = Some(loop_body);
        self.lower_statements(body)?;
        if self.current.is_some() {
            self.exit_cleanup(&[body_scope], &[], span)?;
            self.goto(test, span);
        }
        self.current = Some(test);
        match condition {
            h::DoCondition::Infinite => self.goto(loop_body, span),
            h::DoCondition::PreWhile(expr) | h::DoCondition::PostWhile(expr) => {
                let condition = self.expr(expr)?;
                self.terminate(
                    m::TerminatorKind::Branch {
                        condition,
                        then_block: loop_body,
                        else_block: exit,
                    },
                    span,
                );
            }
            h::DoCondition::PreUntil(expr) | h::DoCondition::PostUntil(expr) => {
                let condition = self.expr(expr)?;
                self.terminate(
                    m::TerminatorKind::Branch {
                        condition,
                        then_block: exit,
                        else_block: loop_body,
                    },
                    span,
                );
            }
        }
        self.loops.pop();
        self.current = Some(exit);
        Ok(())
    }

    fn lower_for(&mut self, statement: &h::Statement) -> Result<(), LowerError> {
        let h::Statement::For {
            id,
            variable,
            start,
            end,
            step,
            body_scope,
            body,
            span,
            ..
        } = statement
        else {
            return Err(LowerError::InvalidHir("expected For statement".into()));
        };
        let (id, variable, span) = (*id, *variable, *span);
        // VB For evaluates start/end/step once, before entering the loop.
        let start_value = self.expr(start)?;
        let end_value = self.expr(end)?;
        let step_value = self.expr(step)?;
        let variable = self.local_place(variable);
        self.store(variable.clone(), start_value, span);
        let test = self.block("for.test");
        let nonzero = self.block("for.nonzero_step");
        let zero_step = self.block("for.zero_step");
        let positive = self.block("for.positive");
        let negative = self.block("for.negative");
        let loop_body = self.block("for.body");
        let advance = self.block("for.step");
        let exit = self.block("for.exit");
        self.goto(test, span);
        self.current = Some(test);
        let zero = self.integer(0, &variable.ty, span);
        let is_zero = self.value(
            m::InstructionKind::Compare {
                op: h::ComparisonOp::Equal,
                left: step_value,
                right: zero,
            },
            TypeName::Boolean,
            span,
        );
        self.terminate(
            m::TerminatorKind::Branch {
                condition: is_zero,
                then_block: zero_step,
                else_block: nonzero,
            },
            span,
        );
        self.current = Some(zero_step);
        self.terminate(m::TerminatorKind::Trap("For Step cannot be zero"), span);
        self.current = Some(nonzero);
        let nonnegative = self.value(
            m::InstructionKind::Compare {
                op: h::ComparisonOp::GreaterEqual,
                left: step_value,
                right: zero,
            },
            TypeName::Boolean,
            span,
        );
        self.terminate(
            m::TerminatorKind::Branch {
                condition: nonnegative,
                then_block: positive,
                else_block: negative,
            },
            span,
        );
        self.current = Some(positive);
        let current = self.value(
            m::InstructionKind::Load(variable.clone()),
            variable.ty.clone(),
            span,
        );
        let in_range = self.value(
            m::InstructionKind::Compare {
                op: h::ComparisonOp::LessEqual,
                left: current,
                right: end_value,
            },
            TypeName::Boolean,
            span,
        );
        self.terminate(
            m::TerminatorKind::Branch {
                condition: in_range,
                then_block: loop_body,
                else_block: exit,
            },
            span,
        );
        self.current = Some(negative);
        let current = self.value(
            m::InstructionKind::Load(variable.clone()),
            variable.ty.clone(),
            span,
        );
        let in_range = self.value(
            m::InstructionKind::Compare {
                op: h::ComparisonOp::GreaterEqual,
                left: current,
                right: end_value,
            },
            TypeName::Boolean,
            span,
        );
        self.terminate(
            m::TerminatorKind::Branch {
                condition: in_range,
                then_block: loop_body,
                else_block: exit,
            },
            span,
        );
        self.loops.push(LoopFrame {
            id,
            exit,
            continue_at: advance,
        });
        self.current = Some(loop_body);
        self.lower_statements(body)?;
        if self.current.is_some() {
            self.exit_cleanup(&[*body_scope], &[], span)?;
            self.goto(advance, span);
        }
        self.current = Some(advance);
        let current = self.value(
            m::InstructionKind::Load(variable.clone()),
            variable.ty.clone(),
            span,
        );
        let next = self.value(
            m::InstructionKind::Arithmetic {
                op: ArithmeticOp::Add,
                left: current,
                right: step_value,
            },
            variable.ty.clone(),
            span,
        );
        self.store(variable, next, span);
        self.goto(test, span);
        self.loops.pop();
        self.current = Some(exit);
        Ok(())
    }

    fn lower_foreach(&mut self, statement: &h::Statement) -> Result<(), LowerError> {
        let h::Statement::ForEach {
            id,
            variable,
            iterable,
            element_type,
            body_scope,
            body,
            span,
        } = statement
        else {
            return Err(LowerError::InvalidHir("expected For Each statement".into()));
        };
        let (id, variable, body_scope, span) = (*id, *variable, *body_scope, *span);
        let h::ExpressionKind::Load(source) = &iterable.kind else {
            return Err(LowerError::Unsupported {
                feature: "For Each over a non-addressable array",
                span,
            });
        };
        let array = self.place_expr(source)?;
        if !matches!(array.ty, TypeName::Array(_)) {
            return Err(LowerError::InvalidHir(
                "For Each source is not an array".into(),
            ));
        }
        // The interpreter enumerates an element snapshot. Preserve that
        // visible behavior even if the source array changes in the body.
        let snapshot = self.value(
            m::InstructionKind::SnapshotArray(array.clone()),
            array.ty.clone(),
            span,
        );
        let snapshot_local = m::LocalId(self.function.locals.len());
        self.function.locals.push(m::Local {
            id: snapshot_local,
            ty: array.ty.clone(),
            properties: crate::frontend::semantics::type_properties::TypeProperties {
                copy: crate::frontend::semantics::type_properties::KnownProperty::Unknown,
                requires_drop: crate::frontend::semantics::type_properties::KnownProperty::Unknown,
            },
            storage: h::LocalStorage::Value,
            parameter_index: None,
            span,
        });
        let array = m::Place {
            root: snapshot_local,
            projections: Vec::new(),
            ty: array.ty,
        };
        self.store(array.clone(), snapshot, span);
        let length = self.value(
            m::InstructionKind::ArrayLen(array.clone()),
            TypeName::Int64,
            span,
        );
        let index_local = m::LocalId(self.function.locals.len());
        self.function.locals.push(m::Local {
            id: index_local,
            ty: TypeName::Int64,
            properties: crate::frontend::semantics::type_properties::TypeProperties {
                copy: crate::frontend::semantics::type_properties::KnownProperty::Yes,
                requires_drop: crate::frontend::semantics::type_properties::KnownProperty::No,
            },
            storage: h::LocalStorage::Value,
            parameter_index: None,
            span,
        });
        let index_place = m::Place {
            root: index_local,
            projections: Vec::new(),
            ty: TypeName::Int64,
        };
        let zero = self.integer(0, &TypeName::Int64, span);
        self.store(index_place.clone(), zero, span);
        let test = self.block("foreach.test");
        let loop_body = self.block("foreach.body");
        let advance = self.block("foreach.step");
        let exit = self.block("foreach.exit");
        self.goto(test, span);
        self.current = Some(test);
        let index = self.value(
            m::InstructionKind::Load(index_place.clone()),
            TypeName::Int64,
            span,
        );
        let in_range = self.value(
            m::InstructionKind::Compare {
                op: h::ComparisonOp::Less,
                left: index,
                right: length,
            },
            TypeName::Boolean,
            span,
        );
        self.terminate(
            m::TerminatorKind::Branch {
                condition: in_range,
                then_block: loop_body,
                else_block: exit,
            },
            span,
        );
        self.loops.push(LoopFrame {
            id,
            exit,
            continue_at: advance,
        });
        self.current = Some(loop_body);
        let index = self.value(
            m::InstructionKind::Load(index_place.clone()),
            TypeName::Int64,
            span,
        );
        let mut element = array;
        element.projections.push(m::Projection::Index(index));
        element.ty = element_type.clone();
        let value = self.value(
            m::InstructionKind::Load(element),
            element_type.clone(),
            span,
        );
        self.store(self.local_place(variable), value, span);
        self.lower_statements(body)?;
        if self.current.is_some() {
            self.exit_cleanup(&[body_scope], &[], span)?;
            self.goto(advance, span);
        }
        self.current = Some(advance);
        let index = self.value(
            m::InstructionKind::Load(index_place.clone()),
            TypeName::Int64,
            span,
        );
        let one = self.integer(1, &TypeName::Int64, span);
        let next = self.value(
            m::InstructionKind::Arithmetic {
                op: ArithmeticOp::Add,
                left: index,
                right: one,
            },
            TypeName::Int64,
            span,
        );
        self.store(index_place, next, span);
        self.goto(test, span);
        self.loops.pop();
        self.current = Some(exit);
        Ok(())
    }

    fn exit_cleanup(
        &mut self,
        scopes: &[h::ScopeId],
        chain: &[h::CleanupStep],
        span: Span,
    ) -> Result<(), LowerError> {
        for scope in scopes {
            let owned_steps = chain
                .iter()
                .filter(|step| step.owner == *scope)
                .cloned()
                .collect::<Vec<_>>();
            self.cleanup_chain(&owned_steps, span)?;
            for local in self.body.scopes[scope.0].locals.iter().rev() {
                let owned = &self.body.locals[local.0];
                if owned.storage == h::LocalStorage::Value {
                    self.emit(
                        m::InstructionKind::DropCandidate(m::LocalId(local.0)),
                        None,
                        span,
                    );
                }
            }
        }
        Ok(())
    }

    fn cleanup_chain(&mut self, chain: &[h::CleanupStep], span: Span) -> Result<(), LowerError> {
        for step in chain {
            let block = self.block(match step.action {
                h::ScopeCleanup::ExplicitDispose { .. } => "cleanup.dispose",
                h::ScopeCleanup::FinallyRegion { .. } => "cleanup.finally",
            });
            self.goto(block, span);
            self.current = Some(block);
            match step.action {
                h::ScopeCleanup::ExplicitDispose { resource, method } => {
                    let disposer = &self.body.disposers[method.0];
                    let receiver = self.local_place(resource);
                    // Dispose is a resolved method call, not native Drop.
                    self.emit(
                        m::InstructionKind::Call {
                            target: m::CallTarget::Dispose(method),
                            arguments: vec![m::CallArgument::Place {
                                mode: h::ArgumentMode::BorrowMutable,
                                place: receiver,
                            }],
                            parameter_types: vec![disposer.owner.clone()],
                            parameter_modes: vec![h::ArgumentMode::BorrowMutable],
                            return_type: None,
                        },
                        None,
                        span,
                    );
                }
                h::ScopeCleanup::FinallyRegion { finally_scope } => {
                    let body = *self
                        .finally_bodies
                        .get(&finally_scope)
                        .ok_or_else(|| LowerError::InvalidHir("Finally body is missing".into()))?;
                    self.lower_statements(body)?;
                    if self.current.is_none() {
                        return Err(LowerError::Unsupported {
                            feature: "control transfer from Finally",
                            span,
                        });
                    }
                    self.exit_cleanup(&[finally_scope], &[], span)?;
                }
            }
        }
        Ok(())
    }

    fn local_place(&self, id: h::LocalId) -> m::Place {
        m::Place {
            root: m::LocalId(id.0),
            projections: Vec::new(),
            ty: self.function.locals[id.0].ty.clone(),
        }
    }

    fn place_expr(&mut self, expr: &h::Expression) -> Result<m::Place, LowerError> {
        let h::ExpressionKind::Place(place) = &expr.kind else {
            return Err(LowerError::InvalidHir("expected a resolved Place".into()));
        };
        let mut projections = Vec::new();
        for projection in &place.projections {
            projections.push(match projection {
                h::Projection::Field(id) => m::Projection::Field(*id),
                h::Projection::TupleField(index) => m::Projection::TupleField(*index),
                h::Projection::Index(index) => m::Projection::Index(self.expr(&index.index)?),
            });
        }
        Ok(m::Place {
            root: m::LocalId(place.root.0),
            projections,
            ty: expr.ty.clone(),
        })
    }

    fn store(&mut self, place: m::Place, value: m::TempId, span: Span) {
        self.emit(m::InstructionKind::Store { place, value }, None, span);
    }

    fn integer(&mut self, value: i64, ty: &TypeName, span: Span) -> m::TempId {
        self.value(
            m::InstructionKind::Const(m::Constant::Integer(value)),
            ty.clone(),
            span,
        )
    }

    fn expr(&mut self, expr: &h::Expression) -> Result<m::TempId, LowerError> {
        let kind = match &expr.kind {
            h::ExpressionKind::Constant(value) => m::InstructionKind::Const(match value {
                h::Constant::ZeroAggregate => m::Constant::ZeroAggregate,
                h::Constant::Integer(value) => m::Constant::Integer(*value),
                h::Constant::Single(value) => m::Constant::Single(*value),
                h::Constant::Double(value) => m::Constant::Double(*value),
                h::Constant::Boolean(value) => m::Constant::Boolean(*value),
                h::Constant::String(value) => m::Constant::String(value.clone()),
            }),
            h::ExpressionKind::StringConcat { left, right } => m::InstructionKind::StringConcat {
                left: self.expr(left)?,
                right: self.expr(right)?,
            },
            h::ExpressionKind::StringCompare {
                operation,
                left,
                right,
                text,
            } => m::InstructionKind::StringCompare {
                op: *operation,
                left: self.expr(left)?,
                right: self.expr(right)?,
                text: *text,
            },
            h::ExpressionKind::StringLen(value) => m::InstructionKind::StringLen(self.expr(value)?),
            h::ExpressionKind::StringFormat { value, decimals } => {
                m::InstructionKind::StringFormat {
                    value: self.expr(value)?,
                    decimals: *decimals,
                }
            }
            h::ExpressionKind::Tuple(values) => m::InstructionKind::TupleInit(
                values
                    .iter()
                    .map(|value| self.expr(value))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            h::ExpressionKind::ArrayInit { lower: 0, upper } => {
                m::InstructionKind::ArrayInit { upper: *upper }
            }
            h::ExpressionKind::ArrayInit { .. } => {
                return Err(LowerError::Unsupported {
                    feature: "non-zero-based arrays",
                    span: expr.span,
                });
            }
            h::ExpressionKind::Load(place) if expr.ty == TypeName::String => {
                m::InstructionKind::CloneString(self.place_expr(place)?)
            }
            h::ExpressionKind::Load(place) => m::InstructionKind::Load(self.place_expr(place)?),
            h::ExpressionKind::Convert { value, conversion } => m::InstructionKind::Cast {
                value: self.expr(value)?,
                conversion: *conversion,
            },
            h::ExpressionKind::Arithmetic {
                operation,
                left,
                right,
                ..
            } => m::InstructionKind::Arithmetic {
                op: *operation,
                left: self.expr(left)?,
                right: self.expr(right)?,
            },
            h::ExpressionKind::Compare {
                operation,
                left,
                right,
            } => m::InstructionKind::Compare {
                op: *operation,
                left: self.expr(left)?,
                right: self.expr(right)?,
            },
            h::ExpressionKind::Call {
                function,
                signature,
                arguments,
            } => {
                let lowered = self.call_arguments(arguments)?;
                m::InstructionKind::Call {
                    target: m::CallTarget::Function(*function),
                    arguments: lowered,
                    parameter_types: signature.parameter_types.clone(),
                    parameter_modes: signature.parameter_modes.clone(),
                    return_type: Some(signature.return_type.clone()),
                }
            }
            h::ExpressionKind::Place(_) => {
                return Err(LowerError::InvalidHir(
                    "Place used as a value without Load".into(),
                ));
            }
            h::ExpressionKind::BorrowMutable(_) | h::ExpressionKind::BorrowImmutable(_) => {
                return Err(LowerError::InvalidHir("Borrow used outside a call".into()));
            }
            h::ExpressionKind::Move(_) => {
                return Err(LowerError::Unsupported {
                    feature: "ownership moves",
                    span: expr.span,
                });
            }
        };
        Ok(self.value(kind, expr.ty.clone(), expr.span))
    }

    fn call_arguments(
        &mut self,
        arguments: &[h::CallArgument],
    ) -> Result<Vec<m::CallArgument>, LowerError> {
        let mut lowered = Vec::with_capacity(arguments.len());
        for argument in arguments {
            lowered.push(match argument.mode {
                h::ArgumentMode::ByVal => m::CallArgument::Value(self.expr(&argument.value)?),
                h::ArgumentMode::BorrowMutable | h::ArgumentMode::BorrowImmutable => {
                    let source = match &argument.value.kind {
                        h::ExpressionKind::BorrowMutable(place)
                        | h::ExpressionKind::BorrowImmutable(place) => place,
                        _ => {
                            return Err(LowerError::InvalidHir(
                                "ByRef call argument has no Place".into(),
                            ));
                        }
                    };
                    m::CallArgument::Place {
                        mode: argument.mode,
                        place: self.place_expr(source)?,
                    }
                }
                h::ArgumentMode::Move => {
                    return Err(LowerError::Unsupported {
                        feature: "ownership-consuming calls",
                        span: argument.value.span,
                    });
                }
            });
        }
        Ok(lowered)
    }
}

fn collect_finally<'a>(
    statements: &'a [h::Statement],
    bodies: &mut HashMap<h::ScopeId, &'a [h::Statement]>,
) {
    for statement in statements {
        match statement {
            h::Statement::TryFinally {
                try_body,
                finally_scope,
                finally_body,
                ..
            } => {
                bodies.insert(*finally_scope, finally_body);
                collect_finally(try_body, bodies);
                collect_finally(finally_body, bodies);
            }
            h::Statement::TryCatch {
                try_body,
                catch_body,
                finally_scope,
                finally_body,
                ..
            } => {
                if let Some(scope) = finally_scope {
                    bodies.insert(*scope, finally_body);
                }
                collect_finally(try_body, bodies);
                collect_finally(catch_body, bodies);
                collect_finally(finally_body, bodies);
            }
            h::Statement::If {
                then_body,
                else_body,
                ..
            } => {
                collect_finally(then_body, bodies);
                collect_finally(else_body, bodies);
            }
            h::Statement::While { body, .. }
            | h::Statement::Do { body, .. }
            | h::Statement::For { body, .. }
            | h::Statement::ForEach { body, .. }
            | h::Statement::UsingDispose { body, .. } => collect_finally(body, bodies),
            _ => {}
        }
    }
}
