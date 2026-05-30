use crate::runtime::{Span, TypeName};

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    String(String),
    Integer(i64),
    Long(i32),
    LongLong(i64),
    Single(f32),
    Double(f64),
    Currency(i64),
    Decimal(i128),
    Boolean(bool),
    DateLiteral(String),
    Nothing,
    Empty,
    Null,
    Me,
    MyBase,
    MyClass,
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
        class_name: TypeName,
        args: Vec<Expr>,
    },
    Call {
        name: String,
        type_args: Vec<TypeName>,
        args: Vec<Expr>,
    },
    Index {
        target: Box<Expr>,
        args: Vec<Expr>,
    },
    IIf {
        condition: Box<Expr>,
        true_expr: Box<Expr>,
        false_expr: Box<Expr>,
    },
    MemberAccess {
        object: Box<Expr>,
        field: String,
    },
    MemberCall {
        object: Box<Expr>,
        method: String,
        type_args: Vec<TypeName>,
        args: Vec<Expr>,
    },
    Lambda {
        params: Vec<crate::frontend::ast::declarations::Parameter>,
        body: Box<Expr>,
    },
    Await(Box<Expr>),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    AddressOf(Box<Expr>),
    PassingModeOverride {
        mode: crate::frontend::ast::PassingMode,
        expr: Box<Expr>,
    },
}

impl Expr {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        Expr {
            kind: self.kind.substitute_generics(bindings),
            span: self.span,
        }
    }
}

impl ExprKind {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        match self {
            ExprKind::NamedArg { name, expr } => ExprKind::NamedArg {
                name: name.clone(),
                expr: Box::new(expr.substitute_generics(bindings)),
            },
            ExprKind::TypeOfIs { expr, class_name } => {
                let ty = TypeName::User(class_name.clone()).substitute_generics(bindings);
                ExprKind::TypeOfIs {
                    expr: Box::new(expr.substitute_generics(bindings)),
                    class_name: ty.display_name(),
                }
            }
            ExprKind::New { class_name, args } => ExprKind::New {
                class_name: class_name.substitute_generics(bindings),
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
            },
            ExprKind::Call {
                name,
                type_args,
                args,
            } => ExprKind::Call {
                name: name.clone(),
                type_args: type_args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
            },
            ExprKind::Index { target, args } => ExprKind::Index {
                target: Box::new(target.substitute_generics(bindings)),
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
            },
            ExprKind::IIf {
                condition,
                true_expr,
                false_expr,
            } => ExprKind::IIf {
                condition: Box::new(condition.substitute_generics(bindings)),
                true_expr: Box::new(true_expr.substitute_generics(bindings)),
                false_expr: Box::new(false_expr.substitute_generics(bindings)),
            },
            ExprKind::MemberAccess { object, field } => ExprKind::MemberAccess {
                object: Box::new(object.substitute_generics(bindings)),
                field: field.clone(),
            },
            ExprKind::MemberCall {
                object,
                method,
                type_args,
                args,
            } => ExprKind::MemberCall {
                object: Box::new(object.substitute_generics(bindings)),
                method: method.clone(),
                type_args: type_args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
            },
            ExprKind::Lambda { params, body } => ExprKind::Lambda {
                params: params.clone(),
                body: Box::new(body.substitute_generics(bindings)),
            },
            ExprKind::Await(expr) => ExprKind::Await(Box::new(expr.substitute_generics(bindings))),
            ExprKind::Binary { left, op, right } => ExprKind::Binary {
                left: Box::new(left.substitute_generics(bindings)),
                op: *op,
                right: Box::new(right.substitute_generics(bindings)),
            },
            ExprKind::Unary { op, expr } => ExprKind::Unary {
                op: *op,
                expr: Box::new(expr.substitute_generics(bindings)),
            },
            ExprKind::AddressOf(expr) => {
                ExprKind::AddressOf(Box::new(expr.substitute_generics(bindings)))
            }
            ExprKind::PassingModeOverride { mode, expr } => ExprKind::PassingModeOverride {
                mode: *mode,
                expr: Box::new(expr.substitute_generics(bindings)),
            },
            _ => self.clone(),
        }
    }
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
    LogicalXor,
    LogicalEqv,
    LogicalImp,
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
    Positive,
    Negate,
    LogicalNot,
}
