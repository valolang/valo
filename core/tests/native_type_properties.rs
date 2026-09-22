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
fn unresolved_native_ownership_is_not_assumed_copy_or_drop_free() {
    let program = parse_source("Structure Holder\nPublic Resource As FileHandle\nEnd Structure\nClass FileHandle\nEnd Class").unwrap();
    for ty in [
        TypeName::String,
        TypeName::User("FileHandle".into()),
        TypeName::User("Holder".into()),
    ] {
        let result = properties(&program, &ty);
        assert_eq!(result.copy, KnownProperty::Unknown, "{ty:?}");
        assert_eq!(result.requires_drop, KnownProperty::Unknown, "{ty:?}");
    }
}
