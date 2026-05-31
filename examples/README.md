# Valo Examples

This directory contains runnable examples for Valo's native language features, VBA compatibility surface, native FFI, and Windows COM automation.

Run an example with:

```sh
valo run examples/hello.valo
```

From source, use:

```sh
cargo run -p valo_cli -- run examples/hello.valo
```

The integration test discovers examples with `Sub Main` and runs the supported set:

```sh
cargo test -p valo_core --test examples -- --nocapture
```

COM examples require Windows and the relevant COM server. They are skipped by the example integration test on non-Windows hosts.

## Core Language

- [Hello World](hello.valo) and [VBA-style Hello](hello.bas)
- [Variables](variables.valo), [Constants](consts.valo), and [Declaration Initializers](declaration_initializers.valo)
- [Types](types.valo), [Type Checks](type_checks.valo), [Structures](structures.valo), and [Structure Writeback](struct_writeback_test.valo)
- [Enums](enums.valo) and [VBA Enum Syntax](vba_new_enum.valo)
- [Arrays](arrays.valo), [Array Builtins](array_builtins.valo), [Multidimensional Arrays](multidimensional_arrays.valo), and [ReDim](redim.valo)
- [Logical Operators](logical.valo), [Short-circuiting](short_circuit.valo), [Like](like_operator.valo), and [Advanced Like](like_advanced.valo)
- [Options](options.valo), [Option Base](option_base.valo), [Option Compare](option_compare.valo), [Conditional Compilation](conditional_compilation.valo), and [Conditional Platform](conditional_platform.valo)
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

## VBA Compatibility

- [VBA Syntax](vba_syntax.valo)
- [Exported Class Module](exported_class.cls)
- [VBA Compatibility Overview](vba_compat.valo)
- [VBA Constants](vba_constants.valo)
- [VBA String Functions](vba_string_functions.valo)
- [VBA Optional Arguments](vba_optional_arguments.valo) and [VBA Optional Forwarding](vba_optional_forwarding.valo)
- [VBA Property Compatibility](vba_property_compat.valo)
- [VBA StrPtr](vba_strptr.valo)
- [VBA Declare Strings](vba_declare_strings.valo) and [VBA Real-World Declares](vba_realworld_declares.valo)
- [VBA File I/O](vba_file_io.valo), [VBA Binary File I/O](vba_binary_file_io.valo), [VBA Random File I/O](vba_random_file_io.valo), [VBA Dir](vba_dir.valo), and [VBA File Attributes](vba_file_attributes.valo)
- [VBA Timer and Date/Time](vba_timer.valo)
- [LSet / RSet](lset_rset.valo)

## Native FFI and Pointer Interop

- [Callback](callback.valo)
- [Pointer Test](ptr_test.valo)

## Windows COM Automation

- [COM Dictionary](com_dictionary.valo)
- [COM FileSystemObject](com_filesystem.valo)
- [COM PowerPoint](com_powerpoint.valo)
