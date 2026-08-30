//! Names the implementation itself depends on.
//!
//! These are the identifiers the language reserves for its own machinery: the
//! implicit receiver, the lifecycle hooks, the pseudo-objects a call site can
//! name, and the built-in types. They appear in the lexer, the analyzer, and the
//! interpreter alike, and every one of them has to mean the same thing in all
//! three.
//!
//! Spelled out as literals they were invisible to the compiler: a typo in one of
//! the dozens of `"me"` comparisons would have silently stopped matching
//! anything. Naming them here makes each use a reference to one definition.
//!
//! Names are case-insensitive, so most are stored twice: once as they are
//! written in source, and once folded for use as a lookup key. A test below
//! keeps the two in step.

/// The implicit receiver bound inside every instance member, as written.
pub const SELF: &str = "Me";
/// The implicit receiver, as a symbol-table key.
pub const SELF_KEY: &str = "me";

/// The entry point every program must define.
pub const MAIN: &str = "Main";

// -- Lifecycle hooks -----------------------------------------------------

/// A constructor, in native Valo spelling.
pub const INITIALIZE: &str = "Initialize";
/// A constructor, in the spelling exported VBA class modules use.
pub const CLASS_INITIALIZE: &str = "Class_Initialize";
/// A destructor, in native Valo spelling.
pub const TERMINATE: &str = "Terminate";
/// A destructor, in the spelling exported VBA class modules use.
pub const CLASS_TERMINATE: &str = "Class_Terminate";

/// Both spellings of a constructor, as symbol-table keys, most specific first.
pub const CONSTRUCTOR_KEYS: &[&str] = &["initialize", "class_initialize"];
/// Both spellings of a destructor, as symbol-table keys, most specific first.
pub const DESTRUCTOR_KEYS: &[&str] = &["terminate", "class_terminate"];

// -- Pseudo-objects ------------------------------------------------------

/// The namespace a builtin may be qualified with, as in `VBA.UCase`.
pub const VBA: &str = "VBA";
/// The ambient error object.
pub const ERR: &str = "Err";
/// The console, as in `Console.WriteLine`.
pub const CONSOLE: &str = "Console";
/// The debug channel, as in `Debug.Print`.
pub const DEBUG: &str = "Debug";

// -- Built-in type names -------------------------------------------------

/// A late-bound object reference.
pub const OBJECT: &str = "Object";
/// The built-in collection type.
pub const COLLECTION: &str = "Collection";
/// The type a lambda value reports.
pub const FUNC: &str = "Func";
/// The type a caught error reports.
pub const ERROR: &str = "Error";

// -- Well-known members --------------------------------------------------

/// The default member of an indexable object.
pub const ITEM: &str = "Item";
/// A nullable's payload.
pub const VALUE: &str = "Value";
/// Whether a nullable holds a payload.
pub const HAS_VALUE: &str = "HasValue";

/// Finds a type's constructor among its members, under either spelling.
///
/// Native Valo writes `Initialize`; an exported VBA class module writes
/// `Class_Initialize`. Both name the same hook, so every lookup has to try
/// both, which is why it lives here rather than being spelled out at each call.
pub fn find_constructor<T>(members: &std::collections::HashMap<String, T>) -> Option<&T> {
    find_member(members, CONSTRUCTOR_KEYS)
}

/// Finds a type's destructor among its members, under either spelling.
pub fn find_destructor<T>(members: &std::collections::HashMap<String, T>) -> Option<&T> {
    find_member(members, DESTRUCTOR_KEYS)
}

fn find_member<'a, T>(
    members: &'a std::collections::HashMap<String, T>,
    keys: &[&str],
) -> Option<&'a T> {
    keys.iter().find_map(|key| members.get(*key))
}

/// Reports whether a member key names a constructor.
pub fn is_constructor_key(key: &str) -> bool {
    CONSTRUCTOR_KEYS.contains(&key)
}

/// Reports whether a member key names a destructor.
pub fn is_destructor_key(key: &str) -> bool {
    DESTRUCTOR_KEYS.contains(&key)
}

// -- Compiler-internal names ---------------------------------------------

/// Prefix of the frame slot holding a function's implicit result.
///
/// The leading underscores keep it out of the identifier space a program can
/// write, so it cannot collide with a user's variable.
const RETURN_SLOT_PREFIX: &str = "__return_";

/// Builds the frame slot that holds `name`'s implicit result.
///
/// Assigning to a function's own name sets its result, so the slot is derived
/// from the function name and must be derived the same way everywhere: the
/// analyzer records it and the interpreter looks it up.
pub fn return_slot(name: &str) -> String {
    format!("{RETURN_SLOT_PREFIX}{name}")
}

/// Reports whether `slot` is the return slot belonging to `name`.
///
/// The same answer as comparing against [`return_slot`], without building the
/// key to compare with: every assignment inside a function asks this.
pub fn is_return_slot_for(slot: &str, name: &str) -> bool {
    slot.strip_prefix(RETURN_SLOT_PREFIX) == Some(name)
}

/// Reports whether `name` is one a program may not declare for itself.
pub fn is_reserved_internal(name: &str) -> bool {
    name.starts_with(RETURN_SLOT_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::fold;

    #[test]
    fn every_key_is_the_folded_form_of_its_source_spelling() {
        assert_eq!(fold(SELF), SELF_KEY);
        assert_eq!(fold(INITIALIZE), CONSTRUCTOR_KEYS[0]);
        assert_eq!(fold(CLASS_INITIALIZE), CONSTRUCTOR_KEYS[1]);
        assert_eq!(fold(TERMINATE), DESTRUCTOR_KEYS[0]);
        assert_eq!(fold(CLASS_TERMINATE), DESTRUCTOR_KEYS[1]);
    }

    #[test]
    fn a_return_slot_cannot_collide_with_a_program_name() {
        let slot = return_slot("Total");
        assert!(is_reserved_internal(&slot));
        assert!(!is_reserved_internal("Total"));
    }
}
