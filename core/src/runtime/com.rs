//! COM/OLE Automation architecture foundation.
//!
//! COM is Windows-native. These types model ownership and import planning
//! without pretending that a full COM runtime exists on every platform.

use crate::runtime::TypeName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComImport {
    pub namespace: String,
    pub source: ComImportSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComImportSource {
    ProgId(String),
    TypeLibraryGuid(String),
    TypeLibraryPath(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComTypeLibrary {
    pub namespace: String,
    pub types: Vec<ComTypeInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComTypeInfo {
    pub name: String,
    pub kind: ComTypeKind,
    pub members: Vec<ComMemberInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComTypeKind {
    DispatchInterface,
    VTableInterface,
    CoClass,
    Enum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComMemberInfo {
    pub name: String,
    pub dispatch_id: i32,
    pub invoke_kind: ComInvokeKind,
    pub params: Vec<ComParamInfo>,
    pub return_type: TypeName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComInvokeKind {
    Method,
    PropertyGet,
    PropertyPut,
    PropertySet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComParamInfo {
    pub name: String,
    pub ty: TypeName,
    pub optional: bool,
    pub by_ref: bool,
}
