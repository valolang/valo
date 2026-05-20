# Valo Examples

This directory contains a comprehensive set of examples demonstrating Valo's features, modern syntax, and VBA compatibility.

## Core Language Features

*   **[Hello World](hello.valo):** The classic starting point.
*   **[Variables](variables.valo):** Variable declarations and assignments.
*   **[Constants](consts.valo):** Global and module-level constants.
*   **[Data Types](types.valo):** Native types and VBA-compatible User-Defined Types (UDT).
*   **[Structures](structures.valo):** Native `Structure` value types.
*   **[Enums](enums.valo):** Enumeration types and their usage.
*   **[Arrays](arrays.valo):** Fixed and dynamic arrays.
*   **[ReDim](redim.valo):** Dynamic memory management for arrays.
*   **[Logical Operators](logical.valo):** Boolean logic and bitwise-style behavior.
*   **[Like Operator](like_operator.valo):** String pattern matching.

## Control Flow

*   **[If / Select Case](select_case.valo):** Basic and [Advanced](select_case_advanced.valo) branching.
*   **[Loops](for_loop.valo):** [For](for_loop.valo), [For Each](for_each.valo), [Do Loop](do_loop.valo), and [Exit](exit.valo) behavior.
*   **[With Block](with_block.valo):** Ergonomic member access for objects and types.

## Procedures and Modules

*   **[Subs](subs.valo) & [Functions](functions.valo):** Procedure definitions.
*   **[Optional Parameters](optional_params.valo):** Handling omitted arguments.
*   **[Named Arguments](named_arguments.valo):** Calling procedures with explicit parameter names.
*   **[Modules](modules/main.valo):** Multi-file project structure and [Module State](module_state.valo).

## Object-Oriented Programming

*   **[Classes](classes.valo):** Class definitions, properties, and methods.
*   **[Properties](properties.valo):** Property Get/Let/Set accessors.
*   **[Native Lifecycle](native_class_lifecycle.valo):** `Sub New` and `Sub Terminate` support.
*   **[Using Dispose](using_dispose.valo):** Deterministic cleanup with `Using` and `Dispose`.
*   **[Events](events.valo):** Declaring and raising events.
*   **[Default Properties](default_properties.valo):** [Native](native_default_property.valo) and [Indexer Style](indexer_style.valo) default members.
*   **[Nothing](nothing.valo):** Object reference management and `Is Nothing` checks.

## Error Handling

*   **[On Error Basic](on_error_basic.valo):** Standard `On Error GoTo` usage.
*   **[On Error Advanced](on_error_advanced.valo):** Complex error handling and recovery.

## VBA Compatibility

*   **[VBA Syntax](vba_syntax.valo):** Legacy Basic constructs supported by Valo.
*   **[Exported .cls](exported_class.cls):** Compatibility with files exported from VBA.
*   **[VBA Compat](vba_compat.valo):** Demonstrating the bridge between native and legacy features.

## Advanced Features

*   **[Conditional Compilation](conditional_compilation.valo):** Using `#If` and `#Const`.
*   **[Options](options.valo):** `Option Explicit`, `Option Base`, and `Option Compare`.
