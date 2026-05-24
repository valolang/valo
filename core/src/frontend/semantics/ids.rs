//! Stable semantic identities used by project-wide analysis.
//!
//! These IDs are deliberately small wrappers instead of strings. The current
//! interpreter still uses string-keyed maps in many places, but tooling, HIR,
//! package builds, and a future VM need durable identities that survive renames,
//! aliases, and import resolution work.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FunctionId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MemberId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolId {
    Module(ModuleId),
    Type(TypeId),
    Function(FunctionId),
    Member(MemberId),
}
