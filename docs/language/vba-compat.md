# VBA Compatibility

Valo is built with first-class support for VBA (Visual Basic for Applications) while evolving as a modern standalone language. This document outlines the compatibility goals, the bridge layer between `.valo` and `.bas`/`.cls` files, and intentional differences.

## The Bridge Layer

Valo distinguishes between modern native code and legacy compatibility code primarily through file extensions and specific syntax choices.

### Source Modes
*   **`.valo` Files:** Modern native syntax. Prefers `Constructor`/`Terminate`, `Default` keyword, and structured imports.
*   **`.bas` / `.cls` Files:** VBA compatibility mode. Supports `Attribute VB_*` metadata, `Class_Initialize`, and `Class_Terminate`.

### Feature Comparison

| Feature | Native Valo (`.valo`) | VBA Compatibility (`.bas`/`.cls`) |
|---------|----------------------|-----------------------------------|
| Constructor | `Public Constructor()` | `Private Sub Class_Initialize()` |
| Destructor | `Public Terminate()` | `Private Sub Class_Terminate()` |
| Default Member | `Public Default Property Get Item()` | `Attribute Item.VB_UserMemId = 0` |
| Imports | `Import Math` | Automatically shared in project |
| Metadata | Not required | `Attribute VB_Name = "..."` |

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
