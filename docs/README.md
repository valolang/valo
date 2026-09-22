# Valo Documentation

> Native systems migration: see the [audit and status](architecture/native-migration.md). Existing VB.NET syntax is preserved; native compilation and ownership analysis are planned.


Valo is a native systems language with VB.NET-inspired syntax, implemented in Rust.
The current backend is an interpreter; native compilation and ownership are planned.

This directory holds the language reference and the design documentation.

## Language Reference

Learn how to write code in Valo.

*   **[Syntax Overview](language/syntax.md):** Basic statements and control flow.
*   **[Expressions](language/expressions.md):** Operators, compound assignment, shifts, and expression evaluation.
*   **[Strings and Interpolation](language/strings.md):** String literals, concatenation, and `$"..."` literals.
*   **[Types and Conversions](language/types.md):** The Valo type system, `CType`, `DirectCast`, `TryCast`, `GetType`, and `NameOf`.
*   **[Functions](language/functions.md):** Procedures, lambdas, argument passing, and optional arguments.
*   **[Properties](language/properties.md):** Modern property blocks and transitional accessors.
*   **[Classes and Objects](language/classes.md):** Lifecycle, properties, events, default members, and visibility.
*   **[Inheritance](language/inheritance.md):** Overrides, abstract members, protected access, and interfaces.
*   **[Generics](language/generics.md):** Generic classes, structures, functions, methods, lambdas, and constraints.
*   **[Async and Await](language/async.md):** Async declaration syntax and current interpreter behavior.
*   **[Modules and Imports](language/modules.md):** Project organization and dependency management.
*   **[Error Handling](language/error-handling.md):** Robust runtime failure management.
* **[Breaking changes](language/vba-compat.md):** Removal of the legacy project and automation direction.
* **[Platform automation](language/com.md):** The library boundary after core COM removal.
*   **[FFI](language/ffi.md):** Calling native libraries.
*   **[REPL](repl.md):** Interactive REPL documentation.
*   **[Examples](../examples/README.md):** Language examples and migration fixtures.

## Reference

*   **[Diagnostic Codes](reference/diagnostics.md):** Every code Valo can report, what it means, and the stability rules around it.

## Architecture

Deep dive into how Valo works under the hood.

*   **[Frontend](architecture/frontend.md):** Lexing, parsing, and semantics.
*   **[Backend](architecture/backend.md):** The execution backends.
*   **[Runtime and Interpreter](architecture/runtime.md):** The execution engine and object model.
*   **[Parser](architecture/parser.md):** Recursive descent and preprocessor logic.
*   **[Diagnostics](architecture/diagnostics.md):** How we provide world-class error reporting.
*   **[Module System](architecture/modules.md):** Discovery and semantic resolution.
*   **[Platform](architecture/platform.md):** Project identity, namespaces, runtime services, and interop direction.
*   **[Performance](architecture/performance.md):** How execution is measured and where the time goes.
*   **[Roadmap](architecture/roadmap.md):** Our future plans and vision.
