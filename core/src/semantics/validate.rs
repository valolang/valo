use std::collections::HashMap;

use crate::runtime::{Diagnostic, TypeName};
use crate::{
    AssignTarget, BinaryOp, CaseCompareOp, CaseItem, ClassMember, DoLoopCondition, ExitTarget,
    Expr, ExprKind, Function, OnErrorMode, Parameter, PassingMode, Procedure, Program,
    PropertyKind, ResumeTarget, Stmt, UnaryOp, Visibility,
};

use crate::semantics::context::Context;
use crate::semantics::symbols::{CallableSig, ParamSig, Signatures, VarType, key};
use crate::semantics::types::{
    ClassEventSig, ClassFieldSig, ClassMethodSig, ClassPropertySig, ClassSig, EnumSig, FieldSig,
    PropertyAccessorSig, TypeRegistry, TypeSig,
};

#[path = "validate_classes.rs"]
mod validate_classes;
#[path = "validate_declarations.rs"]
mod validate_declarations;
#[path = "validate_expressions.rs"]
mod validate_expressions;
#[path = "validate_statements.rs"]
mod validate_statements;

use validate_classes::validate_class;
use validate_declarations::{
    add_module_symbols, add_parameters, collect_module_symbols, collect_signatures, collect_types,
    ensure_const_expr, validate_function, validate_procedure,
};
use validate_expressions::*;
use validate_statements::validate_statements;

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
    let module_symbols = collect_module_symbols(program, &types, &signatures)?;
    let Some(main) = program
        .procedures
        .iter()
        .find(|procedure| procedure.name.eq_ignore_ascii_case("main"))
    else {
        return Err(Diagnostic::new(crate::runtime::DiagnosticCode::GENERIC, "Program must contain Sub Main()", None));
    };

    if !main.params.is_empty() {
        return Err(Diagnostic::new(crate::runtime::DiagnosticCode::GENERIC, "Sub Main() cannot have parameters", Some(main.span),));
    }

    for procedure in &program.procedures {
        validate_procedure(procedure, &types, &signatures, &module_symbols)?;
    }

    for function in &program.functions {
        validate_function(function, &types, &signatures, &module_symbols)?;
    }
    for class_decl in &program.classes {
        validate_class(class_decl, &types, &signatures, &module_symbols)?;
    }

    Ok(())
}
