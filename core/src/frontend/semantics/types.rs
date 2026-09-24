use crate::runtime::well_known;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::ArrayDecl;
use crate::GenericParamConstraint;
use crate::Visibility;
use crate::frontend::type_model::TypeName;

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
    pub(super) type_params: Vec<String>,
    pub(super) generic_constraints: Vec<GenericParamConstraint>,
    pub(super) implements: Vec<TypeName>,
    pub(super) is_structure: bool,
    pub(super) fields: HashMap<String, FieldSig>,
    pub(super) subs: HashMap<String, MethodOverloads>,
    pub(super) functions: HashMap<String, MethodOverloads>,
    pub(super) properties: HashMap<String, ClassPropertySig>,
    pub(super) operators: HashMap<crate::OperatorKind, ClassMethodSig>,
    pub(super) default_property: Option<String>,
}

#[derive(Default, Clone)]
pub(super) struct TypeRegistry {
    pub(super) types: HashMap<String, TypeSig>,
    pub(super) enums: HashMap<String, EnumSig>,
    pub(super) interfaces: HashMap<String, InterfaceSig>,
    pub(super) classes: HashMap<String, ClassSig>,
    pub(super) delegates: HashMap<String, DelegateSig>,
    pub(super) generic_params: HashSet<String>,
    /// What each generic parameter in scope is constrained to be.
    ///
    /// A constraint is what makes a type parameter usable: `Of T As IShape`
    /// says `T` has the interface's members, and `Of T As New` says it can be
    /// constructed. Without them the parameter is a name with nothing on it.
    /// Filled in for the body being checked, so a `T` constrained in one
    /// procedure does not leak into another's.
    pub(super) generic_constraints: HashMap<String, GenericParamConstraint>,
    /// Simple imported names may denote several distinct declarations. Keep
    /// them out of arbitrary first-import-wins lookup.
    pub(super) import_origins: HashMap<String, String>,
    pub(super) ambiguous_imports: HashMap<String, Vec<String>>,
}

/// A named callable shape: what `Delegate Sub`/`Delegate Function` declares.
///
/// It is the same thing as a procedure signature with no body, so it reuses
/// [`ClassMethodSig`]. What makes it a type is being in the registry.
pub(super) type DelegateSig = ClassMethodSig;

#[allow(dead_code)]
impl TypeRegistry {
    /// The type a generic parameter is constrained to, if it is one.
    pub(super) fn bound_of(&self, name: &str) -> Option<&TypeName> {
        self.generic_constraints.get(&key(name))?.bounds.first()
    }

    /// Whether `New T()` is allowed, which `Of T As New` is what says so.
    pub(super) fn can_construct_generic(&self, name: &str) -> bool {
        self.generic_constraints
            .get(&key(name))
            .is_some_and(|constraint| constraint.require_new)
    }

