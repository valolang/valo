//! Conservative native ownership properties for resolved source types.
//! Unknown is never silently treated as Copy or as requiring no cleanup.
use std::collections::HashSet;

use crate::frontend::type_model::TypeName;
use crate::{Program, TypeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownProperty {
    Yes,
    No,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeProperties {
    pub copy: KnownProperty,
    pub requires_drop: KnownProperty,
}

impl TypeProperties {
    const PLAIN: Self = Self {
        copy: KnownProperty::Yes,
        requires_drop: KnownProperty::No,
    };
    const UNKNOWN: Self = Self {
        copy: KnownProperty::Unknown,
        requires_drop: KnownProperty::Unknown,
    };
}

pub fn properties(program: &Program, ty: &TypeName) -> TypeProperties {
    resolve(program, ty, &mut HashSet::new())
}

fn resolve(program: &Program, ty: &TypeName, visiting: &mut HashSet<String>) -> TypeProperties {
    match ty {
        TypeName::Byte
        | TypeName::Int16
        | TypeName::Int32
        | TypeName::Int64
        | TypeName::UInt32
        | TypeName::UInt64
        | TypeName::Single
        | TypeName::Double
        | TypeName::Boolean
        | TypeName::Ptr
        | TypeName::FuncPtr
        | TypeName::Enum(_) => TypeProperties::PLAIN,
        // String is semantically copyable; native copies retain immutable
        // shared storage and every owned reference requires release.
        TypeName::String => TypeProperties {
            copy: KnownProperty::Yes,
            requires_drop: KnownProperty::Yes,
        },
        TypeName::Nullable(inner) => resolve(program, inner, visiting),
        TypeName::Tuple(elements) => combine(
            elements
                .iter()
                .map(|element| resolve(program, &element.ty, visiting)),
        ),
        TypeName::User(name) => {
            let Some(decl) = program
                .types
                .iter()
                .find(|decl| decl.name.eq_ignore_ascii_case(name))
            else {
                return TypeProperties::UNKNOWN;
            };
            if decl.kind != TypeKind::Structure || !visiting.insert(name.to_ascii_lowercase()) {
                return TypeProperties::UNKNOWN;
            }
            let result = combine(decl.fields.iter().map(|field| {
                if field.array.is_some() {
                    TypeProperties::UNKNOWN
                } else {
                    resolve(program, &field.ty, visiting)
                }
            }));
            visiting.remove(&name.to_ascii_lowercase());
            result
        }
        // Class ownership, arrays, dynamic values, generics and
        // runtime-specific scalars need explicit native layout/drop contracts.
        _ => TypeProperties::UNKNOWN,
    }
}

fn combine(values: impl Iterator<Item = TypeProperties>) -> TypeProperties {
    let mut result = TypeProperties::PLAIN;
    for value in values {
        result.copy = combine_copy(result.copy, value.copy);
        result.requires_drop = combine_drop(result.requires_drop, value.requires_drop);
    }
    result
}

fn combine_copy(left: KnownProperty, right: KnownProperty) -> KnownProperty {
    match (left, right) {
        (KnownProperty::No, _) | (_, KnownProperty::No) => KnownProperty::No,
        (KnownProperty::Unknown, _) | (_, KnownProperty::Unknown) => KnownProperty::Unknown,
        _ => KnownProperty::Yes,
    }
}

fn combine_drop(left: KnownProperty, right: KnownProperty) -> KnownProperty {
    match (left, right) {
        (KnownProperty::Yes, _) | (_, KnownProperty::Yes) => KnownProperty::Yes,
        (KnownProperty::Unknown, _) | (_, KnownProperty::Unknown) => KnownProperty::Unknown,
        _ => KnownProperty::No,
    }
}
