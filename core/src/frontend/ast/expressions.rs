use crate::runtime::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    String(String),
    Integer(i64),
    Double(f64),
    Boolean(bool),
    Nothing,
    Empty,
    Null,
    Me,
    WithTarget,
    Missing,
    Variable(String),
    NamedArg {
        name: String,
        expr: Box<Expr>,
    },
    TypeOfIs {
        expr: Box<Expr>,
        class_name: String,
    },
    New {
        class_name: String,
        args: Vec<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    MemberAccess {
        object: Box<Expr>,
        field: String,
    },
    MemberCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Exponent,
    Divide,
    IntegerDivide,
    Modulo,
    Concat,
    LogicalAnd,
    LogicalOr,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Is,
    Like,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    LogicalNot,
}
