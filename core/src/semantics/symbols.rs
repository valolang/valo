use std::collections::HashMap;

use crate::runtime::TypeName;
use crate::{Expr, PassingMode, Visibility};

#[derive(Debug, Clone)]
pub(super) enum VarType {
    Scalar(TypeName),
    Array(TypeName),
    Const(TypeName),
}

impl VarType {
    pub(super) fn same_var_type(&self, other: &VarType) -> bool {
        match (self, other) {
            (VarType::Scalar(left), VarType::Scalar(right))
            | (VarType::Const(left), VarType::Scalar(right))
            | (VarType::Scalar(left), VarType::Const(right))
            | (VarType::Const(left), VarType::Const(right))
            | (VarType::Array(left), VarType::Array(right)) => left.same_type(right),
            _ => false,
        }
    }

    pub(super) fn display_name(&self) -> String {
        match self {
            VarType::Scalar(ty) => ty.display_name(),
            VarType::Const(ty) => ty.display_name(),
            VarType::Array(ty) => format!("{}()", ty.display_name()),
        }
    }

    pub(super) fn scalar_type(&self) -> Option<TypeName> {
        match self {
            VarType::Scalar(ty) | VarType::Const(ty) => Some(ty.clone()),
            VarType::Array(_) => None,
        }
    }

    pub(super) fn is_const(&self) -> bool {
        matches!(self, VarType::Const(_))
    }
}

#[derive(Debug, Clone)]
pub(super) struct ParamSig {
    pub(super) mode: PassingMode,
    pub(super) ty: TypeName,
    pub(super) optional_default: Option<Expr>,
    pub(super) is_param_array: bool,
}

#[derive(Debug, Clone)]
pub(super) struct CallableSig {
    pub(super) visibility: Visibility,
    pub(super) name: String,
    pub(super) params: Vec<ParamSig>,
    pub(super) return_type: Option<TypeName>,
}

pub(super) struct Signatures {
    pub(super) subs: HashMap<String, CallableSig>,
    pub(super) functions: HashMap<String, CallableSig>,
}

pub(super) fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}
