use crate::runtime::{Span, TypeName};

use super::Expr;

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Dim {
        name: String,
        ty: TypeName,
        array: Option<ArrayDecl>,
        span: Span,
    },
    Const {
        name: String,
        ty: Option<TypeName>,
        value: Expr,
        span: Span,
    },
    Assign {
        target: AssignTarget,
        expr: Expr,
        span: Span,
    },
    SetAssign {
        target: AssignTarget,
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
        next_variable: Option<(String, Span)>,
        body: Vec<Stmt>,
        span: Span,
    },
    ForEach {
        variable: String,
        iterable: Expr,
        next_variable: Option<(String, Span)>,
        body: Vec<Stmt>,
        span: Span,
    },
    ReDim {
        name: String,
        upper_bound: Expr,
        preserve: bool,
        span: Span,
    },
    Label {
        name: String,
        span: Span,
    },
    GoTo {
        label: String,
        span: Span,
    },
    OnError {
        mode: OnErrorMode,
        span: Span,
    },
    With {
        target: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Exit {
        target: ExitTarget,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnErrorMode {
    ResumeNext,
    GoToZero,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayDecl {
    Fixed(i64),
    Dynamic,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Variable {
        name: String,
        span: Span,
    },
    ArrayElement {
        name: String,
        index: Expr,
        span: Span,
    },
    Member {
        object: Expr,
        field: String,
        span: Span,
    },
}

impl AssignTarget {
    pub fn span(&self) -> Span {
        match self {
            AssignTarget::Variable { span, .. }
            | AssignTarget::ArrayElement { span, .. }
            | AssignTarget::Member { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElseIfBranch {
    pub condition: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseBranch {
    pub items: Vec<CaseItem>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaseItem {
    Value(Expr),
    Range { start: Expr, end: Expr },
    Compare { op: CaseCompareOp, expr: Expr },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseCompareOp {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
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
