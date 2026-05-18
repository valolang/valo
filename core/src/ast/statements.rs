use crate::runtime::{Span, TypeName};

use super::Expr;

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Dim {
        name: String,
        ty: TypeName,
        array_size: Option<usize>,
        span: Span,
    },
    Assign {
        name: String,
        expr: Expr,
        span: Span,
    },
    SetAssign {
        name: String,
        expr: Expr,
        span: Span,
    },
    ArrayAssign {
        name: String,
        index: Expr,
        expr: Expr,
        span: Span,
    },
    MemberAssign {
        target: Expr,
        field: String,
        expr: Expr,
        span: Span,
    },
    ConsoleWriteLine {
        args: Vec<Expr>,
        span: Span,
    },
    SubCall {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    MemberSubCall {
        object: Expr,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    Return {
        expr: Expr,
        span: Span,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        elseif_branches: Vec<ElseIfBranch>,
        else_body: Vec<Stmt>,
        span: Span,
    },
    SelectCase {
        subject: Expr,
        branches: Vec<CaseBranch>,
        else_body: Vec<Stmt>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    DoLoop {
        condition: DoLoopCondition,
        body: Vec<Stmt>,
        span: Span,
    },
    For {
        variable: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
        body: Vec<Stmt>,
        span: Span,
    },
    Exit {
        target: ExitTarget,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElseIfBranch {
    pub condition: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseBranch {
    pub values: Vec<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DoLoopCondition {
    Infinite,
    PreWhile(Expr),
    PreUntil(Expr),
    PostWhile(Expr),
    PostUntil(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitTarget {
    Sub,
    Function,
    For,
    While,
    Do,
}
