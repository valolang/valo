use crate::runtime::{Span, TypeName};

use super::Stmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionCompare {
    Binary,
    Text,
}

impl Default for OptionCompare {
    fn default() -> Self {
        Self::Binary
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleVarDecl {
    pub visibility: Visibility,
    pub name: String,
    pub ty: TypeName,
    pub array: Option<super::ArrayDecl>,
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
    pub name: String,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Field(ClassField),
    Sub(ClassSub),
    Function(ClassFunction),
    Property(ClassProperty),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassField {
    pub visibility: Visibility,
    pub name: String,
    pub ty: TypeName,
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
    pub function: Function,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassProperty {
    pub visibility: Visibility,
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
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
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
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassingMode {
    ByVal,
    ByRef,
}
