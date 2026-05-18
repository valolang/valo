use std::collections::HashMap;

use valo_parser::{PassingMode, Visibility};
use valo_runtime::TypeName;

#[derive(Debug, Clone)]
pub(super) enum VarType {
    Scalar(TypeName),
    Array(TypeName),
}

impl VarType {
    pub(super) fn same_var_type(&self, other: &VarType) -> bool {
        match (self, other) {
            (VarType::Scalar(left), VarType::Scalar(right))
            | (VarType::Array(left), VarType::Array(right)) => left.same_type(right),
            _ => false,
        }
    }

    pub(super) fn display_name(&self) -> String {
        match self {
            VarType::Scalar(ty) => ty.display_name(),
            VarType::Array(ty) => format!("{}()", ty.display_name()),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ParamSig {
    pub(super) mode: PassingMode,
    pub(super) ty: TypeName,
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
