# Language Reference

> Native systems migration: see the [audit and status](../architecture/native-migration.md). Existing VB.NET syntax is preserved; native compilation and ownership analysis are planned.


Documentation for the Valo language features.

*   **[Syntax Overview](syntax.md):** Basic types, variables, and control flow.
*   **[Expressions](expressions.md):** Operators, precedence, compound assignment, shifts, short-circuiting (`AndAlso`, `OrElse`), and numeric behavior.
*   **[Strings and Interpolation](strings.md):** String literals, concatenation, and `$"..."` with format specifiers and alignment.
*   **[Types and Conversions](types.md):** Native structures, nullable types (`T?`), byte arrays, and `CType` / `DirectCast` / `TryCast` / `GetType` / `NameOf`.
*   **[Generics](generics.md):** VB.NET-style generic classes, structures, functions, lambdas, and runtime strategy.
*   **[Functions](functions.md):** Procedures, lambdas (`Function(x) ...`), argument passing, and optional arguments.
*   **[Async and Await](async.md):** VB.NET-style async declarations, await validation, and current interpreter behavior.
*   **[Classes and Objects](classes.md):** Lifecycle, auto-properties, events (`AddHandler`), and visibility.
*   **[Properties](properties.md):** Properties and transitional accessor behavior.
*   **[Inheritance](inheritance.md):** Base classes, overrides, abstract members, and protected visibility.
*   **[Modules and Imports](modules.md):** Project organization and dependency management.
*   **[Error Handling](error-handling.md):** Robust runtime failure management.
* **[Breaking changes](vba-compat.md):** Removed project modes and automation features.
*   **[Standard Library Reference](standard-library.md):** Built-in functions, constants, implementation status, and known caveats.
* **[Platform automation](com.md):** Core COM removal and future library boundary.
*   **[FFI](ffi.md):** Native library declarations, pointer types, callbacks, and platform-aware loading.
