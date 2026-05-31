pub const DIALOG_STATEMENT_FUNCTIONS: &[&str] = &["MsgBox", "InputBox", "DoEvents"];

pub const FILE_FUNCTIONS: &[&str] = &[
    "FreeFile",
    "EOF",
    "LOF",
    "Seek",
    "Dir",
    "FileLen",
    "FileDateTime",
    "CurDir",
];

pub const DATE_TIME_FUNCTIONS: &[&str] = &[
    "Timer",
    "Now",
    "Date",
    "Time",
    "DateSerial",
    "TimeSerial",
    "DateValue",
    "TimeValue",
    "Year",
    "Month",
    "Day",
    "Hour",
    "Minute",
    "Second",
    "Weekday",
    "MonthName",
    "WeekdayName",
];

pub const FILE_SYSTEM_STATEMENT_FUNCTIONS: &[&str] = &["Kill", "MkDir", "RmDir", "ChDir"];

pub const BUILTIN_STATEMENT_FUNCTIONS: &[&str] =
    &["Randomize", "CallByName", "Kill", "MkDir", "RmDir", "ChDir"];

pub const STRING_ONE_ARG_FUNCTIONS: &[&str] = &[
    "Trim", "LTrim", "RTrim", "UCase", "LCase", "Chr", "ChrW", "Str", "Hex", "Oct", "Val",
];

pub const STRING_FIXED_ARG_FUNCTIONS: &[&str] = &["Left", "Right", "Space", "String"];

pub const BOOLEAN_ONE_ARG_FUNCTIONS: &[&str] = &[
    "IsObject",
    "IsNull",
    "IsError",
    "IsEmpty",
    "IsNumeric",
    "IsDate",
];

pub const INTEGER_ONE_ARG_FUNCTIONS: &[&str] = &[
    "VarType", "Sgn", "Int", "Len", "LenB", "Asc", "AscW", "LOF", "Seek", "FileLen", "Year",
    "Month", "Day", "Hour", "Minute", "Second",
];

pub const BUILTIN_FUNCTIONS: &[&str] = &[
    "Sgn",
    "Int",
    "Randomize",
    "Rnd",
    "Split",
    "Join",
    "Filter",
    "CStr",
    "StrComp",
    "IsObject",
    "IsArray",
    "IsNumeric",
    "IsDate",
    "IsNull",
    "IsEmpty",
    "IsError",
    "VarType",
    "TypeName",
    "CreateObject",
    "CByte",
    "CInt",
    "CLng",
    "CLngLng",
    "CInt64",
    "CSng",
    "CDbl",
    "CDec",
    "CCur",
    "CDate",
    "CBool",
    "Array",
    "LBound",
    "UBound",
    "Len",
    "LenB",
    "Left",
    "Right",
    "Mid",
    "Trim",
    "LTrim",
    "RTrim",
    "UCase",
    "LCase",
    "Replace",
    "InStr",
    "InStrRev",
    "Space",
    "String",
    "Chr",
    "ChrW",
    "Asc",
    "AscW",
    "Val",
    "Str",
    "Hex",
    "Oct",
    "FreeFile",
    "EOF",
    "LOF",
    "Seek",
    "Dir",
    "FileLen",
    "FileDateTime",
    "CurDir",
    "Environ",
    "Timer",
    "Now",
    "Date",
    "Time",
    "DateSerial",
    "TimeSerial",
    "DateValue",
    "TimeValue",
    "Year",
    "Month",
    "Day",
    "Hour",
    "Minute",
    "Second",
    "Weekday",
    "MonthName",
    "WeekdayName",
    "Kill",
    "MkDir",
    "RmDir",
    "ChDir",
    "IsMissing",
];

pub fn strip_vba_namespace(name: &str) -> &str {
    name.strip_prefix("VBA.").unwrap_or(name)
}

pub fn is_name_in(name: &str, names: &[&str]) -> bool {
    names
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

pub fn is_builtin_function(name: &str) -> bool {
    is_name_in(strip_vba_namespace(name), BUILTIN_FUNCTIONS)
}
