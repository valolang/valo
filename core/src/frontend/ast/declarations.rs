use crate::runtime::{Span, TypeName};

use super::Stmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub attributes: Vec<AttributeDecl>,
    pub imports: Vec<ImportDecl>,
    pub option_explicit: bool,
    pub option_base: i64,
    pub option_compare: OptionCompare,
    pub types: Vec<TypeDecl>,
    pub enums: Vec<EnumDecl>,
    pub module_vars: Vec<ModuleVarDecl>,
    pub module_consts: Vec<ConstDecl>,
    pub declares: Vec<DeclareDecl>,
    pub interfaces: Vec<InterfaceDecl>,
    pub classes: Vec<ClassDecl>,
    pub procedures: Vec<Procedure>,
    pub functions: Vec<Function>,
    pub properties: Vec<ClassProperty>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributeDecl {
    pub target: String,
    pub name: String,
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub module: String,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptionCompare {
    #[default]
    Binary,
    Text,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleVarDecl {
    pub visibility: Visibility,
    pub name: String,
    pub ty: Option<TypeName>,
    pub array: Option<super::ArrayDecl>,
    pub initializer: Option<super::Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub visibility: Visibility,
    pub name: String,
    pub ty: Option<TypeName>,
    pub value: super::Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclareDecl {
    pub visibility: Visibility,
    pub ptr_safe: bool,
    pub calling_convention: CallingConvention,
    pub kind: DeclareKind,
    pub name: String,
    pub lib: String,
    pub alias: Option<String>,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeName>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclareKind {
    Function,
    Sub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallingConvention {
    Default,
    CDecl,
    StdCall,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub visibility: Visibility,
    pub name: String,
    pub members: Vec<EnumMemberDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumMemberDecl {
    pub name: String,
    pub value: Option<super::Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    pub visibility: Visibility,
    pub kind: TypeKind,
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Type,
    Structure,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub visibility: Visibility,
    pub name: String,
    pub ty: TypeName,
    pub array: Option<super::ArrayDecl>,
    pub initializer: Option<super::Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub visibility: Visibility,
    pub name: String,
    pub implements: Vec<TypeName>,
    pub attributes: Vec<AttributeDecl>,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub visibility: Visibility,
    pub name: String,
    pub members: Vec<InterfaceMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceMember {
    Sub(InterfaceMethod),
    Function(InterfaceMethod),
    Property(InterfaceProperty),
    Event(InterfaceEvent),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceMethod {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeName>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceProperty {
    pub name: String,
    pub kind: PropertyKind,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeName>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceEvent {
    pub name: String,
    pub params: Vec<Parameter>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Field(ClassField),
    Fields(Vec<ClassField>),
    Const(ConstDecl),
    Event(ClassEvent),
    Sub(ClassSub),
    Function(ClassFunction),
    Iterator(ClassIterator),
    Property(ClassProperty),
    Type(TypeDecl),
    Declare(DeclareDecl),
    Enum(EnumDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassField {
    pub visibility: Visibility,
    pub is_shared: bool,
    pub with_events: bool,
    pub name: String,
    pub ty: Option<TypeName>,
    pub array: Option<super::ArrayDecl>,
    pub initializer: Option<super::Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassEvent {
    pub visibility: Visibility,
    pub is_shared: bool,
    pub name: String,
    pub params: Vec<Parameter>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassSub {
    pub visibility: Visibility,
    pub is_shared: bool,
    pub implements: Vec<ImplementsClause>,
    pub procedure: Procedure,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassFunction {
    pub visibility: Visibility,
    pub is_shared: bool,
    pub implements: Vec<ImplementsClause>,
    pub is_enumerator: bool,
    pub function: Function,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassIterator {
    pub visibility: Visibility,
    pub function: Function,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassProperty {
    pub visibility: Visibility,
    pub is_shared: bool,
    pub implements: Vec<ImplementsClause>,
    pub is_default: bool,
    pub is_enumerator: bool,
    pub is_iterator: bool,
    pub name: String,
    pub kind: PropertyKind,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeName>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplementsClause {
    pub interface_name: TypeName,
    pub member_name: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyKind {
    Get,
    Let,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Friend,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Procedure {
    pub visibility: Visibility,
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub visibility: Visibility,
    pub name: String,
    pub is_iterator: bool,
    pub params: Vec<Parameter>,
    pub return_type: TypeName,
    pub return_slot: Option<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub ty: TypeName,
    pub mode: PassingMode,
    pub is_optional: bool,
    pub optional_default: Option<super::Expr>,
    pub is_param_array: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassingMode {
    ByVal,
    ByRef,
}
