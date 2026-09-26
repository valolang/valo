# Valo Examples

This directory contains VB.NET-style Valo examples, native FFI and transitional runtime fixtures. The experimental LLVM backend can compile the restricted examples in `native/`, including managed Strings, Structures, Classes and numeric Collections. Full ownership, exceptions and GPU execution remain future work.

Run an example with:

```sh
valo run examples/hello.valo
```

From source, use:

```sh
cargo run -p valo_cli -- run examples/hello.valo
```

The integration test discovers examples with `Sub Main`, runs the supported set,
and compares what each one prints against a recorded transcript in
[`golden/`](golden):

```sh
cargo test -p valo_core --test examples -- --nocapture
```

Running an example only proves it does not crash. The transcripts make the
current behaviour explicit, so an example that quietly changed what it prints
fails instead of passing, which is what makes them a safety net for work on the
interpreter.

The four experimental `native/*.valo` examples have a
`Function Main() As Integer` native entry and cover control flow, Structures,
fixed arrays with For Each, and tuples. Build one with
`valo build examples/native/native_control_flow.valo --release`. They are tested
through the native CLI integration test when LLVM tools are available and are
not part of the interpreter's `Sub Main` transcript suite.

After adding an example, or when a change to its output is intended, record it:

```sh
VALO_BLESS=1 cargo test -p valo_core --test examples
```

Then read the diff before committing. A transcript is only worth having if
someone looked at what changed.

COM examples require Windows and the relevant COM server. They are skipped by the example integration test on non-Windows hosts.

## Core Language

- [Hello World](hello.valo) and [Native Introduction](native_intro.valo)
- [Variables](variables.valo), [Constants](consts.valo), and [Declaration Initializers](declaration_initializers.valo)
- [Types](types.valo), [Type Checks](type_checks.valo), [Structures](structures.valo), and [Structure Writeback](struct_writeback_test.valo)
- [Enums](enums.valo) and [VBA Enum Syntax](iterator_collection.valo)
- [Arrays](arrays.valo), [Array Builtins](array_builtins.valo), [Multidimensional Arrays](multidimensional_arrays.valo), and [ReDim](redim.valo)
- [Logical Operators](logical.valo), [Short-circuiting](short_circuit.valo), [Like](like_operator.valo), and [Advanced Like](like_advanced.valo)
- [Options](options.valo), [Zero-based arrays](zero_based_arrays.valo), [Option Compare](option_compare.valo), [Conditional Compilation](conditional_compilation.valo), and [Conditional Platform](conditional_platform.valo)
- [New Builtins](new_builtins.valo), [Collection](collection.valo), and [Collection Position](collection_position.valo)

## Control Flow

- [Control Flow](control_flow.valo)
- [For Loop](for_loop.valo), [For Each](for_each.valo), [Do Loop](do_loop.valo), [Exit](exit.valo), and [Next Variable](next_variable.valo)
- [Select Case](select_case.valo) and [Advanced Select Case](select_case_advanced.valo)
- [With Block](with_block.valo)

## Procedures, Modules, and Project Structure

- [Subs](subs.valo), [Functions](functions.valo), and [Let / Call](let_call.valo)
- [Optional Parameters](optional_params.valo), [Named Arguments](named_arguments.valo), and [Static Variables](static_variables.valo)
- [Module State](module_state.valo)
- [Multi-file Modules](modules/main.valo), with sibling module files under `examples/modules/`

## Classes, Interfaces, and Object Model

- [Classes](classes.valo), [Properties](properties.valo), and [VB.NET Properties](vbnet_properties.valo)
- [Native Class Lifecycle](native_class_lifecycle.valo) and [Using / Dispose](using_dispose.valo)
- [Nothing](nothing.valo), [Default Properties](default_properties.valo), [Native Default Property](native_default_property.valo), and [Indexer Style](indexer_style.valo)
- [Inheritance Basic](inheritance_basic.valo), [Inheritance Override](inheritance_override.valo), [Abstract Classes](abstract_animals.valo), [Protected Members](protected_members.valo), and [Generic Inheritance](generic_inheritance.valo)
- [Interface Polymorphism](interface_polymorphism.valo)
- [Events](events.valo) and [Global Event Handlers](events_global.valo)
- [Shared Auto Property](shared_auto_property.valo)

