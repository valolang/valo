use std::collections::HashMap;

use crate::runtime::TypeName;
use crate::{GenericParamConstraint, PassingMode, Visibility};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) enum VarType {
    Scalar(Visibility, TypeName),
    Optional(Visibility, TypeName),
    Array(Visibility, TypeName, bool), // Visibility, element type, is_dynamic
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
            | (VarType::Optional(_, left), VarType::Optional(_, right)) => left.same_type(right),
            (VarType::Const(left_v, left), VarType::Const(right_v, right)) => {
                left_v == right_v && left.same_type(right)
            }
            (VarType::Array(left_v, left, left_d), VarType::Array(right_v, right, right_d)) => {
                left_v == right_v && left_d == right_d && left.same_type(right)
            }
            (VarType::FunctionReturn(left), VarType::Scalar(_, right))
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
            VarType::Array(_, ty, is_dynamic) => {
                if *is_dynamic {
                    format!("{}()", ty.display_name())
                } else {
                    format!("{} (fixed array)", ty.display_name())
                }
            }
            VarType::Module(name) => format!("Module {}", name),
        }
    }

    pub(super) fn scalar_type(&self) -> Option<TypeName> {
        match self {
            VarType::Scalar(_, ty)
            | VarType::Optional(_, ty)
            | VarType::Const(_, ty)
            | VarType::FunctionReturn(ty) => Some(ty.clone()),
            VarType::Array(_, _, _) | VarType::Module(_) => None,
        }
    }

    pub(super) fn is_const(&self) -> bool {
        matches!(self, VarType::Const(_, _))
    }

    pub(super) fn visibility(&self) -> Visibility {
        match self {
            VarType::Scalar(v, _)
            | VarType::Optional(v, _)
            | VarType::Array(v, _, _)
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

impl ParamSig {
    pub(super) fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        ParamSig {
            name: self.name.clone(),
            mode: self.mode,
            ty: self.ty.substitute_generics(bindings),
            is_optional: self.is_optional,
            is_param_array: self.is_param_array,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct CallableSig {
    pub(super) visibility: Visibility,
    pub(super) name: String,
    pub(super) type_params: Vec<String>,
    pub(super) generic_constraints: Vec<GenericParamConstraint>,
    pub(super) is_shared: bool,
    pub(super) _is_iterator: bool,
    pub(super) is_declare: bool,
    pub(super) params: Vec<ParamSig>,
    pub(super) return_type: Option<TypeName>,
}

impl CallableSig {
    pub(super) fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        CallableSig {
            visibility: self.visibility,
            name: self.name.clone(),
            type_params: self.type_params.clone(),
            generic_constraints: self.generic_constraints.clone(),
            is_shared: self.is_shared,
            _is_iterator: self._is_iterator,
            is_declare: self.is_declare,
            params: self
                .params
                .iter()
                .map(|p| p.substitute_generics(bindings))
                .collect(),
            return_type: self
                .return_type
                .as_ref()
                .map(|ty| ty.substitute_generics(bindings)),
        }
    }
}

#[derive(Clone)]
pub(super) struct Signatures {
    pub(super) subs: HashMap<String, CallableSig>,
    pub(super) functions: HashMap<String, CallableSig>,
}

pub(super) fn key(name: &str) -> String {
    name.to_lowercase()
}
