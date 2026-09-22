//! The builtin function registry.
//!
//! Every builtin is declared once, here, with the name it is called by, how many
//! arguments it accepts, and what it produces. Both the semantic analyzer and
//! the interpreter read this table, so a builtin cannot be known to one and
//! unknown to the other, and its arity cannot be checked one way and
//! implemented another.
//!
//! Adding a builtin means adding a row here and an implementation in
//! `backend::interpreter::builtins`. A test in that module asserts the two stay
//! in step.

use super::TypeName;

/// What a builtin produces.
///
/// Every builtin has one fixed result type. A builtin whose result varies with
/// its arguments is declared [`BuiltinReturn::Variant`] and narrowed by the
/// analyzer, which has the argument types to hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinReturn {
    Boolean,
    Byte,
    Integer,
    Long,
    Int64,
    Single,
    Double,
    Currency,
    Decimal,
    Date,
    String,
    Variant,
    Ptr,
    /// A late-bound COM handle, spelled `Object` in source.
    Object,
    /// A one-dimensional array of strings.
    StringArray,
}

impl BuiltinReturn {
    /// The declared type this result corresponds to.
    pub fn type_name(self) -> TypeName {
        match self {
            BuiltinReturn::Boolean => TypeName::Boolean,
            BuiltinReturn::Byte => TypeName::Byte,
            BuiltinReturn::Integer => TypeName::Int32,
            BuiltinReturn::Long => TypeName::Int64,
            BuiltinReturn::Int64 => TypeName::Int64,
            BuiltinReturn::Single => TypeName::Single,
            BuiltinReturn::Double => TypeName::Double,
            BuiltinReturn::Currency => TypeName::Currency,
            BuiltinReturn::Decimal => TypeName::Decimal,
            BuiltinReturn::Date => TypeName::Date,
            BuiltinReturn::String => TypeName::String,
            BuiltinReturn::Variant => TypeName::Variant,
            BuiltinReturn::Ptr => TypeName::Ptr,
            BuiltinReturn::Object => TypeName::User("Object".to_string()),
            BuiltinReturn::StringArray => TypeName::Array(Box::new(TypeName::String)),
        }
    }
}

/// How a builtin may be written at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinForm {
    /// Only usable where a value is expected.
    Function,
    /// Usable on its own as a statement, as well as in an expression.
    Statement,
}

/// Which dispatcher handles a builtin at run time.
///
/// This mirrors how the interpreter is organized rather than what a builtin is
/// *about*, so that routing is decided here instead of by a set of name lists
/// kept alongside each dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinGroup {
    /// Reads and queries over open files and the file system.
    File,
    /// Creates or removes directories and files.
    FileSystem,
    /// Date and time arithmetic.
    DateTime,
    /// Host dialogs, which may also be written as statements.
    Dialog,
    /// Everything routed by the general dispatcher.
    General,
}

/// How a builtin's arguments are checked.
///
/// Nearly every builtin accepts a simple range. `Pairs` exists for `Switch`,
/// whose arguments come in expression/value couples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arity {
    Range { min: usize, max: usize },
    Pairs,
}

impl Arity {
    /// Reports whether `count` arguments satisfy this arity.
    pub fn accepts(self, count: usize) -> bool {
        match self {
            Arity::Range { min, max } => count >= min && count <= max,
            Arity::Pairs => count > 0 && count.is_multiple_of(2),
        }
    }

