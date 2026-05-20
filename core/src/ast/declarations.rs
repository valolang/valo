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
    pub classes: Vec<ClassDecl>,
    pub procedures: Vec<Procedure>,
    pub functions: Vec<Function>,
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
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeName,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub visibility: Visibility,
    pub name: String,
    pub attributes: Vec<AttributeDecl>,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Field(ClassField),
    Fields(Vec<ClassField>),
    Event(ClassEvent),
    Sub(ClassSub),
    Function(ClassFunction),
    Iterator(ClassIterator),
    Property(ClassProperty),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassField {
    pub visibility: Visibility,
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
    pub name: String,
    pub params: Vec<Parameter>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassSub {
    pub visibility: Visibility,
    pub procedure: Procedure,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassFunction {
    pub visibility: Visibility,
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
    pub is_default: bool,
    pub is_enumerator: bool,
    pub name: String,
    pub kind: PropertyKind,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeName>,
    pub body: Vec<Stmt>,
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
    pub params: Vec<Parameter>,
    pub return_type: TypeName,
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
