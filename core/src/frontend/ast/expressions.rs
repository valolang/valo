use crate::frontend::type_model::TypeName;
use crate::runtime::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// One step of a query, in the order it was written.
///
/// Steps apply in that order: a `Where` after an `Order By` filters what the
/// sort produced. `Select` may only come last, since after a projection the
/// range variable no longer holds what the query started from.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryClause {
    Where(Expr),
    OrderBy { key: Expr, descending: bool },
    Distinct,
    Take(Expr),
    Skip(Expr),
    Select(Expr),
}

/// One element of a tuple literal.
///
/// The name is optional and exists so the element can be read back as
/// `point.X` rather than `point.Item1`; it does not change the tuple's type.
#[derive(Debug, Clone, PartialEq)]
pub struct TupleElementExpr {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    String(String),
    /// An interpolated string, `$"total: {count:0.00}"`.
    Interpolated(Vec<InterpolationPart>),
    /// `CType(x, T)`, `DirectCast(x, T)`, or `TryCast(x, T)`.
    Convert {
        expr: Box<Expr>,
        target: TypeName,
        kind: ConversionKind,
    },
    /// `GetType(T)`, which yields the type's name.
    GetType(TypeName),
    /// `NameOf(x)`, which yields the source name of its operand.
    NameOf(String),
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
    /// `(1, 2)` or `(X := 1, Y := 2)`: a fixed group of values in one place.
    TupleLiteral(Vec<TupleElementExpr>),
    /// `From n In numbers Where n > 2 Select n * 2`.
    Query {
        variable: String,
        source: Box<Expr>,
        clauses: Vec<QueryClause>,
    },
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
        /// A `From { ... }` collection initializer.
        initializer: Option<Vec<Expr>>,
        /// A `With { .Member = value, ... }` object initializer.
        member_initializer: Option<Vec<MemberInit>>,
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
        /// True when this access is part of a `?.` chain, in which case a
        /// `Nothing` receiver yields `Nothing` rather than failing.
        conditional: bool,
    },
    MemberCall {
        object: Box<Expr>,
        method: String,
        type_args: Vec<TypeName>,
        args: Vec<Expr>,
        /// True when this call is part of a `?.` chain.
        conditional: bool,
    },
    Lambda {
        params: Vec<crate::frontend::ast::declarations::Parameter>,
        body: LambdaBody,
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
            ExprKind::New {
                class_name,
                args,
                initializer,
                member_initializer,
            } => ExprKind::New {
                class_name: class_name.substitute_generics(bindings),
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
                member_initializer: member_initializer.as_ref().map(|inits| {
                    inits
                        .iter()
                        .map(|init| MemberInit {
                            name: init.name.clone(),
                            value: init.value.substitute_generics(bindings),
                            span: init.span,
                        })
                        .collect()
                }),
                initializer: initializer.as_ref().map(|init| {
                    init.iter()
                        .map(|arg| arg.substitute_generics(bindings))
                        .collect()
                }),
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
            ExprKind::MemberAccess {
                object,
                field,
                conditional,
            } => ExprKind::MemberAccess {
                object: Box::new(object.substitute_generics(bindings)),
                field: field.clone(),
                conditional: *conditional,
            },
            ExprKind::MemberCall {
                object,
                method,
                type_args,
                args,
                conditional,
            } => ExprKind::MemberCall {
                object: Box::new(object.substitute_generics(bindings)),
                method: method.clone(),
                conditional: *conditional,
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
                body: body.substitute_generics(bindings),
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

/// How a conversion expression treats a value whose type does not match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionKind {
    /// `CType`: converts between compatible types, as the `C*` builtins do.
    Convert,
    /// `DirectCast`: reinterprets a reference, failing if the type is wrong.
    Direct,
    /// `TryCast`: like `DirectCast` but yields `Nothing` instead of failing.
    Try,
}

/// The body of a lambda.
///
/// A single-line lambda is an expression and yields its value directly. A
/// multi-line one is a statement block, and a `Function` lambda yields whatever
/// its `Return` produces while a `Sub` lambda yields nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum LambdaBody {
    Expression(Box<Expr>),
    Statements {
        body: Vec<crate::frontend::ast::statements::Stmt>,
        is_sub: bool,
    },
}

impl LambdaBody {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        match self {
            LambdaBody::Expression(expr) => {
                LambdaBody::Expression(Box::new(expr.substitute_generics(bindings)))
            }
            LambdaBody::Statements { body, is_sub } => LambdaBody::Statements {
                body: body
                    .iter()
                    .map(|stmt| stmt.substitute_generics(bindings))
                    .collect(),
                is_sub: *is_sub,
            },
        }
    }
}

/// One `.Member = value` entry in a `With { ... }` object initializer.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberInit {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

/// One piece of an interpolated string.
#[derive(Debug, Clone, PartialEq)]
pub enum InterpolationPart {
    Literal(String),
    /// A hole: the value to render, optionally padded to `alignment` (negative
    /// pads on the right) and rendered through `format`.
    Value {
        expr: Box<Expr>,
        alignment: Option<i32>,
        format: Option<String>,
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
    ShiftLeft,
    ShiftRight,
    LogicalAnd,
    LogicalAndAlso,
    LogicalOr,
    LogicalOrElse,
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
    IsNot,
    Like,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Positive,
    Negate,
    LogicalNot,
}
