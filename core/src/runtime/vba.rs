use super::{TypeName, Value};
use std::rc::Rc;

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
            VbaConstantValue::Integer(_) => TypeName::Int16,
            VbaConstantValue::String(_) => TypeName::String,
        }
    }

    pub fn value(self) -> Value {
        match self.value {
            VbaConstantValue::Integer(value) => Value::Int64(value),
            VbaConstantValue::String(value) => Value::String(Rc::new(value.to_string())),
        }
    }
}

const fn int(name: &'static str, value: i64) -> VbaConstant {
    VbaConstant {
        name,
        value: VbaConstantValue::Integer(value),
    }
}

const fn string(name: &'static str, value: &'static str) -> VbaConstant {
    VbaConstant {
        name,
        value: VbaConstantValue::String(value),
    }
}

pub const VBA_CONSTANTS: &[VbaConstant] = &[
    // Miscellaneous
    string("vbCrLf", "\r\n"),
    string("vbCr", "\r"),
    string("vbLf", "\n"),
    string("vbNewLine", "\r\n"),
    string("vbNullChar", "\0"),
    string("vbNullString", ""),
    int("vbObjectError", -2147221504),
    string("vbTab", "\t"),
    string("vbBack", "\x08"),
    string("vbFormFeed", "\x0c"),
    string("vbVerticalTab", "\x0b"),
    // Calendar
    int("vbCalGreg", 0),
    int("vbCalHijri", 1),
    // CallType
    int("vbMethod", 1),
    int("vbGet", 2),
    int("vbLet", 4),
    int("vbSet", 8),
    // Color
    int("vbBlack", 0x0),
    int("vbRed", 0xFF),
    int("vbGreen", 0xFF00),
    int("vbYellow", 0xFFFF),
    int("vbBlue", 0xFF0000),
    int("vbMagenta", 0xFF00FF),
    int("vbCyan", 0xFFFF00),
    int("vbWhite", 0xFFFFFF),
    // Comparison
    int("vbUseCompareOption", -1),
    int("vbBinaryCompare", 0),
    int("vbTextCompare", 1),
    int("vbDatabaseCompare", 2),
    // Date and Date Format
    int("vbGeneralDate", 0),
    int("vbLongDate", 1),
    int("vbShortDate", 2),
    int("vbLongTime", 3),
    int("vbShortTime", 4),
    int("vbUseSystem", 0),
    int("vbUseSystemDayOfWeek", 0),
    int("vbSunday", 1),
    int("vbMonday", 2),
    int("vbTuesday", 3),
    int("vbWednesday", 4),
    int("vbThursday", 5),
    int("vbFriday", 6),
    int("vbSaturday", 7),
    int("vbFirstJan1", 1),
    int("vbFirstFourDays", 2),
    int("vbFirstFullWeek", 3),
    // Dir, GetAttr, and SetAttr
    int("vbNormal", 0),
    int("vbReadOnly", 1),
    int("vbHidden", 2),
    int("vbSystem", 4),
    int("vbVolume", 8),
    int("vbDirectory", 16),
    int("vbArchive", 32),
    int("vbAlias", 64),
    // DriveType
    int("Unknown", 0),
    int("Removable", 1),
    int("Fixed", 2),
    int("Remote", 3),
    int("CDROM", 4),
    int("RAMDisk", 5),
    // File Attribute
    int("Normal", 0),
    int("ReadOnly", 1),
    int("Hidden", 2),
    int("System", 4),
    int("Volume", 8),
    int("Directory", 16),
    int("Archive", 32),
    int("Alias", 64),
    int("Compressed", 128),
    // File Input/Output
    int("ForReading", 1),
    int("ForWriting", 2),
    int("ForAppending", 8),
    // Form
    int("vbModeless", 0),
    int("vbModal", 1),
    // IMEStatus
    int("vbIMEModeNoControl", 0),
    int("vbIMEModeOn", 1),
    int("vbIMEModeOff", 2),
    int("vbIMEModeDisable", 3),
    int("vbIMEModeHiragana", 4),
    int("vbIMEModeKatakana", 5),
    int("vbIMEModeKatakanaHalf", 6),
    int("vbIMEModeAlphaFull", 7),
    int("vbIMEModeAlpha", 8),
    int("vbIMEModeHangulFull", 9),
    int("vbIMEModeHangul", 10),
    // Keycode
    int("vbKeyLButton", 0x1),
    int("vbKeyRButton", 0x2),
    int("vbKeyCancel", 0x3),
    int("vbKeyMButton", 0x4),
    int("vbKeyBack", 0x8),
    int("vbKeyTab", 0x9),
    int("vbKeyClear", 0xC),
    int("vbKeyReturn", 0xD),
    int("vbKeyShift", 0x10),
    int("vbKeyControl", 0x11),
    int("vbKeyMenu", 0x12),
    int("vbKeyPause", 0x13),
    int("vbKeyCapital", 0x14),
    int("vbKeyEscape", 0x1B),
    int("vbKeySpace", 0x20),
    int("vbKeyPageUp", 0x21),
    int("vbKeyPageDown", 0x22),
    int("vbKeyEnd", 0x23),
    int("vbKeyHome", 0x24),
    int("vbKeyLeft", 0x25),
    int("vbKeyUp", 0x26),
    int("vbKeyRight", 0x27),
    int("vbKeyDown", 0x28),
    int("vbKeySelect", 0x29),
    int("vbKeyPrint", 0x2A),
    int("vbKeyExecute", 0x2B),
    int("vbKeySnapshot", 0x2C),
    int("vbKeyInsert", 0x2D),
    int("vbKeyDelete", 0x2E),
    int("vbKeyHelp", 0x2F),
    int("vbKeyNumlock", 0x90),
    int("vbKeyA", 65),
    int("vbKeyB", 66),
    int("vbKeyC", 67),
    int("vbKeyD", 68),
    int("vbKeyE", 69),
    int("vbKeyF", 70),
    int("vbKeyG", 71),
    int("vbKeyH", 72),
    int("vbKeyI", 73),
    int("vbKeyJ", 74),
    int("vbKeyK", 75),
    int("vbKeyL", 76),
    int("vbKeyM", 77),
    int("vbKeyN", 78),
    int("vbKeyO", 79),
    int("vbKeyP", 80),
    int("vbKeyQ", 81),
    int("vbKeyR", 82),
    int("vbKeyS", 83),
    int("vbKeyT", 84),
    int("vbKeyU", 85),
    int("vbKeyV", 86),
    int("vbKeyW", 87),
    int("vbKeyX", 88),
    int("vbKeyY", 89),
    int("vbKeyZ", 90),
    int("vbKey0", 48),
    int("vbKey1", 49),
    int("vbKey2", 50),
    int("vbKey3", 51),
    int("vbKey4", 52),
    int("vbKey5", 53),
    int("vbKey6", 54),
    int("vbKey7", 55),
    int("vbKey8", 56),
    int("vbKey9", 57),
    int("vbKeyNumpad0", 0x60),
    int("vbKeyNumpad1", 0x61),
    int("vbKeyNumpad2", 0x62),
    int("vbKeyNumpad3", 0x63),
    int("vbKeyNumpad4", 0x64),
    int("vbKeyNumpad5", 0x65),
    int("vbKeyNumpad6", 0x66),
    int("vbKeyNumpad7", 0x67),
    int("vbKeyNumpad8", 0x68),
    int("vbKeyNumpad9", 0x69),
    int("vbKeyMultiply", 0x6A),
    int("vbKeyAdd", 0x6B),
    int("vbKeySeparator", 0x6C),
    int("vbKeySubtract", 0x6D),
    int("vbKeyDecimal", 0x6E),
    int("vbKeyDivide", 0x6F),
    int("vbKeyF1", 0x70),
    int("vbKeyF2", 0x71),
    int("vbKeyF3", 0x72),
    int("vbKeyF4", 0x73),
    int("vbKeyF5", 0x74),
    int("vbKeyF6", 0x75),
    int("vbKeyF7", 0x76),
    int("vbKeyF8", 0x77),
    int("vbKeyF9", 0x78),
    int("vbKeyF10", 0x79),
    int("vbKeyF11", 0x7A),
    int("vbKeyF12", 0x7B),
    int("vbKeyF13", 0x7C),
    int("vbKeyF14", 0x7D),
    int("vbKeyF15", 0x7E),
    int("vbKeyF16", 0x7F),
    // MsgBox
    int("vbOKOnly", 0),
    int("vbOKCancel", 1),
    int("vbAbortRetryIgnore", 2),
    int("vbYesNoCancel", 3),
    int("vbYesNo", 4),
    int("vbRetryCancel", 5),
    int("vbCritical", 16),
    int("vbQuestion", 32),
    int("vbExclamation", 48),
    int("vbInformation", 64),
    int("vbDefaultButton1", 0),
    int("vbDefaultButton2", 256),
    int("vbDefaultButton3", 512),
    int("vbDefaultButton4", 768),
    int("vbApplicationModal", 0),
    int("vbSystemModal", 4096),
    int("vbMsgBoxHelpButton", 16384),
    int("vbMsgBoxSetForeground", 65536),
    int("vbMsgBoxRight", 524288),
    int("vbMsgBoxRtlReading", 1048576),
    int("vbOK", 1),
    int("vbCancel", 2),
    int("vbAbort", 3),
    int("vbRetry", 4),
    int("vbIgnore", 5),
    int("vbYes", 6),
    int("vbNo", 7),
    // QueryClose
    int("vbFormControlMenu", 0),
    int("vbFormCode", 1),
    int("vbAppWindows", 2),
    int("vbAppTaskManager", 3),
    // Shell
    int("vbHide", 0),
    int("vbNormalFocus", 1),
    int("vbMinimizedFocus", 2),
    int("vbMaximizedFocus", 3),
    int("vbNormalNoFocus", 4),
    int("vbMinimizedNoFocus", 6),
    // SpecialFolder
    int("WindowsFolder", 0),
    int("SystemFolder", 1),
    int("TemporaryFolder", 2),
    // StrConv
    int("vbUpperCase", 1),
    int("vbLowerCase", 2),
    int("vbProperCase", 3),
    int("vbWide", 4),
    int("vbNarrow", 8),
    int("vbKatakana", 16),
    int("vbHiragana", 32),
    int("vbUnicode", 64),
    int("vbFromUnicode", 128),
    // System Color
    int("vbScrollBars", -2147483648),
    int("vbDesktop", -2147483647),
    int("vbActiveTitleBar", -2147483646),
    int("vbInactiveTitleBar", -2147483645),
    int("vbMenuBar", -2147483644),
    int("vbWindowBackground", -2147483643),
    int("vbWindowFrame", -2147483642),
    int("vbMenuText", -2147483641),
    int("vbWindowText", -2147483640),
    int("vbTitleBarText", -2147483639),
    int("vbActiveBorder", -2147483638),
    int("vbInactiveBorder", -2147483637),
    int("vbApplicationWorkspace", -2147483636),
    int("vbHighlight", -2147483635),
    int("vbHighlightText", -2147483634),
    int("vbButtonFace", -2147483633),
    int("vbButtonShadow", -2147483632),
    int("vbGrayText", -2147483631),
    int("vbButtonText", -2147483630),
    int("vbInactiveCaptionText", -2147483629),
    int("vb3DHighlight", -2147483628),
    int("vb3DDKShadow", -2147483627),
    int("vb3DLight", -2147483626),
    int("vbInfoText", -2147483625),
    int("vbInfoBackground", -2147483624),
    // Tristate
    int("vbTrue", -1),
    int("vbFalse", 0),
    int("vbUseDefault", -2),
    // VarType
    int("vbEmpty", 0),
    int("vbNull", 1),
    int("vbInteger", 2),
    int("vbLong", 3),
    int("vbSingle", 4),
    int("vbDouble", 5),
    int("vbCurrency", 6),
    int("vbDate", 7),
    int("vbString", 8),
    int("vbObject", 9),
    int("vbError", 10),
    int("vbBoolean", 11),
    int("vbVariant", 12),
    int("vbDataObject", 13),
    int("vbDecimal", 14),
    int("vbByte", 17),
    int("vbLongLong", 20),
    int("vbLongPtr", 26),
    int("vbUserDefinedType", 36),
    int("vbArray", 8192),
];

pub fn vba_constant(name: &str) -> Option<VbaConstant> {
    VBA_CONSTANTS
        .iter()
        .copied()
        .find(|constant| constant.name.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::{VBA_CONSTANTS, VbaConstantValue, vba_constant};
    use std::collections::BTreeSet;

    #[test]
    fn constants_are_unique_case_insensitively() {
        let mut names = BTreeSet::new();
        for constant in VBA_CONSTANTS {
            assert!(
                names.insert(constant.name.to_ascii_lowercase()),
                "duplicate VBA constant {}",
                constant.name
            );
        }
    }

    #[test]
    fn type_library_constants_are_available_even_when_names_overlap_keywords() {
        assert_eq!(
            vba_constant("ReadOnly").map(|constant| constant.value),
            Some(VbaConstantValue::Integer(1))
        );
        assert_eq!(
            vba_constant("System").map(|constant| constant.value),
            Some(VbaConstantValue::Integer(4))
        );
        assert_eq!(
            vba_constant("Compressed").map(|constant| constant.value),
            Some(VbaConstantValue::Integer(128))
        );
    }
}
