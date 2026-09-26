use crate::frontend::semantics::arithmetic::ArithmeticOp;
use crate::frontend::semantics::type_properties::TypeProperties;
use crate::frontend::semantics::typed_hir::{
    ArgumentMode, BodyFunctionId, ComparisonOp, Conversion, DisposeMethodId, FieldId, LocalStorage,
};
use crate::frontend::type_model::TypeName;
use crate::runtime::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BorrowId(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub functions: Vec<Function>,
    /// Resolved by the semantic compilation before backend lowering.
    pub entry: Option<BodyFunctionId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub id: BodyFunctionId,
    pub name: String,
    pub symbol_name: String,
    pub return_type: TypeName,
    pub locals: Vec<Local>,
    pub structures: Vec<TypeName>,
    pub classes: Vec<TypeName>,
    pub fields: Vec<Field>,
    /// Resolved Dispose owners indexed by DisposeMethodId.
    pub disposers: Vec<TypeName>,
    pub temps: Vec<TypeName>,
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
    pub span: Span,
}

impl Function {
    /// Whether a value owns managed fields and therefore needs semantic clone
    /// and destruction rather than a bitwise aggregate copy. Unknown user
    /// types remain the native eligibility checker's responsibility.
    pub fn has_managed_fields(&self, ty: &TypeName) -> bool {
        self.has_managed_fields_inner(ty, &mut Vec::new())
    }

    fn has_managed_fields_inner(&self, ty: &TypeName, visiting: &mut Vec<usize>) -> bool {
        match ty {
            TypeName::String | TypeName::Variant => true,
            TypeName::Tuple(elements) => elements
                .iter()
                .any(|element| self.has_managed_fields_inner(&element.ty, visiting)),
            TypeName::Array(element) => self.has_managed_fields_inner(element, visiting),
            TypeName::User(_) => {
                if matches!(ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
                {
                    return true;
                }
                if self.classes.iter().any(|item| item.same_type(ty)) {
                    return true;
                }
                let Some(id) = self.structures.iter().position(|item| item.same_type(ty)) else {
                    return false;
                };
                if visiting.contains(&id) {
                    return false;
                }
                visiting.push(id);
                let result = self
                    .fields
                    .iter()
                    .filter(|field| field.owner.same_type(ty))
                    .any(|field| self.has_managed_fields_inner(&field.ty, visiting));
                visiting.pop();
                result
            }
            _ => false,
        }
    }
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
    pub properties: TypeProperties,
    pub storage: LocalStorage,
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
    ZeroAggregate,
    Integer(i64),
    Single(f32),
    Double(f64),
    Boolean(bool),
    String(String),
    NullReference,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionKind {
    Const(Constant),
    TupleInit(Vec<TempId>),
    /// Fixed, zero-based array allocation. Bounds checks remain required.
    ArrayInit {
        upper: i64,
    },
    NewClass(TypeName),
    NewCollection,
    /// Owned entry snapshot of an ordered Collection. Consumes the source share.
    SnapshotCollection(TempId),
    BoxDynamic {
        value: TempId,
        ty: TypeName,
    },
    UnboxDynamic {
        value: TempId,
        ty: TypeName,
    },
    CollectionCount(TempId),
    CollectionItem {
        collection: TempId,
        index: TempId,
    },
    CollectionAdd {
        collection: TempId,
        item: TempId,
        before: Option<TempId>,
    },
    CollectionRemove {
        collection: TempId,
        index: TempId,
    },
    ArrayLen(Place),
    /// Copy the fixed array's element sequence once at For Each entry.
    SnapshotArray(Place),
    Load(Place),
    /// Retain a managed, immutable String reference from an addressable Place.
    CloneString(Place),
    /// Semantic copy of a managed aggregate Place. Each managed field acquires
    /// its own ownership share before the resulting value is transferred.
    CloneManaged(Place),
    /// Consume two owned String temporaries and produce one owned String.
    StringConcat {
        left: TempId,
        right: TempId,
    },
    /// Consume two owned String temporaries and produce a Boolean.
    StringCompare {
        op: ComparisonOp,
        left: TempId,
        right: TempId,
        text: bool,
    },
    ReferenceIdentity {
        left: TempId,
        right: TempId,
        negated: bool,
    },
    /// Consume an owned String temporary and produce its scalar length.
    StringLen(TempId),
    StringFormat {
        value: TempId,
        decimals: Option<u8>,
    },
    /// Internal ownership transfer; source syntax remains gated.
    Move(Place),
    /// Internal deterministic destruction, distinct from Dispose calls.
    Drop(Place),
    /// HIR scope-exit obligation, expanded before MIR verification/dataflow.
    DropCandidate(LocalId),
    BorrowStart {
        id: BorrowId,
        kind: BorrowKind,
        place: Place,
    },
    EndBorrow(BorrowId),
    Store {
        place: Place,
        value: TempId,
    },
    /// RHS is already evaluated; release the previous owned value, then store.
    Replace {
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
    ReturnVoid,
    Trap(&'static str),
    Unreachable,
}
