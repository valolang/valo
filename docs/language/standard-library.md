# Standard Library Reference

This page inventories the current interpreter builtins. Many overlap VB.NET; legacy-only functions and constants are migration debt, not a source-compatibility promise. See the [migration audit](../architecture/native-migration.md).

## Implementation Status

| Level | Meaning |
|-------|---------|
| Implemented | Runs in the standalone Valo runtime with deterministic behavior covered by tests. |
| Recognized, limited | The function name validates and has a practical implementation, but locale, host, Office, or edge-case behavior is narrower than VBA. |
| Platform-dependent | Behavior depends on the operating system, available native libraries, or host facilities. |
| Stub / diagnostic | Remaining legacy behavior: some names return fallback data or an unsupported-feature diagnostic. These stubs are scheduled for removal or real library implementations. |

## Built-in Constants

Valo exposes the Microsoft Learn VBA constant groups case-insensitively, both unqualified and through the `VBA.` namespace.

| Group | Level | Notes |
|-------|-------|-------|
| Miscellaneous constants such as `vbCrLf`, `vbTab`, `vbNullString` | Implemented | Deterministic string and integer constants. |
| Calendar, comparison, date, first-day, and first-week constants | Implemented | Used by date and formatting helpers where supported. |
| Color, keycode, MsgBox, CallType, Shell, StrConv, TriState, VarType constants | Implemented | Numeric values match the VBA constant surface used by compatibility code. |
| File, directory, drive, and attribute constants | Recognized, limited | Common local filesystem values are honored. Platform-specific attributes such as system, volume, archive, alias, and hidden metadata are partial. |
| System color, form, query-close, IME, and special-folder constants | Recognized, limited | Values are available so imported code validates; host-specific behavior is not implied. |

## Runtime Functions

| Category | Functions | Level | Notes |
|----------|-----------|-------|-------|
| Arrays | `Array`, `Filter`, `Join`, `LBound`, `Split`, `UBound` | Implemented | Supports VBA-style arrays and common string array workflows. |
| Strings | `Asc`, `AscW`, `Chr`, `ChrW`, `Hex`, `InStr`, `InStrRev`, `LCase`, `Left`, `Len`, `LenB`, `LTrim`, `Mid`, `Oct`, `Partition`, `Replace`, `Right`, `RTrim`, `Space`, `Str`, `StrComp`, `StrConv`, `String`, `StrReverse`, `Trim`, `UCase`, `Val` | Implemented | `$` string-returning spellings dispatch to the same implementations. `StrConv` covers the practical case-conversion subset. |
| Math | `Abs`, `Atn`, `Cos`, `Exp`, `Fix`, `Int`, `Log`, `Rnd`, `Round`, `Sgn`, `Sin`, `Sqr`, `Tan` | Implemented | Numeric behavior is deterministic and uses Valo runtime numeric promotion. |
| Conversion and type checks | `CBool`, `CByte`, `CCur`, `CDate`, `CDbl`, `CDec`, `CInt`, `CLng`, `CLngLng`, `CLngPtr`, `CStr`, `CVar`, `CVErr`, `IsArray`, `IsDate`, `IsEmpty`, `IsError`, `IsMissing`, `IsNull`, `IsNumeric`, `IsObject`, `TypeName`, `VarType` | Implemented | Includes modern conversion functions and legacy helpers awaiting classification. Conversions to an integral type round, with ties to even: `CInt(3.9)` is 4 and `CInt(2.5)` is 2. `Int` floors and `Fix` truncates. |
| Selection helpers | `Choose`, `IIf`, `Switch` | Implemented | `IIf`, `Choose`, and `Switch` are evaluated as Valo special forms so selected branches behave predictably. |
| Date/time basics | `Date`, `DateSerial`, `DateValue`, `Day`, `Hour`, `Minute`, `Month`, `MonthName`, `Now`, `Second`, `Time`, `TimeSerial`, `TimeValue`, `Timer`, `Weekday`, `WeekdayName`, `Year` | Implemented | Uses Valo's VBA serial date value. `DateValue` and `TimeValue` intentionally accept a deterministic parse subset. |
| Date intervals | `DateAdd`, `DateDiff`, `DatePart` | Recognized, limited | Common intervals are implemented. Month, quarter, and year `DateAdd` clamp end-of-month results. First-week and locale-sensitive edge cases remain precision targets. |
| Formatting | `Format`, `FormatCurrency`, `FormatDateTime`, `FormatNumber`, `FormatPercent`, `Spc`, `Tab` | Recognized, limited | Numeric/date formatting is deterministic and intentionally less locale-dependent than Office VBA. Common named formats such as Short Date, Long Time, Standard, Percent, and Currency are covered. |
| Financial | `DDB`, `FV`, `IPmt`, `IRR`, `MIRR`, `NPer`, `NPV`, `Pmt`, `PPmt`, `PV`, `Rate`, `SLN`, `SYD` | Recognized, limited | Financial formulas are implemented. IRR/MIRR validate mixed cashflows and rate solving uses a Newton pass with a bracketed fallback. Rounding and edge cases remain active fidelity targets. |
| Classic file numbers | `Close`, `EOF`, `FileAttr`, `FreeFile`, `Get #`, `Input #`, `Input`, `Line Input #`, `LOF`, `Loc`, `Open`, `Print #`, `Put #`, `Seek`, `Write #` | Implemented | Covers deterministic local Input, Output, Append, Binary, and Random file workflows. |
| Filesystem helpers | `ChDir`, `CurDir`, `Dir`, `FileDateTime`, `FileLen`, `GetAttr`, `Kill`, `MkDir`, `Name`, `RmDir` | Recognized, limited | Local filesystem behavior is supported. Exact OS locking and platform-specific attributes are not complete VBA parity. |
| Dialog and host helpers | `Command`, `Environ`, `Error`, `IMEStatus`, `InputBox`, `MsgBox` | Platform-dependent | Console and OS behavior vary by runtime. Non-Windows dialogs use pragmatic fallbacks. |
| Settings helpers | `GetSetting`, `GetAllSettings` | Stub / diagnostic | `GetSetting` returns the supplied default when no host settings store exists. `GetAllSettings` returns an empty array. |
| Shell helpers | `Shell` | Platform-dependent | Starts a process through the platform shell and returns its process id. Window style behavior is not claimed as full VBA parity. |
| Color helpers | `QBColor`, `RGB` | Implemented | Clamps RGB components and maps the 16 QBColor indexes. |
| Mac helpers | `MacID`, `MacScript` | Stub / diagnostic | `MacID` computes the four-character code. `MacScript` reports an unsupported-runtime diagnostic. |
| Pointer helpers | `AddressOf`, `ObjPtr`, `StrPtr`, `VarPtr` | Platform-dependent | Intended for native interop and callbacks; behavior depends on runtime object storage and native ABI support. |

## Known Caveats

Formatting, date intervals, and financial functions are intentionally useful before they are complete. They are deterministic across platforms, which means some Office VBA locale behavior, host settings, and precision edge cases differ today.

Host-owned features are tracked separately from pure runtime functions. core COM automation has been removed, registry-backed settings are not persisted by the standalone runtime, `MacScript` is accepted only to produce a compatibility diagnostic.
