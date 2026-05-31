# Runtime Architecture

The Valo Runtime defines the language's core data types, object model, and behaviors that must remain consistent regardless of whether code is interpreted or compiled.

## The Value System (`core/src/runtime/value.rs`)

The `Value` enum represents every possible data type in Valo:
*   **Scalars:** `Integer`, `Long`, `Double`, `String`, `Boolean`, `Date`.
*   **Fixed-Point:** `Currency` (4 decimal places), `Decimal`.
*   **Collections:** `Array` (multidimensional, dynamic/fixed).
*   **Structures:** `Record` (User-defined `Type`).
*   **Objects:** `Object` (Reference-counted class instances).
*   **Special:** `Nothing`, `Empty`, `Null`, `Missing`.

## Centralized Operations (`core/src/runtime/ops.rs`, `compare.rs`, `numeric.rs`, `coerce.rs`)

Core language behaviors are implemented in the runtime layer to ensure consistency across backends:
*   **`ops.rs`**: Binary operations and mapping.
*   **`compare.rs`**: Equality, comparisons, and `Like` operator.
*   **`numeric.rs`**: Numeric promotion and conversion.
*   **`coerce.rs`**: Assignment coercion and type validation.

## Object Model (`core/src/backend/interpreter/objects.rs`)
*Note: Some logic currently resides in the interpreter backend but is conceptually part of the runtime.*

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

## Native FFI Runtime

The interpreter registers `Declare Function` and `Declare Sub` declarations as callable signatures during semantic validation and runtime initialization. Declares use the same callable lookup surface as normal functions and subs while retaining FFI metadata for native dispatch. Calls are dispatched before normal Valo procedure lookup; a `Declare Function` can also be invoked through the sub-call path when VBA statement syntax intentionally discards the return value.

Native support lives in `core/src/backend/interpreter/ffi.rs` and `core/src/runtime/ffi_platform.rs`, providing:

*   Platform library resolution and caching.
*   Symbol caching per loaded library to avoid repeated platform lookups.
*   Per-interpreter libffi CIF caching keyed by resolved Declare signature, so repeated native calls do not rebuild call metadata in tight loops.
*   Automatic system library mapping (`libc`/`libm` to platform equivalents).
*   Symbol lookup through `LoadLibrary`/`GetProcAddress` on Windows and `dlopen`/`dlsym` on Unix platforms.
*   Mixed-signature invocation through `libffi`.
*   Pointer-aware `PtrSafe` and `LongPtr` validation.
*   Scalar, string, ByRef, simple array, and native-aligned blittable structure marshaling where safe.
*   Dynamic libffi closure trampolines for native callbacks (`AddressOf`).
*   Pointer builtins for raw memory inspection (`VarPtr`, `StrPtr`, `ObjPtr`).
*   Diagnostics `V3001` through `V3004` for library, symbol, marshaling, and ABI/call failures.

Libraries are closed when the interpreter shuts down. Unsupported native shapes are rejected with diagnostics rather than exposing internal panics.

For project modules, runtime registration stores all declares under their module-qualified key so private declares remain callable from the same module. Only public declares are exported into unqualified imported lookup. Qualified access to an imported private declare reports a private-access diagnostic, matching the normal module callable rules. `Alias` is kept on the declare metadata and is applied only when resolving the native symbol.

The callback model keeps libffi closure memory, executable code pointers, CIF metadata, and callback signature data alive in the interpreter until shutdown. The active interpreter is installed through a thread-local guard while native calls are in progress, so callback re-entry restores the previous interpreter state even if a native call reports an error. Callback panics and Valo callback errors are contained at the trampoline boundary and translated into diagnostics/default return values; they are not allowed to unwind into native code.

Executable callback memory and instruction-cache synchronization are delegated to libffi and the host runtime. Android/Termux ARM64 also gets a single core-crate `__clear_cache` compatibility shim, because the bundled libffi archive can reference that symbol while Bionic does not always provide it at link time. The CLI does not export its own copy, preventing duplicate linker symbols.

Platform ABI notes:

*   Windows uses `LoadLibraryA`/`GetProcAddress`; `StdCall` is rejected except where it is meaningful on 32-bit Windows. Windows x64 uses the platform default ABI.
*   macOS maps `libc`/`libm` to `libSystem.B.dylib`; ARM64 structure marshaling uses native field alignment and padding.
*   Linux maps `libc`/`libm` to the usual glibc sonames.
*   Android/Termux maps `libc`/`libm` to Bionic `.so` names and uses the single core cache-flush shim required by bundled libffi builds.

## Builtins

Builtins are standard runtime functions and host services exposed without user-defined imports. Runtime dispatch lives under `core/src/backend/interpreter/builtins/`, while shared builtin-name metadata lives in `core/src/runtime/builtins.rs` so frontend validation and backend dispatch do not maintain separate hardcoded lists.

Builtins are grouped by domain:

*   `Math`: `Sgn`, `Int`, `Rnd`, `Randomize`.
*   `Strings`: `Split`, `Join`, `Filter`, `CStr`, `StrComp`, `Len`, `Left`, `Right`, `Mid`, `Trim`, `Replace`, `InStr`, `Chr`, `Asc`, `Val`, `Hex`, `Oct`, and related `$` aliases.
*   `Arrays`: `Array`, `LBound`, `UBound`.
*   `Types`: `VarType`, `TypeName`, `IsObject`, `IsNumeric`, `IsDate`, `IsArray`, `IsNull`, `IsEmpty`, `IsError`, `IsMissing`, and conversion helpers such as `CInt`, `CLng`, `CDbl`, `CDate`, and `CBool`.
*   `Date/Time`: `Timer`, `Now`, `Date`, `Time`, `DateSerial`, `TimeSerial`, `DateValue`, `TimeValue`, `Year`, `Month`, `Day`, `Hour`, `Minute`, `Second`, `Weekday`, `MonthName`, and `WeekdayName`.
*   `File I/O`: `FreeFile`, `EOF`, `LOF`, `Seek`, `Dir`, `FileLen`, `FileDateTime`, `CurDir`, `Kill`, `MkDir`, `RmDir`, and `ChDir`.
*   `Console`: `WriteLine`, `Write`, `ReadLine`.
*   `Debug`: `Print`, `Assert`.
*   `Err`: `Number`, `Description`, `Source`, `HelpFile`, `HelpContext`, `Raise`, `Clear`.
