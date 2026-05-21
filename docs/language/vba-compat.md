# VBA Compatibility

Valo is built with first-class support for VBA (Visual Basic for Applications) while evolving as a modern standalone language. This document outlines the compatibility goals, the bridge layer between `.valo` and `.bas`/`.cls` files, and intentional differences.

## The Bridge Layer

Valo distinguishes between modern native code and legacy compatibility code primarily through file extensions and specific syntax choices.

### Source Modes
*   **`.valo` Files:** Modern native syntax. Prefers class `Sub New`/`Sub Terminate`, `Structure`, `Default` keyword, and structured imports.
*   **`.bas` / `.cls` Files:** VBA compatibility mode. Supports `Attribute VB_*` metadata, `Class_Initialize`, `Class_Terminate`, `Type`, `Declare`, and common exported-module encodings.

### Feature Comparison

| Feature | Native Valo (`.valo`) | VBA Compatibility (`.bas`/`.cls`) |
|---------|----------------------|-----------------------------------|
| Constructor | `Public Sub New()` | `Private Sub Class_Initialize()` |
| Destructor | `Public Sub Terminate()` | `Private Sub Class_Terminate()` |
| Default Member | `Public Default Property Get Item()` | `Attribute Item.VB_UserMemId = 0` on the Get/Let/Set property group |
| Value Records | `Public Structure Point` | `Public Type Point` |
| Byte Arrays | `Dim data() As Byte` | `Dim data() As Byte` |
| Debug Output | `Console.WriteLine` | `Debug.Print` |
| Error Handling | `Try/Catch/Finally` | `On Error GoTo` |
| Array Bounds | `1 To N` (optional) | `1 To N` (optional) |

### Built-in Compatibility
- `Debug.Print`: Available in all file modes, outputs to the standard console. Supports multiple comma-separated arguments.
- `Err` Object: Full support for `Err.Raise`, `Err.Number`, and `Err.Description` in all modes.
- `Array Built-ins`: `Split`, `Join`, `Filter`, `LBound`, and `UBound` behave according to standard VBA semantics.
- `Multidimensional Arrays`: Fully supported with `ReDim Preserve` compatibility (last-dimension only resizing).
- `New ClassName`: Parentheses are optional for zero-argument construction, matching VBA (`Set v = New Vec2`).
- `Const`: Module, local, and class-scope constants are supported, including multi-Const declarations such as `Public Const PI = 3.14, E = 2.71`.
- `^`: Exponent expressions are supported and evaluate through numeric promotion.
- `Declare`/`PtrSafe`: The frontend parses `Declare Function`/`Declare Sub` metadata, including `Lib`, `Alias`, `PtrSafe`, `LongPtr`, `LongLong`, and `As Any`. Runtime FFI invocation is intentionally future work.
- Source encodings: `.bas` and `.cls` imports accept UTF-8, UTF-8 BOM, UTF-16 LE/BE BOM, and Windows-1252/ANSI fallback, with normalized line endings for diagnostics.

`Structure` is the native Valo value type and supports methods, properties, constructors, and copy semantics. `Type` remains the VBA-compatible fields-only record syntax.
Structure fields may use constant-expression defaults, for example `Public X As Double = 0#`.

## Intentional Differences

While Valo strives for high compatibility, it is not a "bug-for-bug" clone. Some differences are intentional to improve safety and performance:

1.  **Strict Validation:** Valo performs comprehensive semantic analysis before execution. Many errors that VBA only catches at runtime (like type mismatches in assignments) are caught during compilation in Valo.
2.  **Explicit Scoping:** In modern `.valo` files, cross-module access requires explicit `Import` statements, whereas VBA modules share a global namespace.
3.  **No COM Dependency:** Valo does not depend on the Component Object Model (COM). It uses a native object model designed for performance and portability.
4.  **Modern Keywords:** Keywords like `Return` are preferred for returning values from functions and properties, although name-based assignment is still supported for compatibility.

## Compatibility Goals

*   **Migration Support:** Allow existing `.bas` and `.cls` files to be dropped into a Valo project and "just work" where practical.
*   **Ergonomic Bridge:** Native Valo code should be able to call into VBA-style modules and vice-versa seamlessly.
*   **Modern Foundation:** Ensure that the compatibility layer doesn't compromise the safety or performance of the core runtime.
