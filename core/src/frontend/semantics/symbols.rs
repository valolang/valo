use std::collections::HashMap;

use crate::runtime::TypeName;
use crate::{PassingMode, Visibility};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) enum VarType {
    Scalar(Visibility, TypeName),
    Optional(Visibility, TypeName),
    Array(Visibility, TypeName),
    Const(Visibility, TypeName),
    FunctionReturn(TypeName),
    Module(String),
}

#[allow(dead_code)]
impl VarType {
    pub(super) fn same_var_type(&self, other: &VarType) -> bool {
        match (self, other) {
            (VarType::Scalar(_, left), VarType::Scalar(_, right))
            | (VarType::Optional(_, left), VarType::Scalar(_, right))
            | (VarType::Scalar(_, left), VarType::Optional(_, right))
            | (VarType::Optional(_, left), VarType::Optional(_, right))
            | (VarType::Const(_, left), VarType::Scalar(_, right))
            | (VarType::Scalar(_, left), VarType::Const(_, right))
            | (VarType::Const(_, left), VarType::Const(_, right))
            | (VarType::Array(_, left), VarType::Array(_, right))
            | (VarType::FunctionReturn(left), VarType::Scalar(_, right))
            | (VarType::Scalar(_, left), VarType::FunctionReturn(right))
            | (VarType::FunctionReturn(left), VarType::FunctionReturn(right)) => {
                left.same_type(right)
            }
            (VarType::Module(left), VarType::Module(right)) => left == right,
            _ => false,
        }
    }

    pub(super) fn display_name(&self) -> String {
        match self {
            VarType::Scalar(_, ty) | VarType::Optional(_, ty) | VarType::FunctionReturn(ty) => {
                ty.display_name()
            }
            VarType::Const(_, ty) => ty.display_name(),
            VarType::Array(_, ty) => format!("{}()", ty.display_name()),
            VarType::Module(name) => format!("Module {}", name),
        }
    }

    pub(super) fn scalar_type(&self) -> Option<TypeName> {
        match self {
            VarType::Scalar(_, ty)
            | VarType::Optional(_, ty)
            | VarType::Const(_, ty)
            | VarType::FunctionReturn(ty) => Some(ty.clone()),
            VarType::Array(_, _) | VarType::Module(_) => None,
        }
    }

    pub(super) fn is_const(&self) -> bool {
        matches!(self, VarType::Const(_, _))
    }

    pub(super) fn visibility(&self) -> Visibility {
        match self {
            VarType::Scalar(v, _)
            | VarType::Optional(v, _)
            | VarType::Array(v, _)
            | VarType::Const(v, _) => *v,
            VarType::FunctionReturn(_) | VarType::Module(_) => Visibility::Public,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ParamSig {
    pub(super) name: String,
    pub(super) mode: PassingMode,
    pub(super) ty: TypeName,
    pub(super) is_optional: bool,
    pub(super) is_param_array: bool,
}

#[derive(Debug, Clone)]
pub(super) struct CallableSig {
    pub(super) visibility: Visibility,
    pub(super) name: String,
    pub(super) is_shared: bool,
    pub(super) _is_iterator: bool,
    pub(super) is_declare: bool,
    pub(super) params: Vec<ParamSig>,
    pub(super) return_type: Option<TypeName>,
}

#[derive(Clone)]
pub(super) struct Signatures {
    pub(super) subs: HashMap<String, CallableSig>,
    pub(super) functions: HashMap<String, CallableSig>,
}

pub(super) fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}
