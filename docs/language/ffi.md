# Native FFI

Valo supports VBA-style native external calls through `Declare Function` and `Declare Sub`.

```vb
Private Declare PtrSafe Function lstrlen Lib "libc.so.6" Alias "strlen" CDecl (
    ByVal value As String
) As Long
```

## Library Loading

Native libraries are loaded on first use and cached for the runtime lifetime. Resolution tries the declared name directly, then the current directory, executable directory, `PATH`, and platform-specific names.

- Windows accepts names such as `kernel32`, `kernel32.dll`, `user32`, and `ws2_32`.
- Linux accepts exact `.so` names such as `libc.so.6`; `libc` and `libm` have common fallbacks.
- macOS accepts `.dylib` names and basic framework-style fallbacks.

Loader failures are reported as `V3001`; missing symbols are reported as `V3002`.

## Calling Conventions

`CDecl` is supported as a Declare modifier after `Alias`/`Lib` and before the parameter list. The default ABI is the platform C ABI. `StdCall` is recognized and is only distinct on 32-bit Windows; unsupported ABI use is diagnosed instead of panicking.

## Marshaling

Supported scalar marshaling:

- `Byte`, `Integer`, `Long`, `LongLong`, `LongPtr`
- `Single`, `Double`, `Currency`
- `Boolean`
- `String` by value as a NUL-terminated ANSI/UTF-8 byte string
- `Variant` numeric/string coercion where the target parameter type is known

ByRef parameters pass mutable native pointers for supported scalar types and write the value back after the call. `LongPtr` maps to a pointer-sized runtime value: 32-bit on 32-bit targets and 64-bit on 64-bit targets.

Simple blittable arrays and structures are packed for native calls where practical, and ByRef structures and arrays are synchronized (write-back) into Valo runtime memory after native calls. Unsupported forms, including object references, mutable string buffers, and non-blittable fields, produce `V3003`.

## Pointers & Callbacks

Valo provides the `VarPtr`, `StrPtr`, and `ObjPtr` builtins for interfacing with raw memory pointers, as well as `AddressOf` for generating native function pointers for callbacks.

```vb
Declare PtrSafe Function EnumWindows Lib "user32" (ByVal lpEnumFunc As LongPtr, ByVal lParam As LongPtr) As Long

Function MyEnumWindowsProc(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Console.WriteLine("Got hwnd: " & Hex(hwnd))
    MyEnumWindowsProc = 1
End Function

Sub Main()
    ' AddressOf generates a libffi closure trampoline for native code to call back into Valo.
    EnumWindows(AddressOf MyEnumWindowsProc, 0)
End Sub
```

## Safety

Native calls are inherently unsafe at the ABI boundary. Valo isolates user-facing failures into diagnostics for missing libraries, missing symbols, unsupported marshaling, arity mismatches, pointer-safety issues, and unsupported calling conventions. Rust panic output is not part of the user-facing FFI surface.

## Limitations

The current runtime uses `libffi` for mixed native signatures. Complex COM/OLE Automation `Variant` pointers, object-pointer marshaling, and mutable string buffers are intentionally not exposed until the runtime has safe ownership rules for those cases.
