use std::collections::HashMap;

use valo_parser::Visibility;
use valo_runtime::TypeName;

use crate::symbols::key;

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
    pub(super) classes: HashMap<String, ClassSig>,
}

impl TypeRegistry {
    pub(super) fn contains(&self, name: &str) -> bool {
        self.types.contains_key(&key(name)) || self.classes.contains_key(&key(name))
    }

    pub(super) fn get(&self, name: &str) -> Option<&TypeSig> {
        self.types.get(&key(name))
    }

    pub(super) fn get_class(&self, name: &str) -> Option<&ClassSig> {
        self.classes.get(&key(name))
    }
}

#[derive(Debug, Clone)]
pub(super) struct ClassSig {
    pub(super) name: String,
    pub(super) fields: HashMap<String, ClassFieldSig>,
    pub(super) subs: HashMap<String, ClassMethodSig>,
    pub(super) functions: HashMap<String, ClassMethodSig>,
}

#[derive(Debug, Clone)]
pub(super) struct ClassFieldSig {
    pub(super) visibility: Visibility,
    pub(super) ty: TypeName,
}

pub(super) type ClassMethodSig = crate::symbols::CallableSig;
