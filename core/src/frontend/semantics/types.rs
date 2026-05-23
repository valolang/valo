use std::collections::HashMap;

use crate::ArrayDecl;
use crate::Visibility;
use crate::runtime::TypeName;

use crate::frontend::semantics::symbols::key;

#[derive(Debug, Clone)]
pub(super) struct FieldSig {
    pub(super) visibility: Visibility,
    pub(super) ty: TypeName,
    pub(super) array: Option<ArrayDecl>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) struct TypeSig {
    pub(super) visibility: Visibility,
    pub(super) name: String,
    pub(super) is_structure: bool,
    pub(super) fields: HashMap<String, FieldSig>,
    pub(super) subs: HashMap<String, ClassMethodSig>,
    pub(super) functions: HashMap<String, ClassMethodSig>,
    pub(super) properties: HashMap<String, ClassPropertySig>,
    pub(super) default_property: Option<String>,
}

#[derive(Clone)]
pub(super) struct TypeRegistry {
    pub(super) types: HashMap<String, TypeSig>,
    pub(super) enums: HashMap<String, EnumSig>,
    pub(super) interfaces: HashMap<String, InterfaceSig>,
    pub(super) classes: HashMap<String, ClassSig>,
}

#[allow(dead_code)]
impl TypeRegistry {
    pub(super) fn contains(&self, name: &str) -> bool {
        self.types.contains_key(&key(name))
            || self.enums.contains_key(&key(name))
            || self.interfaces.contains_key(&key(name))
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

    pub(super) fn get_interface(&self, name: &str) -> Option<&InterfaceSig> {
        self.interfaces.get(&key(name))
    }

    pub(super) fn canonical_type_name(&self, ty: &TypeName) -> TypeName {
        match ty {
            TypeName::User(name) => {
                if let Some(sig) = self.types.get(&key(name)) {
                    return TypeName::User(sig.name.clone());
                }
                if let Some(sig) = self.classes.get(&key(name)) {
                    return TypeName::User(sig.name.clone());
                }
                if let Some(sig) = self.enums.get(&key(name)) {
                    return TypeName::User(sig.name.clone());
                }
                if let Some(sig) = self.interfaces.get(&key(name)) {
                    return TypeName::User(sig.name.clone());
                }
                // Handle Object
                if name.eq_ignore_ascii_case("Object") {
                    return TypeName::User("Object".to_string());
                }
                TypeName::User(name.clone())
            }
            TypeName::Array(inner) => TypeName::Array(Box::new(self.canonical_type_name(inner))),
            _ => ty.clone(),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) struct EnumSig {
    pub(super) visibility: Visibility,
    pub(super) name: String,
    pub(super) members: HashMap<String, i64>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) struct ClassSig {
    pub(super) visibility: Visibility,
    pub(super) name: String,
    pub(super) fields: HashMap<String, ClassFieldSig>,
    pub(super) events: HashMap<String, ClassEventSig>,
    pub(super) subs: HashMap<String, ClassMethodSig>,
    pub(super) functions: HashMap<String, ClassMethodSig>,
    pub(super) iterator: Option<ClassMethodSig>,
    pub(super) properties: HashMap<String, ClassPropertySig>,
    pub(super) enumerator: Option<String>,
    pub(super) default_property: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) struct InterfaceSig {
    pub(super) visibility: Visibility,
    pub(super) name: String,
    pub(super) subs: HashMap<String, ClassMethodSig>,
    pub(super) functions: HashMap<String, ClassMethodSig>,
    pub(super) events: HashMap<String, ClassEventSig>,
    pub(super) properties: HashMap<String, ClassPropertySig>,
}

#[derive(Debug, Clone)]
pub(super) struct ClassFieldSig {
    pub(super) visibility: Visibility,
    pub(super) is_shared: bool,
    pub(super) with_events: bool,
    pub(super) ty: TypeName,
    pub(super) array: Option<ArrayDecl>,
}

pub(super) type ClassMethodSig = crate::semantics::symbols::CallableSig;
pub(super) type ClassEventSig = crate::semantics::symbols::CallableSig;

#[derive(Debug, Clone)]
pub(super) struct ClassPropertySig {
    pub(super) name: String,
    pub(super) is_shared: bool,
    pub(super) get: Option<PropertyAccessorSig>,
    pub(super) let_: Option<PropertyAccessorSig>,
    pub(super) set: Option<PropertyAccessorSig>,
}

#[derive(Debug, Clone)]
pub(super) struct PropertyAccessorSig {
    pub(super) visibility: Visibility,
    pub(super) is_iterator: bool,
    pub(super) params: Vec<crate::semantics::symbols::ParamSig>,
    pub(super) return_type: Option<TypeName>,
}
