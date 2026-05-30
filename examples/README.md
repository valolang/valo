# Valo Examples

This directory contains a comprehensive set of examples demonstrating Valo's features, modern syntax, and VBA compatibility.

## Core Language Features

*   **[Hello World](hello.valo):** The classic starting point.
*   **[Variables](variables.valo):** Variable declarations and assignments.
*   **[Constants](consts.valo):** Global and module-level constants.
*   **[Data Types](types.valo):** Native types and VBA-compatible User-Defined Types (UDT).
*   **[Structures](structures.valo):** Native `Structure` value types.
*   **Generics:** [Box](generic_box.valo), [Pair](generic_pair.valo), [Identity](generic_identity.valo), [Nested](generic_nested.valo), and [Runtime](generic_runtime.valo) examples.
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
*   **[Inheritance Basic](inheritance_basic.valo):** Inherited fields and methods.
*   **[Inheritance Override](inheritance_override.valo):** `Overridable` and `Overrides` dispatch.
*   **[Abstract Animals](abstract_animals.valo):** `MustInherit` and `MustOverride`.
*   **[Protected Members](protected_members.valo):** `Protected` member access from derived classes.
*   **[Generic Inheritance](generic_inheritance.valo):** Generic base classes.
*   **[Interface Polymorphism](interface_polymorphism.valo):** Interface implementation dispatch.
*   **[Properties](properties.valo):** Property Get/Let/Set accessors.
*   **[VB.NET Properties](vbnet_properties.valo):** Auto-properties, full property blocks, `ReadOnly`, and `WriteOnly`.
*   **[Native Lifecycle](native_class_lifecycle.valo):** `Sub New` and `Sub Terminate` support.
*   **[Using Dispose](using_dispose.valo):** Deterministic cleanup with `Using` and `Dispose`.
*   **[Events](events.valo):** Declaring and raising events.
*   **[Default Properties](default_properties.valo):** [Native](native_default_property.valo) and [Indexer Style](indexer_style.valo) default members.
*   **[Nothing](nothing.valo):** Object reference management and `Is Nothing` checks.

## Error Handling

*   **[Throw](throw_example.valo):** Throwing exceptions and `Try/Catch` block usage.
*   **[On Error Basic](on_error_basic.valo):** Standard `On Error GoTo` usage.
*   **[On Error Advanced](on_error_advanced.valo):** Complex error handling and recovery.

## VBA Compatibility

*   **[VBA Syntax](vba_syntax.valo):** Legacy Basic constructs supported by Valo.
*   **[Exported .cls](exported_class.cls):** Compatibility with files exported from VBA.
*   **[VBA Compat](vba_compat.valo):** Demonstrating the bridge between native and legacy features.
*   **[VBA Constants](vba_constants.valo):** Generic VBA runtime constants such as `vbCrLf`, `vbTab`, `vbTrue`, and `vbString`.
*   **[VBA String Functions](vba_string_functions.valo):** Safe VBA string/runtime functions such as `Len`, `Left$`, `Mid$`, `Replace`, `InStr`, `Chr$`, `Hex$`, and `Oct$`.
*   **[VBA File I/O](vba_file_io.valo):** Classic file-number I/O using `FreeFile`, `Open`, `Print #`, `Line Input #`, `EOF`, `Close`, and `Kill`.
*   **[VBA Property Compatibility](vba_property_compat.valo):** Property Let without explicit `ByVal`.
*   **[VBA StrPtr](vba_strptr.valo):** `StrPtr` with variables and temporary string expressions.
*   **[VBA Optional Forwarding](vba_optional_forwarding.valo):** Omitted optional Variant handling with `IsMissing`.
*   **[VBA Declare Strings](vba_declare_strings.valo):** Declare string argument temporaries.
*   **[VBA Real-World Declares](vba_realworld_declares.valo):** Keyword-like parameter names in Declare signatures.
*   **[VBA Dir](vba_dir.valo):** Basic wildcard enumeration with `Dir(pattern)` and repeated `Dir()` calls.
*   **[VBA Binary File I/O](vba_binary_file_io.valo):** `Open For Binary`, scalar `Put #`/`Get #`, `LOF`, `Close`, and cleanup.
*   **[VBA Random File I/O](vba_random_file_io.valo):** Fixed-length `Open For Random Len =` records with `Put #` and `Get #`.
*   **[VBA File Attributes](vba_file_attributes.valo):** `Dir` with `vbDirectory`, `FileLen`, `FileDateTime`, `CurDir`, `MkDir`, and `RmDir`.
*   **[VBA Timer](vba_timer.valo):** `Timer`, `Now`, `DateSerial`, `Year`, `MonthName`, and `WeekdayName`.
*   **[VBA Optional Arguments](vba_optional_arguments.valo):** Optional defaults and Optional Variant `IsMissing`.
*   **[COM FileSystem](com_filesystem.valo):** Scripting.FileSystemObject for listing files and folders.
*   **[COM Dictionary](com_dictionary.valo):** Using the Scripting.Dictionary automation object.
*   **[COM PowerPoint](com_powerpoint.valo):** Automating Microsoft PowerPoint (requires PowerPoint).


Run an example with:

```sh
valo run examples/vba_file_io.valo
```

## Advanced Features

*   **[Conditional Compilation](conditional_compilation.valo):** Using `#If` and `#Const`.
*   **[Options](options.valo):** `Option Explicit`, `Option Base`, and `Option Compare`.
*   **[Operator Overloading](operator_overloading.valo):** Defining custom behavior for operators (`+`, `-`, `=`, etc.) on classes and structures.
*   **[Extension Methods](extension_methods.valo):** Extending existing types with new methods using the `<Extension()>` attribute.
*   **[Partial Classes](partial_classes.valo):** Splitting class definitions across multiple blocks or files.
*   **[Nullable Types](nullable_types.valo):** Using `T?` for value and reference types, checking `Is Nothing`, `.HasValue`, and `.Value`.
*   **[Collection Initializers](collection_initializers.valo):** Populating collections inline using `New Collection() From { ... }`.
*   **[LINQ-style APIs](linq_demo.valo):** Fluent querying of collections using Lambda expressions (`Function(n) ...`) and Extension Methods (`Where`, `Select`, `Any`, etc.).
*   **[Async/Await](async_demo.valo):** Asynchronous programming syntax with `Async Function` and `Await`.
