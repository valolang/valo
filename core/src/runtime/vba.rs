use super::{TypeName, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VbaConstantValue {
    Integer(i64),
    String(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VbaConstant {
    pub name: &'static str,
    pub value: VbaConstantValue,
}

impl VbaConstant {
    pub fn type_name(self) -> TypeName {
        match self.value {
            VbaConstantValue::Integer(_) => TypeName::Integer,
            VbaConstantValue::String(_) => TypeName::String,
        }
    }

    pub fn value(self) -> Value {
        match self.value {
            VbaConstantValue::Integer(value) => Value::Int64(value),
            VbaConstantValue::String(value) => Value::String(value.to_string()),
        }
    }
}

pub const VBA_CONSTANTS: &[VbaConstant] = &[
    VbaConstant {
        name: "vbNullString",
        value: VbaConstantValue::String(""),
    },
    VbaConstant {
        name: "vbCr",
        value: VbaConstantValue::String("\r"),
    },
    VbaConstant {
        name: "vbLf",
        value: VbaConstantValue::String("\n"),
    },
    VbaConstant {
        name: "vbCrLf",
        value: VbaConstantValue::String("\r\n"),
    },
    VbaConstant {
        name: "vbNewLine",
        value: VbaConstantValue::String("\r\n"),
    },
    VbaConstant {
        name: "vbTab",
        value: VbaConstantValue::String("\t"),
    },
    VbaConstant {
        name: "vbBack",
        value: VbaConstantValue::String("\x08"),
    },
    VbaConstant {
        name: "vbFormFeed",
        value: VbaConstantValue::String("\x0c"),
    },
    VbaConstant {
        name: "vbVerticalTab",
        value: VbaConstantValue::String("\x0b"),
    },
    VbaConstant {
        name: "vbNullChar",
        value: VbaConstantValue::String("\0"),
    },
    VbaConstant {
        name: "vbTrue",
        value: VbaConstantValue::Integer(-1),
    },
    VbaConstant {
        name: "vbFalse",
        value: VbaConstantValue::Integer(0),
    },
    VbaConstant {
        name: "vbUseDefault",
        value: VbaConstantValue::Integer(-2),
    },
    VbaConstant {
        name: "vbObjectError",
        value: VbaConstantValue::Integer(-2147221504),
    },
    VbaConstant {
        name: "vbUseCompareOption",
        value: VbaConstantValue::Integer(-1),
    },
    VbaConstant {
        name: "vbBinaryCompare",
        value: VbaConstantValue::Integer(0),
    },
    VbaConstant {
        name: "vbTextCompare",
        value: VbaConstantValue::Integer(1),
    },
    VbaConstant {
        name: "vbDatabaseCompare",
        value: VbaConstantValue::Integer(2),
    },
    VbaConstant {
        name: "vbGeneralDate",
        value: VbaConstantValue::Integer(0),
    },
    VbaConstant {
        name: "vbLongDate",
        value: VbaConstantValue::Integer(1),
    },
    VbaConstant {
        name: "vbShortDate",
        value: VbaConstantValue::Integer(2),
    },
    VbaConstant {
        name: "vbLongTime",
        value: VbaConstantValue::Integer(3),
    },
    VbaConstant {
        name: "vbShortTime",
        value: VbaConstantValue::Integer(4),
    },
    VbaConstant {
        name: "vbUseSystem",
        value: VbaConstantValue::Integer(0),
    },
    VbaConstant {
        name: "vbUseSystemDayOfWeek",
        value: VbaConstantValue::Integer(0),
    },
    VbaConstant {
        name: "vbSunday",
        value: VbaConstantValue::Integer(1),
    },
    VbaConstant {
        name: "vbMonday",
        value: VbaConstantValue::Integer(2),
    },
    VbaConstant {
        name: "vbTuesday",
        value: VbaConstantValue::Integer(3),
    },
    VbaConstant {
        name: "vbWednesday",
        value: VbaConstantValue::Integer(4),
    },
    VbaConstant {
        name: "vbThursday",
        value: VbaConstantValue::Integer(5),
    },
    VbaConstant {
        name: "vbFriday",
        value: VbaConstantValue::Integer(6),
    },
    VbaConstant {
        name: "vbSaturday",
        value: VbaConstantValue::Integer(7),
    },
    VbaConstant {
        name: "vbFirstJan1",
        value: VbaConstantValue::Integer(1),
    },
    VbaConstant {
        name: "vbFirstFourDays",
        value: VbaConstantValue::Integer(2),
    },
    VbaConstant {
        name: "vbFirstFullWeek",
        value: VbaConstantValue::Integer(3),
    },
    VbaConstant {
        name: "vbOKOnly",
        value: VbaConstantValue::Integer(0),
    },
    VbaConstant {
        name: "vbOKCancel",
        value: VbaConstantValue::Integer(1),
    },
    VbaConstant {
        name: "vbAbortRetryIgnore",
        value: VbaConstantValue::Integer(2),
    },
    VbaConstant {
        name: "vbYesNoCancel",
        value: VbaConstantValue::Integer(3),
    },
    VbaConstant {
        name: "vbYesNo",
        value: VbaConstantValue::Integer(4),
    },
    VbaConstant {
        name: "vbRetryCancel",
        value: VbaConstantValue::Integer(5),
    },
    VbaConstant {
        name: "vbCritical",
        value: VbaConstantValue::Integer(16),
    },
    VbaConstant {
        name: "vbQuestion",
        value: VbaConstantValue::Integer(32),
    },
    VbaConstant {
        name: "vbExclamation",
        value: VbaConstantValue::Integer(48),
    },
    VbaConstant {
        name: "vbInformation",
        value: VbaConstantValue::Integer(64),
    },
    VbaConstant {
        name: "vbDefaultButton1",
        value: VbaConstantValue::Integer(0),
    },
    VbaConstant {
        name: "vbDefaultButton2",
        value: VbaConstantValue::Integer(256),
    },
    VbaConstant {
        name: "vbDefaultButton3",
        value: VbaConstantValue::Integer(512),
    },
    VbaConstant {
        name: "vbDefaultButton4",
        value: VbaConstantValue::Integer(768),
    },
    VbaConstant {
        name: "vbApplicationModal",
        value: VbaConstantValue::Integer(0),
    },
    VbaConstant {
        name: "vbSystemModal",
        value: VbaConstantValue::Integer(4096),
    },
    VbaConstant {
        name: "vbMsgBoxHelpButton",
        value: VbaConstantValue::Integer(16384),
    },
    VbaConstant {
        name: "vbMsgBoxSetForeground",
        value: VbaConstantValue::Integer(65536),
    },
    VbaConstant {
        name: "vbMsgBoxRight",
        value: VbaConstantValue::Integer(524288),
    },
    VbaConstant {
        name: "vbMsgBoxRtlReading",
        value: VbaConstantValue::Integer(1048576),
    },
    VbaConstant {
        name: "vbOK",
        value: VbaConstantValue::Integer(1),
    },
    VbaConstant {
        name: "vbCancel",
        value: VbaConstantValue::Integer(2),
    },
    VbaConstant {
        name: "vbAbort",
        value: VbaConstantValue::Integer(3),
    },
    VbaConstant {
        name: "vbRetry",
        value: VbaConstantValue::Integer(4),
    },
    VbaConstant {
        name: "vbIgnore",
        value: VbaConstantValue::Integer(5),
    },
    VbaConstant {
        name: "vbYes",
        value: VbaConstantValue::Integer(6),
    },
    VbaConstant {
        name: "vbNo",
        value: VbaConstantValue::Integer(7),
    },
    VbaConstant {
        name: "vbNormal",
        value: VbaConstantValue::Integer(0),
    },
    VbaConstant {
        name: "vbReadOnly",
        value: VbaConstantValue::Integer(1),
    },
    VbaConstant {
        name: "vbHidden",
        value: VbaConstantValue::Integer(2),
    },
    VbaConstant {
        name: "vbSystem",
        value: VbaConstantValue::Integer(4),
    },
    VbaConstant {
        name: "vbVolume",
        value: VbaConstantValue::Integer(8),
    },
    VbaConstant {
        name: "vbDirectory",
        value: VbaConstantValue::Integer(16),
    },
    VbaConstant {
        name: "vbArchive",
        value: VbaConstantValue::Integer(32),
    },
    VbaConstant {
        name: "vbAlias",
        value: VbaConstantValue::Integer(64),
    },
    VbaConstant {
        name: "vbUpperCase",
        value: VbaConstantValue::Integer(1),
    },
    VbaConstant {
        name: "vbLowerCase",
        value: VbaConstantValue::Integer(2),
    },
    VbaConstant {
        name: "vbProperCase",
        value: VbaConstantValue::Integer(3),
    },
    VbaConstant {
        name: "vbWide",
        value: VbaConstantValue::Integer(4),
    },
    VbaConstant {
        name: "vbNarrow",
        value: VbaConstantValue::Integer(8),
    },
    VbaConstant {
        name: "vbKatakana",
        value: VbaConstantValue::Integer(16),
    },
    VbaConstant {
        name: "vbHiragana",
        value: VbaConstantValue::Integer(32),
    },
    VbaConstant {
        name: "vbUnicode",
        value: VbaConstantValue::Integer(64),
    },
    VbaConstant {
        name: "vbFromUnicode",
        value: VbaConstantValue::Integer(128),
    },
    VbaConstant {
        name: "vbHide",
        value: VbaConstantValue::Integer(0),
    },
    VbaConstant {
        name: "vbNormalFocus",
        value: VbaConstantValue::Integer(1),
    },
    VbaConstant {
        name: "vbMinimizedFocus",
        value: VbaConstantValue::Integer(2),
    },
    VbaConstant {
        name: "vbMaximizedFocus",
        value: VbaConstantValue::Integer(3),
    },
    VbaConstant {
        name: "vbNormalNoFocus",
        value: VbaConstantValue::Integer(4),
    },
    VbaConstant {
        name: "vbMinimizedNoFocus",
        value: VbaConstantValue::Integer(6),
    },
    VbaConstant {
        name: "vbEmpty",
        value: VbaConstantValue::Integer(0),
    },
    VbaConstant {
        name: "vbNull",
        value: VbaConstantValue::Integer(1),
    },
    VbaConstant {
        name: "vbInteger",
        value: VbaConstantValue::Integer(2),
    },
    VbaConstant {
        name: "vbLong",
        value: VbaConstantValue::Integer(3),
    },
    VbaConstant {
        name: "vbSingle",
        value: VbaConstantValue::Integer(4),
    },
    VbaConstant {
        name: "vbDouble",
        value: VbaConstantValue::Integer(5),
    },
    VbaConstant {
        name: "vbCurrency",
        value: VbaConstantValue::Integer(6),
    },
    VbaConstant {
        name: "vbDate",
        value: VbaConstantValue::Integer(7),
    },
    VbaConstant {
        name: "vbString",
        value: VbaConstantValue::Integer(8),
    },
    VbaConstant {
        name: "vbObject",
        value: VbaConstantValue::Integer(9),
    },
    VbaConstant {
        name: "vbError",
        value: VbaConstantValue::Integer(10),
    },
    VbaConstant {
        name: "vbBoolean",
        value: VbaConstantValue::Integer(11),
    },
    VbaConstant {
        name: "vbVariant",
        value: VbaConstantValue::Integer(12),
    },
    VbaConstant {
        name: "vbDataObject",
        value: VbaConstantValue::Integer(13),
    },
    VbaConstant {
        name: "vbDecimal",
        value: VbaConstantValue::Integer(14),
    },
    VbaConstant {
        name: "vbByte",
        value: VbaConstantValue::Integer(17),
    },
    VbaConstant {
        name: "vbLongLong",
        value: VbaConstantValue::Integer(20),
    },
    VbaConstant {
        name: "vbLongPtr",
        value: VbaConstantValue::Integer(26),
    },
    VbaConstant {
        name: "vbUserDefinedType",
        value: VbaConstantValue::Integer(36),
    },
    VbaConstant {
        name: "vbArray",
        value: VbaConstantValue::Integer(8192),
    },
    VbaConstant {
        name: "VbMethod",
        value: VbaConstantValue::Integer(1),
    },
    VbaConstant {
        name: "VbGet",
        value: VbaConstantValue::Integer(2),
    },
    VbaConstant {
        name: "VbLet",
        value: VbaConstantValue::Integer(4),
    },
    VbaConstant {
        name: "VbSet",
        value: VbaConstantValue::Integer(8),
    },
];

pub fn vba_constant(name: &str) -> Option<VbaConstant> {
    VBA_CONSTANTS
        .iter()
        .copied()
        .find(|constant| constant.name.eq_ignore_ascii_case(name))
}
