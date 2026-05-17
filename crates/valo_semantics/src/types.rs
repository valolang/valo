use std::collections::HashMap;

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
}

impl TypeRegistry {
    pub(super) fn contains(&self, name: &str) -> bool {
        self.types.contains_key(&key(name))
    }

    pub(super) fn get(&self, name: &str) -> Option<&TypeSig> {
        self.types.get(&key(name))
    }
}
