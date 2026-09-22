use crate::frontend::semantics::arithmetic::ArithmeticOp;
use crate::frontend::semantics::typed_hir::{
    ArgumentMode, BodyFunctionId, ComparisonOp, Conversion, DisposeMethodId, FieldId,
};
use crate::frontend::type_model::TypeName;
use crate::runtime::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempId(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub id: BodyFunctionId,
    pub name: String,
    pub return_type: TypeName,
    pub locals: Vec<Local>,
    pub fields: Vec<Field>,
    /// Resolved Dispose owners indexed by DisposeMethodId.
    pub disposers: Vec<TypeName>,
    pub temps: Vec<TypeName>,
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub id: FieldId,
    pub owner: TypeName,
    pub ty: TypeName,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Local {
    pub id: LocalId,
    pub ty: TypeName,
    pub parameter_index: Option<usize>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: BlockId,
    pub label: Option<String>,
    pub instructions: Vec<Instruction>,
    pub terminator: Option<Terminator>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub result: Option<TempId>,
    pub kind: InstructionKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Integer(i64),
    Single(f32),
    Double(f64),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub root: LocalId,
    pub projections: Vec<Projection>,
    pub ty: TypeName,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Projection {
    Field(FieldId),
    TupleField(usize),
    Index(TempId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallTarget {
    Function(BodyFunctionId),
    Dispose(DisposeMethodId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallArgument {
    Value(TempId),
    Place { mode: ArgumentMode, place: Place },
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionKind {
    Const(Constant),
    /// Fixed, zero-based array allocation. Bounds checks remain required.
    ArrayInit {
        upper: i64,
    },
    ArrayLen(Place),
    Load(Place),
    Store {
        place: Place,
        value: TempId,
    },
    Arithmetic {
        op: ArithmeticOp,
        left: TempId,
        right: TempId,
    },
    Compare {
        op: ComparisonOp,
        left: TempId,
        right: TempId,
    },
    Cast {
        value: TempId,
        conversion: Conversion,
    },
    Call {
        target: CallTarget,
        arguments: Vec<CallArgument>,
        parameter_types: Vec<TypeName>,
        parameter_modes: Vec<ArgumentMode>,
        return_type: Option<TypeName>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TerminatorKind {
    Goto(BlockId),
    Branch {
        condition: TempId,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return(TempId),
    Trap(&'static str),
    Unreachable,
}
