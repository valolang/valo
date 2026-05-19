use std::collections::HashMap;

use crate::Visibility;
use crate::runtime::TypeName;

use crate::semantics::symbols::key;

#[derive(Debug, Clone)]
pub(super) struct FieldSig {
    pub(super) ty: TypeName,
}

#[derive(Debug, Clone)]
pub(super) struct TypeSig {
    pub(super) name: String,
    pub(super) fields: HashMap<String, FieldSig>,
}

pub(super) struct TypeRegistry {
    pub(super) types: HashMap<String, TypeSig>,
    pub(super) enums: HashMap<String, EnumSig>,
    pub(super) classes: HashMap<String, ClassSig>,
}

impl TypeRegistry {
    pub(super) fn contains(&self, name: &str) -> bool {
        self.types.contains_key(&key(name))
            || self.enums.contains_key(&key(name))
            || self.classes.contains_key(&key(name))
    }

    pub(super) fn get(&self, name: &str) -> Option<&TypeSig> {
        self.types.get(&key(name))
    }

    pub(super) fn get_class(&self, name: &str) -> Option<&ClassSig> {
        self.classes.get(&key(name))
    }

    pub(super) fn get_enum(&self, name: &str) -> Option<&EnumSig> {
        self.enums.get(&key(name))
    }
}

#[derive(Debug, Clone)]
pub(super) struct EnumSig {
    pub(super) name: String,
    pub(super) members: HashMap<String, i64>,
}

#[derive(Debug, Clone)]
pub(super) struct ClassSig {
    pub(super) name: String,
    pub(super) fields: HashMap<String, ClassFieldSig>,
    pub(super) events: HashMap<String, ClassEventSig>,
    pub(super) subs: HashMap<String, ClassMethodSig>,
    pub(super) functions: HashMap<String, ClassMethodSig>,
    pub(super) properties: HashMap<String, ClassPropertySig>,
    pub(super) default_property: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct ClassFieldSig {
    pub(super) visibility: Visibility,
    pub(super) with_events: bool,
    pub(super) ty: TypeName,
}

pub(super) type ClassMethodSig = crate::semantics::symbols::CallableSig;
pub(super) type ClassEventSig = crate::semantics::symbols::CallableSig;

#[derive(Debug, Clone)]
pub(super) struct ClassPropertySig {
    pub(super) name: String,
    pub(super) get: Option<PropertyAccessorSig>,
    pub(super) let_: Option<PropertyAccessorSig>,
    pub(super) set: Option<PropertyAccessorSig>,
}

#[derive(Debug, Clone)]
pub(super) struct PropertyAccessorSig {
    pub(super) visibility: Visibility,
    pub(super) params: Vec<crate::semantics::symbols::ParamSig>,
    pub(super) return_type: Option<TypeName>,
}
