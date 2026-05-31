# Valo Documentation

Welcome to the Valo documentation. This repository contains the source code and design documentation for the Valo language and runtime.

## Language Reference

Learn how to write code in Valo.

*   **[Syntax Overview](language/syntax.md):** Basic statements and control flow.
*   **[Expressions](language/expressions.md):** Operators and expression evaluation.
*   **[Types](language/types.md):** The Valo type system.
*   **[Functions](language/functions.md):** Procedures, lambdas, argument passing, and optional arguments.
*   **[Properties](language/properties.md):** Native and VBA-compatible property accessors.
*   **[Classes and Objects](language/classes.md):** Lifecycle, properties, events, default members, and visibility.
*   **[Inheritance](language/inheritance.md):** Overrides, abstract members, protected access, and interfaces.
*   **[Generics](language/generics.md):** Generic classes, structures, functions, methods, lambdas, and constraints.
*   **[Async and Await](language/async.md):** Async declaration syntax and current interpreter behavior.
*   **[Modules and Imports](language/modules.md):** Project organization and dependency management.
*   **[Error Handling](language/error-handling.md):** Robust runtime failure management.
*   **[VBA Compatibility](language/vba-compat.md):** Information on the bridge between modern Valo and legacy VBA.
*   **[COM Automation](language/com.md):** Windows COM/OLE Automation support.
*   **[FFI](language/ffi.md):** Calling native libraries.
*   **[REPL](repl.md):** Interactive REPL documentation.
*   **[Examples](../examples/README.md):** Runnable language and compatibility examples.

## Architecture

Deep dive into how Valo works under the hood.

*   **[Frontend](architecture/frontend.md):** Lexing, parsing, and semantics.
*   **[Backend](architecture/backend.md):** The execution backends.
*   **[Runtime and Interpreter](architecture/runtime.md):** The execution engine and object model.
*   **[Parser](architecture/parser.md):** Recursive descent and preprocessor logic.
*   **[Diagnostics](architecture/diagnostics.md):** How we provide world-class error reporting.
*   **[Module System](architecture/modules.md):** Discovery and semantic resolution.
*   **[Platform](architecture/platform.md):** Project identity, namespaces, runtime services, and interop direction.
*   **[Roadmap](architecture/roadmap.md):** Our future plans and vision.
