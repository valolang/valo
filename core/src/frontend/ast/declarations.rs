use crate::runtime::{Span, TypeName};

use super::Stmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub namespace: Option<String>,
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

impl Program {
    pub fn flatten_nested_types(&mut self) {
        let mut new_types = Vec::new();
        let mut new_enums = Vec::new();
        let mut new_classes = Vec::new();

        fn extract_nested(
            members: &mut Vec<ClassMember>,
            prefix: &str,
            parent_type_params: &[String],
            new_types: &mut Vec<TypeDecl>,
            new_enums: &mut Vec<EnumDecl>,
            new_classes: &mut Vec<ClassDecl>,
        ) {
            let mut i = 0;
            while i < members.len() {
                match &mut members[i] {
                    ClassMember::Type(t) => {
                        let mut t_clone = t.clone();
                        let mut params = parent_type_params.to_vec();
                        params.extend(t_clone.type_params);
                        t_clone.type_params = params;
                        t_clone.name = format!("{}.{}", prefix, t.name);
                        new_types.push(t_clone);
                        members.remove(i);
                    }
                    ClassMember::Enum(e) => {
                        let mut e_clone = e.clone();
                        e_clone.name = format!("{}.{}", prefix, e.name);
                        new_enums.push(e_clone);
                        members.remove(i);
                    }
                    ClassMember::Class(c) => {
                        let mut c_clone = *c.clone();
                        let qualified = format!("{}.{}", prefix, c.name);
                        let mut params = parent_type_params.to_vec();
                        params.extend(c_clone.type_params);
                        c_clone.type_params = params.clone();
                        c_clone.name = qualified.clone();
                        extract_nested(
                            &mut c_clone.members,
                            &qualified,
                            &params,
                            new_types,
                            new_enums,
                            new_classes,
                        );
                        new_classes.push(c_clone);
                        members.remove(i);
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
        }

        for c in &mut self.classes {
            extract_nested(
                &mut c.members,
                &c.name,
                &c.type_params,
                &mut new_types,
                &mut new_enums,
                &mut new_classes,
            );
        }

        self.types.append(&mut new_types);
        self.enums.append(&mut new_enums);
        self.classes.append(&mut new_classes);
    }
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
    pub type_params: Vec<String>,
    pub generic_constraints: Vec<GenericParamConstraint>,
    pub implements: Vec<TypeName>,
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
    pub inheritance: ClassInheritance,
    pub name: String,
    pub type_params: Vec<String>,
    pub generic_constraints: Vec<GenericParamConstraint>,
    pub base_class: Option<TypeName>,
    pub implements: Vec<TypeName>,
    pub attributes: Vec<AttributeDecl>,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClassInheritance {
    #[default]
    Normal,
    MustInherit,
    NotInheritable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub visibility: Visibility,
    pub name: String,
    pub type_params: Vec<String>,
    pub generic_constraints: Vec<GenericParamConstraint>,
    pub members: Vec<InterfaceMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GenericParamConstraint {
    pub name: String,
    pub require_class: bool,
    pub require_structure: bool,
    pub require_new: bool,
    pub bounds: Vec<TypeName>,
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
    Operator(OperatorDecl),
    Type(TypeDecl),
    Declare(DeclareDecl),
    Enum(EnumDecl),
    Class(Box<ClassDecl>),
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
    pub override_kind: OverrideKind,
    pub is_shared: bool,
    pub implements: Vec<ImplementsClause>,
    pub procedure: Procedure,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassFunction {
    pub visibility: Visibility,
    pub override_kind: OverrideKind,
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
    pub override_kind: OverrideKind,
    pub is_shared: bool,
    pub implements: Vec<ImplementsClause>,
    pub is_default: bool,
    pub is_enumerator: bool,
    pub is_iterator: bool,
    pub is_readonly: bool,
    pub is_writeonly: bool,
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
    Protected,
    ProtectedFriend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverrideKind {
    #[default]
    None,
    Overridable,
    Overrides,
    MustOverride,
    Shadows,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OperatorDecl {
    pub visibility: Visibility,
    pub kind: OperatorKind,
    pub params: Vec<Parameter>,
    pub return_type: TypeName,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatorKind {
    Add,
    Subtract,
    Multiply,
    Divide,
    IntegerDivide,
    Exponent,
    Modulo,
    And,
    Or,
    Xor,
    Not,
    UnaryMinus,
    UnaryPlus,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Like,
    Concatenate,
    True,
    False,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModernAttribute {
    pub name: String,
    pub args: Option<Vec<crate::frontend::ast::Expr>>,
    pub span: crate::runtime::Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Procedure {
    pub attributes: Vec<ModernAttribute>,
    pub visibility: Visibility,
    pub name: String,
    pub type_params: Vec<String>,
    pub generic_constraints: Vec<GenericParamConstraint>,
    pub params: Vec<Parameter>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub attributes: Vec<ModernAttribute>,
    pub visibility: Visibility,
    pub name: String,
    pub is_iterator: bool,
    pub type_params: Vec<String>,
    pub generic_constraints: Vec<GenericParamConstraint>,
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
