# Native FFI

Valo supports native external calls with VB.NET-style declarations through `Declare Function` and `Declare Sub`.

```vb
Private Declare Function lstrlen Lib "libc" Alias "strlen" CDecl (
    ByVal value As String
) As UInt64
```

Declares are real callable symbols. A `Private Declare` is visible to code in the same module; a `Public Declare` is also available to importing modules according to normal module visibility rules. Declares live in `.valo` modules.

`Declare Function` can be used as an expression when the return value matters, or as a statement when the native return value is intentionally ignored:

```vb
Debug.Print lstrlen("Valo")
lstrlen("Valo")
Call lstrlen("Valo")
lstrlen "Valo"
```

`Declare Sub` follows the normal sub-call forms. `Alias` affects only native symbol lookup: the Valo callable name remains the declared name, while the loader resolves the alias target in the native library.

## Library Loading

Native libraries are loaded on first use and cached for the runtime lifetime. Resolution tries the declared name directly, then the current directory, executable directory, `PATH`, and platform-specific names.

Valo automatically maps common library names to platform-specific system libraries:
- `libc` and `libm` map to `msvcrt.dll` on Windows.
- `libc` and `libm` map to `libSystem.B.dylib` on macOS.
- `libc` and `libm` map to `libc.so.6` and `libm.so.6` on most Linux distributions.
- `libc` and `libm` map to `libc.so` and `libm.so` on Android/Termux.

Standard platform extensions are added automatically:
- Windows accepts names such as `kernel32`, `kernel32.dll`, `user32`, and `ws2_32`.
- Linux and Android accept exact `.so` names such as `libc.so.6`, `libm.so.6`, `libc.so`, or `libm.so`.
- macOS accepts `.dylib` names and basic framework-style fallbacks.

Loaded libraries and resolved symbols are cached for the interpreter lifetime. Loader failures are reported as `V3001`; missing symbols are reported as `V3002`.

## Calling Conventions

`CDecl` is supported as a Declare modifier after `Alias`/`Lib` and before the parameter list. The default ABI is the platform C ABI. `StdCall` is recognized and is only distinct on 32-bit Windows. On 64-bit platforms (Windows x64, macOS ARM64/x64, Linux x64/ARM64), all calling conventions typically collapse into the platform's single standard ABI.

## Marshaling

Supported scalar marshaling:

- `Byte`/`UInt8`, `Short`/`Int16`, `Integer`/`Int32`, `Long`/`Int64`, `UInt32`, `UInt64`, `Ptr` (pointer-sized)
- `Single`, `Double`, `Currency`
- `Boolean`
- `String` by value as a NUL-terminated ANSI/UTF-8 byte string
- `Variant` numeric/string coercion where the target parameter type is known

ByRef parameters pass mutable native pointers for supported scalar types and write the value back after the call. `Ptr` maps to a pointer-sized runtime value: 32-bit on 32-bit targets and 64-bit on 64-bit targets. Pointer conversions that cannot be represented on the current target are rejected as marshaling diagnostics.

ByVal strings are converted to temporary NUL-terminated byte strings and kept alive until the native call returns. Interior NUL bytes are rejected because C string APIs would otherwise observe a truncated value.

Simple blittable arrays and structures are packed for native calls where practical, and ByRef structures and arrays are synchronized (write-back) into Valo runtime memory after native calls. Structure fields use native-size alignment and final padding, which is required by ARM64 ABIs on macOS and Android. Unsupported forms, including object references, mutable string buffers, fixed arrays inside structures, and non-blittable fields, produce `V3003`.

## Pointers & Callbacks

Valo provides the `VarPtr`, `StrPtr`, and `ObjPtr` builtins for interfacing with raw memory pointers, as well as `AddressOf` for generating native function pointers for callbacks.

`StrPtr` accepts string variables and String-compatible temporary expressions. Temporary strings are stored by the interpreter until the current statement completes, keeping their buffers alive for the statement.

```vb
Declare Function EnumWindows Lib "user32" (ByVal lpEnumFunc As Ptr, ByVal lParam As Ptr) As Int32

Function MyEnumWindowsProc(ByVal hwnd As Ptr, ByVal lParam As Ptr) As Int32
    Console.WriteLine("Got hwnd: " & Hex(hwnd))
    MyEnumWindowsProc = 1
End Function

Sub Main()
    ' AddressOf generates a libffi closure trampoline for native code to call back into Valo.
    EnumWindows(AddressOf MyEnumWindowsProc, 0)
End Sub
```

Callback trampolines remain owned by the interpreter for the runtime lifetime. Callback parameters currently must be `ByVal` blittable scalar or pointer types; use `ByVal Ptr` for native handles and pointer payloads. Callback failures are converted into runtime diagnostics and default native return values instead of unwinding across the FFI boundary.

On Android/Termux ARM64, Valo exports one process-level `__clear_cache` compatibility shim for libffi closure preparation. The shim is defined only once, in the core crate, to avoid duplicate-symbol linker failures.

## Safety

Typed `Pointer(Of T)`, `Unsafe`, ownership and lifetime checking are planned. The current experimental pointer helpers do not provide memory-safety guarantees. The opening strlen example assumes a 64-bit target; its C return type is size_t. Use target-correct declarations when porting.

Native calls are inherently unsafe at the ABI boundary. Valo isolates user-facing failures into diagnostics for missing libraries, missing symbols, unsupported marshaling, callback initialization failure, arity mismatches, pointer-safety issues, executable trampoline failure, and unsupported calling conventions. Rust panic output is not part of the user-facing FFI surface.

## Limitations

The current runtime uses `libffi` for mixed native signatures. COM/OLE Automation marshalling is removed. Unsupported forms include object-pointer marshaling, mutable string buffers, nested non-blittable structures, and ByRef callback parameters are intentionally not exposed by the current interpreter.