    /// A phrase describing what was expected, for a diagnostic.
    pub fn describe(self) -> String {
        match self {
            Arity::Pairs => "expression/value pairs".to_string(),
            Arity::Range { min, max } if max == usize::MAX => {
                format!("at least {min} argument{}", plural(min))
            }
            Arity::Range { min, max } if min == max => {
                format!("exactly {min} argument{}", plural(min))
            }
            Arity::Range { min, max } => format!("{min} to {max} arguments"),
        }
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// A builtin's calling contract.
#[derive(Debug, Clone, Copy)]
pub struct Builtin {
    pub name: &'static str,
    pub arity: Arity,
    pub returns: BuiltinReturn,
    pub form: BuiltinForm,
    pub group: BuiltinGroup,
}

impl Builtin {
    /// Checks a call's argument count against this builtin's declared arity.
    ///
    /// The analyzer and the interpreter both call this, so a program is
    /// rejected for the same reason and with the same wording whether the
    /// mistake is caught before running or while running.
    pub fn check_arity(
        &self,
        count: usize,
        span: crate::runtime::Span,
    ) -> Result<(), crate::runtime::Diagnostic> {
        if self.arity.accepts(count) {
            return Ok(());
        }
        Err(crate::runtime::Diagnostic::new(
            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
            format!(
                "{} expects {}, found {}",
                self.name,
                self.arity.describe(),
                count
            ),
            Some(span),
        )
        .with_primary_label(format!("expected {}", self.arity.describe())))
    }
}

/// Declares a builtin that accepts between `min` and `max` arguments.
const fn f(name: &'static str, min: usize, max: usize, returns: BuiltinReturn) -> Builtin {
    Builtin {
        name,
        arity: Arity::Range { min, max },
        returns,
        form: BuiltinForm::Function,
        group: BuiltinGroup::General,
    }
}

/// Declares a builtin that can also stand alone as a statement.
const fn s(name: &'static str, min: usize, max: usize, returns: BuiltinReturn) -> Builtin {
    Builtin {
        name,
        arity: Arity::Range { min, max },
        returns,
        form: BuiltinForm::Statement,
        group: BuiltinGroup::General,
    }
}

/// Places a builtin in a dispatch group.
const fn grouped(builtin: Builtin, group: BuiltinGroup) -> Builtin {
    Builtin { group, ..builtin }
}

/// Every builtin the language provides.
///
/// Keep this grouped by area and alphabetical within a group; it is the list
/// people read to find out what exists.
pub const BUILTINS: &[Builtin] = &[
    // -- Math ------------------------------------------------------------
    f("Abs", 1, 1, BuiltinReturn::Double),
    f("Atn", 1, 1, BuiltinReturn::Double),
    f("Cos", 1, 1, BuiltinReturn::Double),
    f("Exp", 1, 1, BuiltinReturn::Double),
    f("Fix", 1, 1, BuiltinReturn::Integer),
    f("Int", 1, 1, BuiltinReturn::Integer),
    f("Log", 1, 1, BuiltinReturn::Double),
    s("Randomize", 0, 1, BuiltinReturn::Variant),
    f("Rnd", 0, 1, BuiltinReturn::Double),
    f("Round", 1, 2, BuiltinReturn::Double),
    f("Sgn", 1, 1, BuiltinReturn::Integer),
    f("Sin", 1, 1, BuiltinReturn::Double),
    f("Sqr", 1, 1, BuiltinReturn::Double),
    f("Tan", 1, 1, BuiltinReturn::Double),
    // -- Financial -------------------------------------------------------
    f("DDB", 1, usize::MAX, BuiltinReturn::Double),
    f("FV", 1, usize::MAX, BuiltinReturn::Double),
    f("IPmt", 1, usize::MAX, BuiltinReturn::Double),
    f("IRR", 1, usize::MAX, BuiltinReturn::Double),
    f("MIRR", 1, usize::MAX, BuiltinReturn::Double),
    f("NPer", 1, usize::MAX, BuiltinReturn::Double),
    f("NPV", 1, usize::MAX, BuiltinReturn::Double),
    f("PPmt", 1, usize::MAX, BuiltinReturn::Double),
    f("PV", 1, usize::MAX, BuiltinReturn::Double),
    f("Pmt", 1, usize::MAX, BuiltinReturn::Double),
    f("Rate", 1, usize::MAX, BuiltinReturn::Double),
    f("SLN", 1, usize::MAX, BuiltinReturn::Double),
    f("SYD", 1, usize::MAX, BuiltinReturn::Double),
    // -- Type inspection and conversion ----------------------------------
    f("CBool", 1, 1, BuiltinReturn::Boolean),
    f("CByte", 1, 1, BuiltinReturn::Byte),
    f("CCur", 1, 1, BuiltinReturn::Currency),
    f("CDate", 1, 1, BuiltinReturn::Date),
    f("CDbl", 1, 1, BuiltinReturn::Double),
    f("CDec", 1, 1, BuiltinReturn::Decimal),
    f("CInt", 1, 1, BuiltinReturn::Integer),
    f("CInt64", 1, 1, BuiltinReturn::Int64),
    f("CLng", 1, 1, BuiltinReturn::Long),
    f("CLngLng", 1, 1, BuiltinReturn::Int64),
    f("CLngPtr", 1, 1, BuiltinReturn::Int64),
    f("CSng", 1, 1, BuiltinReturn::Single),
    f("CStr", 1, 1, BuiltinReturn::String),
    f("CVErr", 1, 1, BuiltinReturn::Variant),
    f("CVar", 1, 1, BuiltinReturn::Variant),
    f("IsArray", 1, 1, BuiltinReturn::Boolean),
    f("IsDate", 1, 1, BuiltinReturn::Boolean),
    f("IsEmpty", 1, 1, BuiltinReturn::Boolean),
    f("IsError", 1, 1, BuiltinReturn::Boolean),
    f("IsMissing", 1, 1, BuiltinReturn::Boolean),
    f("IsNull", 1, 1, BuiltinReturn::Boolean),
    f("IsNumeric", 1, 1, BuiltinReturn::Boolean),
    f("IsObject", 1, 1, BuiltinReturn::Boolean),
    f("TypeName", 1, 1, BuiltinReturn::String),
    f("VarType", 1, 1, BuiltinReturn::Integer),
    // -- Strings ---------------------------------------------------------
    f("Asc", 1, 1, BuiltinReturn::Integer),
    f("AscW", 1, 1, BuiltinReturn::Integer),
    f("Chr", 1, 1, BuiltinReturn::String),
    f("ChrW", 1, 1, BuiltinReturn::String),
    f("Format", 1, 5, BuiltinReturn::String),
    f("FormatCurrency", 1, 5, BuiltinReturn::String),
    f("FormatDateTime", 1, 5, BuiltinReturn::String),
    f("FormatNumber", 1, 5, BuiltinReturn::String),
    f("FormatPercent", 1, 5, BuiltinReturn::String),
    f("Hex", 1, 1, BuiltinReturn::String),
    f("InStr", 2, 4, BuiltinReturn::Integer),
    f("InStrRev", 2, 4, BuiltinReturn::Integer),
    f("LCase", 1, 1, BuiltinReturn::String),
    f("LTrim", 1, 1, BuiltinReturn::String),
    f("Left", 2, 2, BuiltinReturn::String),
    f("Len", 1, 1, BuiltinReturn::Integer),
    f("LenB", 1, 1, BuiltinReturn::Integer),
    f("Mid", 2, 3, BuiltinReturn::String),
    f("Oct", 1, 1, BuiltinReturn::String),
    f("Partition", 4, 4, BuiltinReturn::String),
    f("RTrim", 1, 1, BuiltinReturn::String),
    f("Replace", 3, 6, BuiltinReturn::String),
    f("Right", 2, 2, BuiltinReturn::String),
    f("Space", 1, 1, BuiltinReturn::String),
    f("Str", 1, 1, BuiltinReturn::String),
    f("StrComp", 2, 3, BuiltinReturn::Integer),
    f("StrConv", 2, 3, BuiltinReturn::String),
    f("StrReverse", 1, 1, BuiltinReturn::String),
    f("String", 2, 2, BuiltinReturn::String),
    f("Trim", 1, 1, BuiltinReturn::String),
    f("UCase", 1, 1, BuiltinReturn::String),
    f("Val", 1, 1, BuiltinReturn::Double),
    // -- Arrays and collections ------------------------------------------
    f("Array", 0, usize::MAX, BuiltinReturn::Variant),
    f("Filter", 2, 4, BuiltinReturn::Variant),
    f("Join", 1, 2, BuiltinReturn::String),
    f("LBound", 1, 2, BuiltinReturn::Integer),
    f("Split", 1, 4, BuiltinReturn::StringArray),
    f("UBound", 1, 2, BuiltinReturn::Integer),
    // -- Selection -------------------------------------------------------
    f("Choose", 2, usize::MAX, BuiltinReturn::Variant),
    f("IIf", 3, 3, BuiltinReturn::Variant),
    Builtin {
        name: "Switch",
        arity: Arity::Pairs,
        returns: BuiltinReturn::Variant,
        form: BuiltinForm::Function,
        group: BuiltinGroup::General,
    },
    // -- Date and time ---------------------------------------------------
    grouped(f("Date", 0, 0, BuiltinReturn::Date), BuiltinGroup::DateTime),
    grouped(
        f("DateAdd", 3, 3, BuiltinReturn::Date),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("DateDiff", 3, 5, BuiltinReturn::Integer),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("DatePart", 2, 4, BuiltinReturn::Integer),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("DateSerial", 3, 3, BuiltinReturn::Date),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("DateValue", 1, 1, BuiltinReturn::Date),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("Day", 1, 1, BuiltinReturn::Integer),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("Hour", 1, 1, BuiltinReturn::Integer),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("Minute", 1, 1, BuiltinReturn::Integer),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("Month", 1, 1, BuiltinReturn::Integer),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("MonthName", 1, 2, BuiltinReturn::String),
        BuiltinGroup::DateTime,
    ),
    grouped(f("Now", 0, 0, BuiltinReturn::Date), BuiltinGroup::DateTime),
    grouped(
        f("Second", 1, 1, BuiltinReturn::Integer),
        BuiltinGroup::DateTime,
    ),
    grouped(f("Time", 0, 0, BuiltinReturn::Date), BuiltinGroup::DateTime),
    grouped(
        f("TimeSerial", 3, 3, BuiltinReturn::Date),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("TimeValue", 1, 1, BuiltinReturn::Date),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("Timer", 0, 0, BuiltinReturn::Double),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("Weekday", 1, 2, BuiltinReturn::Integer),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("WeekdayName", 1, 3, BuiltinReturn::String),
        BuiltinGroup::DateTime,
    ),
    grouped(
        f("Year", 1, 1, BuiltinReturn::Integer),
        BuiltinGroup::DateTime,
    ),
    // -- Files and the environment ---------------------------------------
    grouped(
        s("ChDir", 1, 1, BuiltinReturn::Variant),
        BuiltinGroup::FileSystem,
    ),
    grouped(f("CurDir", 0, 1, BuiltinReturn::String), BuiltinGroup::File),
    grouped(f("Dir", 0, 2, BuiltinReturn::String), BuiltinGroup::File),
    grouped(f("EOF", 1, 1, BuiltinReturn::Boolean), BuiltinGroup::File),
    f("Environ", 1, 1, BuiltinReturn::String),
    grouped(
        f("FileAttr", 2, 2, BuiltinReturn::Integer),
        BuiltinGroup::File,
    ),
    grouped(
        f("FileDateTime", 1, 1, BuiltinReturn::Date),
        BuiltinGroup::File,
    ),
    grouped(
        f("FileLen", 1, 1, BuiltinReturn::Integer),
        BuiltinGroup::File,
    ),
    grouped(
        f("FreeFile", 0, 1, BuiltinReturn::Integer),
        BuiltinGroup::File,
    ),
    grouped(
        f("GetAttr", 1, 1, BuiltinReturn::Integer),
        BuiltinGroup::File,
    ),
    f("Input", 2, 2, BuiltinReturn::String),
    grouped(
        s("Kill", 1, 1, BuiltinReturn::Variant),
        BuiltinGroup::FileSystem,
    ),
    grouped(f("LOF", 1, 1, BuiltinReturn::Integer), BuiltinGroup::File),
    grouped(f("Loc", 1, 1, BuiltinReturn::Integer), BuiltinGroup::File),
    grouped(
        s("MkDir", 1, 1, BuiltinReturn::Variant),
        BuiltinGroup::FileSystem,
    ),
    grouped(
        s("RmDir", 1, 1, BuiltinReturn::Variant),
        BuiltinGroup::FileSystem,
    ),
    grouped(f("Seek", 1, 1, BuiltinReturn::Integer), BuiltinGroup::File),
    // -- Host and interop -------------------------------------------------
    s("CallByName", 3, usize::MAX, BuiltinReturn::Variant),
    f("Command", 0, 0, BuiltinReturn::String),
    f("Error", 0, 1, BuiltinReturn::String),
    f("GetAllSettings", 2, 2, BuiltinReturn::Variant),
    f("GetSetting", 3, 4, BuiltinReturn::String),
    f("IMEStatus", 0, 0, BuiltinReturn::Integer),
    grouped(
        f("InputBox", 1, 7, BuiltinReturn::String),
        BuiltinGroup::Dialog,
    ),
    f("MacID", 1, 1, BuiltinReturn::Long),
    f("MacScript", 1, 1, BuiltinReturn::String),
    grouped(
        f("MsgBox", 1, 5, BuiltinReturn::Integer),
        BuiltinGroup::Dialog,
    ),
    f("ObjPtr", 1, 1, BuiltinReturn::Ptr),
    f("QBColor", 1, 1, BuiltinReturn::Long),
    f("RGB", 3, 3, BuiltinReturn::Long),
    f("Shell", 1, 2, BuiltinReturn::Long),
    f("Spc", 1, 1, BuiltinReturn::String),
    f("StrPtr", 1, 1, BuiltinReturn::Ptr),
    f("Tab", 1, 1, BuiltinReturn::String),
    f("VarPtr", 1, 1, BuiltinReturn::Ptr),
];

/// Strips the optional `VBA.` qualifier a builtin may be called through.
pub fn strip_vba_namespace(name: &str) -> &str {
    name.strip_prefix("VBA.").unwrap_or(name)
}

pub fn is_name_in(name: &str, names: &[&str]) -> bool {
    names
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

/// Looks up a builtin by the name it is called with, ignoring case and an
/// optional `VBA.` qualifier.
/// The registry keyed by folded name, built once.
///
/// The table is written in reading order and grouped by what each builtin is
/// for, which is how it should stay. Searching it in that order is another
/// matter: `lookup` is asked about every call in a program, including the ones
/// that turn out to be user procedures, and scanning a hundred and fifty names
/// to answer "no" was most of what a call cost.
fn by_name() -> &'static std::collections::HashMap<String, &'static Builtin> {
    static BY_NAME: std::sync::OnceLock<std::collections::HashMap<String, &'static Builtin>> =
        std::sync::OnceLock::new();
    BY_NAME.get_or_init(|| {
        BUILTINS
            .iter()
            .map(|builtin| (crate::runtime::fold(builtin.name), builtin))
            .collect()
    })
}

pub fn lookup(name: &str) -> Option<&'static Builtin> {
    let name = strip_vba_namespace(name);
    crate::runtime::with_folded(name, |folded| by_name().get(folded).copied())
}

pub fn is_builtin_function(name: &str) -> bool {
    lookup(name).is_some()
}

/// Reports whether a builtin may be written as a statement of its own.
pub fn is_builtin_statement(name: &str) -> bool {
    lookup(name).is_some_and(|builtin| builtin.form == BuiltinForm::Statement)
}

/// Reports whether a builtin belongs to the given dispatch group.
pub fn is_in_group(name: &str, group: BuiltinGroup) -> bool {
    lookup(name).is_some_and(|builtin| builtin.group == group)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_builtin_is_declared_twice() {
        let mut seen = HashSet::new();
        for builtin in BUILTINS {
            let name = builtin.name.to_lowercase();
            assert!(
                seen.insert(name),
                "builtin '{}' is declared more than once",
                builtin.name
            );
        }
    }

    #[test]
    fn every_builtin_has_a_reachable_arity() {
        for builtin in BUILTINS {
            if let Arity::Range { min, max } = builtin.arity {
                assert!(
                    min <= max,
                    "builtin '{}' declares an empty arity range",
                    builtin.name
                );
            }
        }
    }

    #[test]
    fn lookup_ignores_case_and_the_vba_qualifier() {
        assert!(lookup("ucase").is_some());
        assert!(lookup("UCase").is_some());
        assert!(lookup("VBA.UCase").is_some());
        assert!(lookup("NotABuiltin").is_none());
    }

    #[test]
    fn arity_describes_what_it_accepts() {
        assert!(Arity::Range { min: 1, max: 1 }.accepts(1));
        assert!(!Arity::Range { min: 1, max: 1 }.accepts(2));
        assert!(
            Arity::Range {
                min: 2,
                max: usize::MAX
            }
            .accepts(9)
        );
        assert!(Arity::Pairs.accepts(4));
        assert!(!Arity::Pairs.accepts(3));
        assert!(!Arity::Pairs.accepts(0));
    }
}
