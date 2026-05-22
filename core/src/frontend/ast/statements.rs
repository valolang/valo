use crate::runtime::{Span, TypeName};

use super::Expr;

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Dim {
        name: String,
        ty: Option<TypeName>,
        array: Option<ArrayDecl>,
        as_new: bool,
        new_args: Vec<Expr>,
        initializer: Option<Expr>,
        span: Span,
    },
    DimMany {
        decls: Vec<VariableDecl>,
        span: Span,
    },
    Static {
        name: String,
        ty: Option<TypeName>,
        array: Option<ArrayDecl>,
        as_new: bool,
        new_args: Vec<Expr>,
        initializer: Option<Expr>,
        span: Span,
    },
    StaticMany {
        decls: Vec<VariableDecl>,
        span: Span,
    },
    Const {
        name: String,
        ty: Option<TypeName>,
        value: Expr,
        span: Span,
    },
    ConstMany {
        consts: Vec<crate::ConstDecl>,
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
    RaiseEvent {
        name: String,
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
        target: ReDimTarget,
        dims: Vec<(Option<Expr>, Expr)>,
        preserve: bool,
        span: Span,
    },
    Erase {
        target: ReDimTarget,
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
    Resume {
        target: ResumeTarget,
        span: Span,
    },
    With {
        target: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Using {
        resource: UsingResource,
        body: Vec<Stmt>,
        span: Span,
    },
    Exit {
        target: ExitTarget,
        span: Span,
    },
    TryCatch {
        try_body: Vec<Stmt>,
        catch_block: Option<CatchBlock>,
        finally_body: Option<Vec<Stmt>>,
        span: Span,
    },
    DebugPrint {
        args: Vec<Expr>,
        span: Span,
    },
    Yield {
        expr: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchBlock {
    pub variable: Option<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnErrorMode {
    ResumeNext,
    GoToZero,
    GoToMinusOne,
    GoToLabel(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeTarget {
    Retry,
    Next,
    Label(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayDecl {
    Fixed(Vec<crate::runtime::ArrayBound>),
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
        indices: Vec<Expr>,
        span: Span,
    },
    Member {
        object: Expr,
        field: String,
        span: Span,
    },
    MemberArrayElement {
        object: Expr,
        field: String,
        indices: Vec<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDecl {
    pub name: String,
    pub ty: Option<TypeName>,
    pub array: Option<ArrayDecl>,
    pub as_new: bool,
    pub new_args: Vec<Expr>,
    pub initializer: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UsingResource {
    Declaration(VariableDecl),
    Target(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReDimTarget {
    Variable {
        name: String,
        span: Span,
    },
    Member {
        object: Expr,
        field: String,
        span: Span,
    },
}

impl ReDimTarget {
    pub fn name(&self) -> &str {
        match self {
            ReDimTarget::Variable { name, .. } => name,
            ReDimTarget::Member { field, .. } => field,
        }
    }
}

impl AssignTarget {
    pub fn span(&self) -> Span {
        match self {
            AssignTarget::Variable { span, .. }
            | AssignTarget::ArrayElement { span, .. }
            | AssignTarget::Member { span, .. }
            | AssignTarget::MemberArrayElement { span, .. } => *span,
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
