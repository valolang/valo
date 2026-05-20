# Runtime Architecture

The Valo Runtime defines the language's "soul"—its data types, object model, and core behaviors that must remain consistent regardless of whether code is interpreted or compiled.

## The Value System (`core/src/runtime/value.rs`)

The `Value` enum represents every possible data type in Valo:
*   **Scalars:** `Integer`, `Long`, `Double`, `String`, `Boolean`, `Date`.
*   **Fixed-Point:** `Currency` (4 decimal places), `Decimal`.
*   **Collections:** `Array` (multidimensional, dynamic/fixed).
*   **Structures:** `Record` (User-defined `Type`).
*   **Objects:** `Object` (Reference-counted class instances).
*   **Special:** `Nothing`, `Empty`, `Null`, `Missing`.

## Object Model (`core/src/interpreter/objects.rs`)
*Note: Some logic currently resides in the interpreter but is conceptually part of the runtime.*

*   **Reference Counting:** Valo uses `Rc<RefCell<ObjectValue>>` for automatic memory management.
*   **Lifecycle:** Supports `Sub New` (constructor) and `Sub Terminate` (finalizer) / `Class_Terminate` (VBA).
*   **Deterministic Cleanup:** The `Using` block and `IDisposable` pattern (via `Dispose` method) provide deterministic resource management.

## Type System (`core/src/runtime/type_name.rs`)

`TypeName` defines the static and dynamic types used by the semantic validator and runtime coercion logic.

## Resource Cleanup Model

1.  **Dispose:** An explicit method call for immediate cleanup.
2.  **Using:** A language construct that guarantees `Dispose` is called when a variable goes out of scope.
3.  **Terminate:** A fallback finalizer called when the last reference to an object is dropped.

Future FFI resources (file handles, database connections) will primarily utilize the `Using/Dispose` pattern for reliability.

## Builtins

Builtins are standard library functions that are baked into the runtime environment. They are categorized into domains:
*   `Math`: `Sgn`, `Int`, `Rnd`, `Randomize`.
*   `Strings`: `Split`, `Join`, `Filter`, `CStr`, `StrComp`.
*   `Arrays`: `Array`, `LBound`, `UBound`.
*   `Types`: `VarType`, `TypeName`, `IsNumeric`, `IsDate`, `IsArray`.
*   `Console`: `WriteLine`, `Write`.
*   `Debug`: `Print`.
*   `Err`: `Number`, `Description`, `Source`, `Raise`, `Clear`.
