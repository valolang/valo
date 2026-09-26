//! Typed bodies for the initial native lowering subset.
//!
//! IDs are scoped to the input Program (functions) or body (locals), not to the
//! separate project declaration index. There are no runtime Value objects here.
use super::arithmetic::{ArithmeticOp, ArithmeticSignature};
use super::type_properties::{KnownProperty, TypeProperties};
use crate::frontend::type_model::TypeName;
use crate::runtime::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyFunctionId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoopId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DisposeMethodId(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedDispose {
    pub id: DisposeMethodId,
    pub owner: TypeName,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeCleanup {
    ExplicitDispose {
        resource: LocalId,
        method: DisposeMethodId,
    },
    /// Execute the already typed Finally body before leaving the Try scope.
    FinallyRegion { finally_scope: ScopeId },
}

/// One handler on a transfer path. The value of a Return is evaluated before
/// this chain begins; handlers run in this order before the final transfer.
#[derive(Debug, Clone, PartialEq)]
pub struct CleanupStep {
    pub owner: ScopeId,
    pub action: ScopeCleanup,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedField {
    pub id: FieldId,
    pub owner: TypeName,
    pub ty: TypeName,
    pub name: String,
}

/// An addressable location after name resolution. Projections are ordered from
/// the root outward; field and element lowering is not enabled until their
/// identities and bounds have been resolved by the frontend.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub root: LocalId,
    pub projections: Vec<Projection>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Projection {
    Field(FieldId),
    TupleField(usize),
    Index(IndexProjection),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexProjection {
    pub index: Box<Expression>,
    pub element_type: TypeName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexKey {
    Constant(i64),
    /// A resolved index whose inequality with another index is not proven.
    Unknown,
}

impl IndexProjection {
    pub fn key(&self) -> IndexKey {
        match &self.index.kind {
            ExpressionKind::Constant(Constant::Integer(value)) => IndexKey::Constant(*value),
            _ => IndexKey::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceOverlap {
    Disjoint,
    Overlap,
    Unknown,
}

impl Place {
    pub fn local(root: LocalId) -> Self {
        Self {
            root,
            projections: Vec::new(),
        }
    }

    /// `Unknown` must be treated as a conflict for mutable borrows.
    pub fn overlap(&self, other: &Self) -> PlaceOverlap {
        if self.root != other.root {
            return PlaceOverlap::Disjoint;
        }
        for (left, right) in self.projections.iter().zip(&other.projections) {
            match (left, right) {
                (Projection::Field(a), Projection::Field(b)) if a != b => {
                    return PlaceOverlap::Disjoint;
                }
                (Projection::TupleField(a), Projection::TupleField(b)) if a != b => {
                    return PlaceOverlap::Disjoint;
                }
                (Projection::Index(a), Projection::Index(b)) => match (a.key(), b.key()) {
                    (IndexKey::Constant(left), IndexKey::Constant(right)) if left != right => {
                        return PlaceOverlap::Disjoint;
                    }
                    (IndexKey::Unknown, _) | (_, IndexKey::Unknown) => {
                        return PlaceOverlap::Unknown;
                    }
                    _ => {}
                },
                (a, b) if a != b => return PlaceOverlap::Unknown,
                _ => {}
            }
        }
        PlaceOverlap::Overlap
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalStorage {
    Value,
    BorrowedMutable,
    BorrowedImmutable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitialState {
    /// Function arguments are available on entry.
    Initialized,
    /// A local becomes available at its explicit Initialize statement.
    Uninitialized,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueCategory {
    Value,
    Place,
    MutableReference,
    ImmutableReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentMode {
    ByVal,
    BorrowMutable,
    BorrowImmutable,
    Move,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallArgument {
    pub mode: ArgumentMode,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallSignature {
    pub parameter_types: Vec<TypeName>,
    pub parameter_modes: Vec<ArgumentMode>,
    pub return_type: TypeName,
}

/// Locals are recorded in declaration order. A future cleanup pass can visit
/// them in reverse order on each normal or non-local exit.
#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub locals: Vec<LocalId>,
    pub cleanup: Vec<ScopeCleanup>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Local {
    pub id: LocalId,
    pub name: String,
    pub ty: TypeName,
    pub properties: TypeProperties,
    pub parameter_index: Option<usize>,
    pub storage: LocalStorage,
    pub initial_state: InitialState,
    pub scope: ScopeId,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq)]
pub struct TypedBody {
    pub function: BodyFunctionId,
    pub name: String,
    /// Owner-qualified semantic callable identity, including parameter shape.
    pub symbol_name: String,
    pub return_type: TypeName,
    pub locals: Vec<Local>,
    /// Declared value types, including empty Structures, in source declaration order.
    pub structures: Vec<TypeName>,
    /// Native Class identities; fields share the resolved FieldId table.
    pub classes: Vec<TypeName>,
    pub fields: Vec<ResolvedField>,
    pub disposers: Vec<ResolvedDispose>,
    pub scopes: Vec<Scope>,
    pub root_scope: ScopeId,
    pub statements: Vec<Statement>,
    pub span: Span,
}

impl TypedBody {
    pub fn cleanup_chain(&self, exited_scopes: &[ScopeId]) -> Vec<CleanupStep> {
        exited_scopes
            .iter()
            .flat_map(|scope| {
                self.scopes[scope.0]
                    .cleanup
                    .iter()
                    .rev()
                    .cloned()
                    .map(|action| CleanupStep {
                        owner: *scope,
                        action,
                    })
            })
            .collect()
    }
    /// Owned locals encountered on a control-flow edge, innermost scope and
    /// newest declaration first. These are candidates, not unconditional drops:
    /// ownership state and type-level drop requirements are resolved later.
    pub fn owned_exit_locals(&self, exited_scopes: &[ScopeId]) -> Vec<LocalId> {
        exited_scopes
            .iter()
            .flat_map(|scope| self.scopes[scope.0].locals.iter().rev())
            .copied()
            .filter(|local| self.locals[local.0].storage == LocalStorage::Value)
            .collect()
    }

    /// The type-level part of each exit obligation. `Unknown` blocks drop
    /// emission until native resource semantics and path ownership are known.
    pub fn exit_drop_requirements(
        &self,
        exited_scopes: &[ScopeId],
    ) -> Vec<(LocalId, KnownProperty)> {
        self.owned_exit_locals(exited_scopes)
            .into_iter()
            .map(|local| (local, self.locals[local.0].properties.requires_drop))
            .collect()
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Initialize {
        target: LocalId,
        value: Expression,
        span: Span,
    },
    Store {
        target: Box<Expression>,
        value: Expression,
        span: Span,
    },
    Return {
        value: Expression,
        exited_scopes: Vec<ScopeId>,
        cleanup_chain: Vec<CleanupStep>,
        span: Span,
    },
    ReturnVoid {
        exited_scopes: Vec<ScopeId>,
        cleanup_chain: Vec<CleanupStep>,
        span: Span,
    },
    CallSub {
        function: BodyFunctionId,
        signature: CallSignature,
        arguments: Vec<CallArgument>,
        span: Span,
    },
    CollectionAdd {
        collection: Expression,
        item: Expression,
        before: Box<Option<Expression>>,
        span: Span,
    },
    CollectionRemove {
        collection: Expression,
        index: Expression,
        span: Span,
    },
    If {
        condition: Expression,
        then_scope: ScopeId,
        then_body: Vec<Statement>,
        else_scope: ScopeId,
        else_body: Vec<Statement>,
        span: Span,
    },
    /// Structured cleanup region. Transfers out of Try carry a Finally
    /// handler; exception dispatch/unwind edges remain future work.
    TryFinally {
        try_scope: ScopeId,
        try_body: Vec<Statement>,
        finally_scope: ScopeId,
        finally_body: Vec<Statement>,
        span: Span,
    },
    /// One source Catch clause (the currently parsed surface), optionally
    /// followed by Finally. Catch dispatch and exception edges remain abstract.
    TryCatch {
        try_scope: ScopeId,
        try_body: Vec<Statement>,
        catch_scope: ScopeId,
        catch_local: Option<LocalId>,
        catch_body: Vec<Statement>,
        finally_scope: Option<ScopeId>,
        finally_body: Vec<Statement>,
        span: Span,
    },
    UsingDispose {
        resource: LocalId,
        method: DisposeMethodId,
        body_scope: ScopeId,
        body: Vec<Statement>,
        span: Span,
    },
    While {
        id: LoopId,
        condition: Expression,
        body_scope: ScopeId,
        body: Vec<Statement>,
        span: Span,
    },
    Do {
        id: LoopId,
        condition: DoCondition,
        body_scope: ScopeId,
        body: Vec<Statement>,
        span: Span,
    },
    For {
        id: LoopId,
        variable: LocalId,
        start: Box<Expression>,
        end: Box<Expression>,
        step: Box<Expression>,
        body_scope: ScopeId,
        body: Vec<Statement>,
        span: Span,
    },
    ForEach {
        id: LoopId,
        variable: LocalId,
        iterable: Expression,
        element_type: TypeName,
        body_scope: ScopeId,
        body: Vec<Statement>,
        span: Span,
    },
    ExitLoop {
        loop_id: LoopId,
        exited_scopes: Vec<ScopeId>,
        cleanup_chain: Vec<CleanupStep>,
        span: Span,
    },
    ContinueLoop {
        loop_id: LoopId,
        exited_scopes: Vec<ScopeId>,
        cleanup_chain: Vec<CleanupStep>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DoCondition {
    Infinite,
    PreWhile(Expression),
    PreUntil(Expression),
    PostWhile(Expression),
    PostUntil(Expression),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub ty: TypeName,
    pub category: ValueCategory,
    pub span: Span,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conversion {
    /// Ordinary assignment/argument conversion, including checked narrowing.
    NumericChecked,
    /// Integer division truncates floating operands before division.
    TruncateToInteger,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    /// Default value of a plain value aggregate; its type is carried by Expression.
    ZeroAggregate,
    Integer(i64),
    Single(f32),
    Double(f64),
    Boolean(bool),
    String(String),
    NullReference,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
}

impl ComparisonOp {
    pub fn from_ast(op: crate::BinaryOp) -> Option<Self> {
        Some(match op {
            crate::BinaryOp::Equal => Self::Equal,
            crate::BinaryOp::NotEqual => Self::NotEqual,
            crate::BinaryOp::Less => Self::Less,
            crate::BinaryOp::Greater => Self::Greater,
            crate::BinaryOp::LessEqual => Self::LessEqual,
            crate::BinaryOp::GreaterEqual => Self::GreaterEqual,
            _ => return None,
        })
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionKind {
    Constant(Constant),
    StringConcat {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    StringCompare {
        operation: ComparisonOp,
        left: Box<Expression>,
        right: Box<Expression>,
        text: bool,
    },
    /// Object identity; consumes the two owned reference temporaries.
    ReferenceIdentity {
        left: Box<Expression>,
        right: Box<Expression>,
        negated: bool,
    },
    StringLen(Box<Expression>),
    /// Render a primitive interpolation hole; `decimals` is a resolved fixed
    /// decimal format (e.g. `0.0`), not source format syntax for the backend.
    StringFormat {
        value: Box<Expression>,
        decimals: Option<u8>,
    },
    Tuple(Vec<Expression>),
    /// The source upper bound is inclusive. Only one-dimensional fixed arrays
    /// enter the current native HIR subset.
    ArrayInit {
        lower: i64,
        upper: i64,
    },
    /// Allocate a resolved native Class. Constructor dispatch is represented
    /// separately and is not inferred by LLVM.
    NewClass(TypeName),
    NewCollection,
    BoxDynamic(Box<Expression>),
    UnboxDynamic {
        value: Box<Expression>,
        target: TypeName,
    },
    CollectionCount(Box<Expression>),
    CollectionItem {
        collection: Box<Expression>,
        index: Box<Expression>,
    },
    Place(Place),
    Load(Box<Expression>),
    BorrowMutable(Box<Expression>),
    BorrowImmutable(Box<Expression>),
    /// Ownership transfer. No source construct lowers to this yet.
    Move(Box<Expression>),
    Convert {
        value: Box<Expression>,
        conversion: Conversion,
    },
    Arithmetic {
        operation: ArithmeticOp,
        signature: ArithmeticSignature,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Compare {
        operation: ComparisonOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Call {
        function: BodyFunctionId,
        signature: CallSignature,
        arguments: Vec<CallArgument>,
    },
}