## Modern VB.NET Syntax

- [String Interpolation](string_interpolation.valo)
- [Compound Assignment and Shift Operators](compound_operators.valo)
- [Continue For / While / Do](continue_loops.valo)
- [Conversions and Reflection](conversions.valo) (`CType`, `DirectCast`, `TryCast`, `GetType`, `NameOf`)
- [Object Initializers](object_initializers.valo) (`New T With { .Member = value }`)
- [Lambdas](lambdas.valo) (single-expression and multi-line `Function` / `Sub`)
- [Overloads](overloads.valo) (several procedures sharing one name)
- [Delegates](delegates.valo) (named callable types)
- [Tuples](tuples.valo) (grouped values, tuple returns, and naming their elements)
- [Queries](queries.valo) (`From ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¦ Where ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¦ Order By ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¦ Select`)
- [Option Strict](option_strict.valo) (no silent narrowing, no late binding)
- [Null-Conditional Access](null_conditional.valo) (`obj?.Member`)

## Generics, Advanced Syntax, and Modern Features

- [Generic Box](generic_box.valo), [Generic Pair](generic_pair.valo), [Generic Identity](generic_identity.valo), [Generic Nested](generic_nested.valo), and [Generic Runtime](generic_runtime.valo)
- [Operator Overloading](operator_overloading.valo)
- [Extension Methods](extension_methods.valo) and [Extension Methods for Integer](extension_methods_int.valo)
- [Partial Classes](partial_classes.valo)
- [Nullable Types](nullable_types.valo)
- [Collection Initializers](collection_initializers.valo)
- [Iterator](iterator.valo) and [Iterator Range](iterator_range.valo)
- [LINQ-style APIs](linq_demo.valo)
- [Async / Await](async_demo.valo)
- [Ultimate Demo](ultimate_demo.valo)

## Error Handling

- [Try / Catch](try_catch.valo)
- [Throw](throw_example.valo)
- [On Error](on_error.valo), [On Error Basic](on_error_basic.valo), and [On Error Advanced](on_error_advanced.valo)

## Transitional runtime fixtures

The following old examples exercise remaining migration debt. They do not establish a VBA compatibility commitment. See the [audit](../docs/architecture/native-migration.md).

- [VBA Syntax](vba_syntax.valo)
- [VBA Compatibility Overview](vba_compat.valo)
- [VBA Constants](vba_constants.valo), covering the built-in constant groups exposed by the compatibility runtime
- [VBA String Functions](vba_string_functions.valo), plus broader runtime function coverage for math, conversion, type-checking, date/time, formatting, financial, selection, color, file, dialog, and host helpers
- [VBA Optional Arguments](vba_optional_arguments.valo) and [VBA Optional Forwarding](vba_optional_forwarding.valo)
- [VBA Property Compatibility](property_accessors.valo)
- [VBA StrPtr](vba_strptr.valo)
- [VBA Declare Strings](vba_declare_strings.valo) and [VBA Real-World Declares](vba_realworld_declares.valo)
- [VBA File I/O](vba_file_io.valo), [VBA Binary File I/O](vba_binary_file_io.valo), [VBA Random File I/O](vba_random_file_io.valo), [VBA Dir](vba_dir.valo), and [VBA File Attributes](vba_file_attributes.valo)
- [VBA Timer and Date/Time](vba_timer.valo)
- [LSet / RSet](lset_rset.valo)

## Native FFI and Pointer Interop

- [Callback](callback.valo)
- [Pointer Test](ptr_test.valo)

## Transcripts

Every example's output is recorded in [`golden/`](golden/) and compared on each
test run, so a change to what one prints has to be one someone meant to make.
Run the suite with `VALO_BLESS=1` to record a new example or accept a change.

An example whose output depends on where it runs, on the working directory or
the platform or what is installed, says so in a comment near the top:

```vb
' transcript: environment-dependent - prints the working directory.
```

It still runs, so a crash in it is still caught; only the comparison is skipped.
