use valo_core::semantics::type_properties::{KnownProperty, properties};
use valo_core::{TypeName, parse_source};

#[test]
fn plain_scalars_and_structures_are_copy_without_drop() {
    let program = parse_source("Structure Vec2\nPublic X As Single\nPublic Y As Single\nEnd Structure\nStructure Pair\nPublic A As Vec2\nPublic B As Integer\nEnd Structure").unwrap();
    for ty in [
        TypeName::Int32,
        TypeName::Single,
        TypeName::User("Vec2".into()),
        TypeName::User("Pair".into()),
    ] {
        let result = properties(&program, &ty);
        assert_eq!(result.copy, KnownProperty::Yes, "{ty:?}");
        assert_eq!(result.requires_drop, KnownProperty::No, "{ty:?}");
    }
}

#[test]
fn class_references_and_containing_structures_require_managed_copy_and_drop() {
    let program = parse_source("Structure Holder\nPublic Resource As FileHandle\nEnd Structure\nClass FileHandle\nEnd Class").unwrap();
    let string = properties(&program, &TypeName::String);
    assert_eq!(string.copy, KnownProperty::Yes);
    assert_eq!(string.requires_drop, KnownProperty::Yes);
    for ty in [
        TypeName::User("FileHandle".into()),
        TypeName::User("Holder".into()),
    ] {
        let result = properties(&program, &ty);
        assert_eq!(result.copy, KnownProperty::Yes, "{ty:?}");
        assert_eq!(result.requires_drop, KnownProperty::Yes, "{ty:?}");
    }
    let unknown = properties(&program, &TypeName::User("Unresolved".into()));
    assert_eq!(unknown.copy, KnownProperty::Unknown);
    assert_eq!(unknown.requires_drop, KnownProperty::Unknown);
}

#[test]
fn string_fields_propagate_managed_copy_and_drop() {
    let program = parse_source("Structure Person\nPublic Name As String\nEnd Structure\nStructure Record\nPublic Owner As Person\nEnd Structure").unwrap();
    for ty in [
        TypeName::User("Person".into()),
        TypeName::User("Record".into()),
    ] {
        let result = properties(&program, &ty);
        assert_eq!(result.copy, KnownProperty::Yes);
        assert_eq!(result.requires_drop, KnownProperty::Yes);
    }
}