    /// This registry with the given constraints in scope.
    ///
    /// Used to check one body: the bounds belong to the declaration being
    /// checked, and go out of scope with it.
    pub(super) fn with_constraints(
        &self,
        constraints: &[GenericParamConstraint],
    ) -> std::borrow::Cow<'_, Self> {
        if constraints.is_empty() {
            return std::borrow::Cow::Borrowed(self);
        }
        let mut scoped = self.clone();
        scoped.generic_constraints.extend(
            constraints
                .iter()
                .map(|constraint| (key(&constraint.name), constraint.clone())),
        );
        std::borrow::Cow::Owned(scoped)
    }

    pub(super) fn contains(&self, name: &str) -> bool {
        if self.ambiguous_imports.contains_key(&key(name)) {
            return false;
        }
        self.types.contains_key(&key(name))
            || self.enums.contains_key(&key(name))
            || self.interfaces.contains_key(&key(name))
            || self.classes.contains_key(&key(name))
            || self.delegates.contains_key(&key(name))
            || self.unique_nested_type_name(name).is_some()
    }

    pub(super) fn get(&self, name: &str) -> Option<&TypeSig> {
        if self.ambiguous_imports.contains_key(&key(name)) {
            return None;
        }
        self.types.get(&key(name))
    }

    pub(super) fn get_class(&self, name: &str) -> Option<&ClassSig> {
        if self.ambiguous_imports.contains_key(&key(name)) {
            return None;
        }
        self.classes.get(&key(name))
    }

    pub(super) fn get_enum(&self, name: &str) -> Option<&EnumSig> {
        if self.ambiguous_imports.contains_key(&key(name)) {
            return None;
        }
        self.enums.get(&key(name))
    }

    pub(super) fn get_interface(&self, name: &str) -> Option<&InterfaceSig> {
        if self.ambiguous_imports.contains_key(&key(name)) {
            return None;
        }
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
                if let Some(canonical) = self.unique_nested_type_name(name) {
                    return TypeName::User(canonical);
                }
                // Handle Object
                if name.eq_ignore_ascii_case(well_known::OBJECT) {
                    return TypeName::User("Object".to_string());
                }
                if name.eq_ignore_ascii_case(well_known::COLLECTION) {
                    return TypeName::User("Collection".to_string());
                }
                TypeName::User(name.clone())
            }
            TypeName::GenericInstance { name, args } => {
                let canonical_name = if let Some(sig) = self.types.get(&key(name)) {
                    sig.name.clone()
                } else if let Some(sig) = self.classes.get(&key(name)) {
                    sig.name.clone()
                } else if let Some(sig) = self.interfaces.get(&key(name)) {
                    sig.name.clone()
                } else {
                    name.clone()
                };
                TypeName::GenericInstance {
                    name: canonical_name,
                    args: args
                        .iter()
                        .map(|arg| self.canonical_type_name(arg))
                        .collect(),
                }
            }
            TypeName::Array(inner) => TypeName::Array(Box::new(self.canonical_type_name(inner))),
            _ => ty.clone(),
        }
    }

    fn unique_nested_type_name(&self, name: &str) -> Option<String> {
        if name.contains('.') {
            return None;
        }
        let name_key = key(name);
        let mut matches = Vec::new();
        for sig in self.types.values() {
            if sig
                .name
                .rsplit('.')
                .next()
                .is_some_and(|short| key(short) == name_key)
            {
                matches.push(sig.name.clone());
            }
        }
        for sig in self.enums.values() {
            if sig
                .name
                .rsplit('.')
                .next()
                .is_some_and(|short| key(short) == name_key)
            {
                matches.push(sig.name.clone());
            }
        }
        for sig in self.interfaces.values() {
            if sig
                .name
                .rsplit('.')
                .next()
                .is_some_and(|short| key(short) == name_key)
            {
                matches.push(sig.name.clone());
            }
        }
        for sig in self.classes.values() {
            if sig
                .name
                .rsplit('.')
                .next()
                .is_some_and(|short| key(short) == name_key)
            {
                matches.push(sig.name.clone());
            }
        }
        matches.sort();
        matches.dedup();
        (matches.len() == 1).then(|| matches.remove(0))
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
    pub(super) inheritance: crate::ClassInheritance,
    pub(super) name: String,
    pub(super) type_params: Vec<String>,
    pub(super) generic_constraints: Vec<GenericParamConstraint>,
    pub(super) base_class: Option<TypeName>,
    pub(super) implements: Vec<TypeName>,
    pub(super) fields: HashMap<String, ClassFieldSig>,
    pub(super) events: HashMap<String, ClassEventSig>,
    pub(super) subs: HashMap<String, MethodOverloads>,
    pub(super) functions: HashMap<String, MethodOverloads>,
    pub(super) iterator: Option<ClassMethodSig>,
    pub(super) properties: HashMap<String, ClassPropertySig>,
    pub(super) operators: HashMap<crate::OperatorKind, ClassMethodSig>,
    pub(super) default_property: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) struct InterfaceSig {
    pub(super) visibility: Visibility,
    pub(super) name: String,
    pub(super) type_params: Vec<String>,
    pub(super) generic_constraints: Vec<GenericParamConstraint>,
    pub(super) subs: HashMap<String, MethodOverloads>,
    pub(super) functions: HashMap<String, MethodOverloads>,
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

/// The methods of one type sharing a name, in declaration order.
///
/// The module-level [`Overloads`](crate::semantics::symbols::Overloads) holds
/// the same thing for procedures. Both are lists whether or not the name is
/// actually overloaded, so one code path serves both.
pub(super) type MethodOverloads = Vec<ClassMethodSig>;
pub(super) type ClassEventSig = crate::semantics::symbols::CallableSig;

#[derive(Debug, Clone)]
pub(super) struct ClassPropertySig {
    pub(super) name: String,
    pub(super) is_shared: bool,
    pub(super) is_readonly: bool,
    pub(super) is_writeonly: bool,
    /// The accessors of each kind, in declaration order.
    ///
    /// A property usually has one of each. Several are overloads, picked by the
    /// arguments at the use site the way a method call is: `Item(1)` and
    /// `Item("a")` can reach different getters.
    pub(super) get: Vec<PropertyAccessorSig>,
    pub(super) set: Vec<PropertyAccessorSig>,
}

impl ClassPropertySig {
    /// The getter to answer with when the use site offers nothing to choose by.
    pub(super) fn getter(&self) -> Option<&PropertyAccessorSig> {
        self.get.first()
    }

    /// The accessor used for a property assignment.
    pub(super) fn writer(&self) -> Option<&PropertyAccessorSig> {
        self.set.first()
    }

    /// Every accessor a write could go through.
    pub(super) fn writers(&self) -> &[PropertyAccessorSig] {
        &self.set
    }

    pub(super) fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        ClassPropertySig {
            name: self.name.clone(),
            is_shared: self.is_shared,
            is_readonly: self.is_readonly,
            is_writeonly: self.is_writeonly,
            get: substitute_accessors(&self.get, bindings),
            set: substitute_accessors(&self.set, bindings),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct PropertyAccessorSig {
    pub(super) visibility: Visibility,
    pub(super) is_iterator: bool,
    pub(super) params: Vec<crate::semantics::symbols::ParamSig>,
    pub(super) return_type: Option<TypeName>,
}

impl PropertyAccessorSig {
    pub(super) fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        PropertyAccessorSig {
            visibility: self.visibility,
            is_iterator: self.is_iterator,
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

fn substitute_accessors(
    accessors: &[PropertyAccessorSig],
    bindings: &[(String, TypeName)],
) -> Vec<PropertyAccessorSig> {
    accessors
        .iter()
        .map(|accessor| accessor.substitute_generics(bindings))
        .collect()
}
