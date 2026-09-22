//! Types the language provides itself.
//!
//! These have no source declaration, so their signatures are written here
//! instead of being collected from a program. They were previously spelled out
//! as nested struct literals inside the routine that collects *user* types,
//! where 131 lines of `HashMap::new()` and field-by-field initialization
//! described a single class with two methods and two properties.
//!
//! The small builders below carry the defaults, so each type reads as its
//! shape rather than as the boilerplate around it, and adding another built-in
//! type is a few lines rather than a transcription exercise.

use super::*;
use crate::runtime::well_known;
use std::collections::HashMap;

/// Adds every built-in class to a type registry's class table.
pub(super) fn register_classes(classes: &mut HashMap<String, ClassSig>) {
    let collection = collection();
    classes.insert(key(&collection.name), collection);
}

/// `Collection`: an ordered, optionally keyed sequence of values.
fn collection() -> ClassSig {
    let mut sig = class(well_known::COLLECTION);

    sig.subs.insert(
        key("Add"),
        vec![sub(
            "Add",
            vec![
                required("Item", TypeName::Variant),
                optional("Key", TypeName::String),
                optional("Before", TypeName::Variant),
                optional("After", TypeName::Variant),
            ],
        )],
    );
    sig.subs.insert(
        key("Remove"),
        vec![sub("Remove", vec![required("Index", TypeName::Variant)])],
    );

    sig.properties.insert(
        key(well_known::ITEM),
        read_only_property(
            well_known::ITEM,
            vec![required("Index", TypeName::Variant)],
            TypeName::Variant,
        ),
    );
    sig.properties.insert(
        key("Count"),
        read_only_property("Count", Vec::new(), TypeName::Int32),
    );

    // A collection is always enumerable, and indexing it reaches `Item`.

    sig.default_property = Some(well_known::ITEM.to_string());
    sig
}

/// An empty public class with no base, no members, and no generics.
fn class(name: &str) -> ClassSig {
    ClassSig {
        name: name.to_string(),
        type_params: Vec::new(),
        generic_constraints: Vec::new(),
        inheritance: crate::ClassInheritance::Normal,
        base_class: None,
        implements: Vec::new(),
        visibility: Visibility::Public,
        fields: HashMap::new(),
        subs: HashMap::new(),
        functions: HashMap::new(),
        properties: HashMap::new(),
        events: HashMap::new(),
        operators: HashMap::new(),
        iterator: None,
        default_property: None,
    }
}

/// A public instance `Sub`.
fn sub(name: &str, params: Vec<ParamSig>) -> CallableSig {
    CallableSig {
        attributes: Vec::new(),
        visibility: Visibility::Public,
        name: name.to_string(),
        type_params: Vec::new(),
        generic_constraints: Vec::new(),
        is_shared: false,
        is_declare: false,
        _is_iterator: false,
        params,
        return_type: None,
    }
}

/// A public instance property with only a `Get`.
fn read_only_property(
    name: &str,
    params: Vec<ParamSig>,
    return_type: TypeName,
) -> ClassPropertySig {
    ClassPropertySig {
        name: name.to_string(),
        is_shared: false,
        is_readonly: true,
        is_writeonly: false,
        get: vec![PropertyAccessorSig {
            visibility: Visibility::Public,
            is_iterator: false,
            params,
            return_type: Some(return_type),
        }],
        set: Vec::new(),
    }
}

/// A parameter that must be supplied.
fn required(name: &str, ty: TypeName) -> ParamSig {
    param(name, ty, false)
}

/// A parameter that may be omitted.
fn optional(name: &str, ty: TypeName) -> ParamSig {
    param(name, ty, true)
}

fn param(name: &str, ty: TypeName, is_optional: bool) -> ParamSig {
    ParamSig {
        name: name.to_string(),
        ty,
        mode: PassingMode::ByVal,
        is_optional,
        is_param_array: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_is_registered_with_its_members() {
        let mut classes = HashMap::new();
        register_classes(&mut classes);

        let collection = classes
            .get(&key(well_known::COLLECTION))
            .expect("Collection is a built-in class");

        assert!(collection.subs.contains_key(&key("Add")));
        assert!(collection.subs.contains_key(&key("Remove")));
        assert!(collection.properties.contains_key(&key("Count")));
        assert_eq!(
            collection.default_property.as_deref(),
            Some(well_known::ITEM)
        );
    }

    #[test]
    fn add_accepts_a_value_alone_and_a_value_with_a_key_and_a_position() {
        let collection = collection();
        let add = collection.subs[&key("Add")]
            .first()
            .expect("Add is declared");

        assert_eq!(add.params.len(), 4);
        assert!(!add.params[0].is_optional, "the value is required");
        assert!(
            add.params[1..].iter().all(|param| param.is_optional),
            "the key and position are optional"
        );
    }
}
