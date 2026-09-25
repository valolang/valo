# Valo

<img src="assets/valo-mascot.png" width="140" alt="Valo mascot">

**Basic syntax, systems semantics.**

Valo is a native systems programming language with VB.NET-inspired syntax,
implemented in Rust. Its direction combines readable application code with
deterministic memory management, value semantics, low-level control, zero-cost
abstractions, native compilation and heterogeneous CPU/GPU computing.

VB.NET is the language's syntax and ergonomics baseline. It does not define Valo's
runtime. Existing modern VB.NET-shaped features are preserved as Valo develops its
own native representation, ownership model and ABI.

## Current status

Valo is experimental. `valo run` uses a tree-walking interpreter; `valo build`
has an LLVM native compiler for a restricted subset. General native Class,
Collection, Variant, exception and reference-lifetime support is not available.

| Status | Capabilities |
| --- | --- |
| Implemented subsets | Classes, structures, interfaces, inheritance, modules, namespaces/imports, properties, events/delegates, generics and constraints, overloads, lambdas, tuples, query syntax, iterators, exceptions and Using |
| Implemented tooling | `valo run`, `valo check`, `valo repl`, source diagnostics, `valo.toml` entrypoints, native calls through libffi |
| Experimental | Typed HIR, CFG-based MIR, native primitives, plain value aggregates, fixed arrays and a managed String subset; native FFI/pointers and dynamic runtime behavior remain limited |
| In migration | Strict static defaults, coercions, array semantics, lifecycle and removal of remaining legacy-only syntax/runtime helpers |
| Planned | Full ownership/borrowing, safe unsafe boundaries, native Class/Collection support, deterministic user-resource destruction, specialization, SIMD, parallel/async runtimes, compile-time execution, GPU computing |
| Not planned | VBA/VB6 compatibility modes, Office macro execution, core COM/ActiveX semantics, mandatory CLR or tracing GC |

See the [repository audit and migration plan](docs/architecture/native-migration.md)
for exact implementation boundaries and remaining work. Existing legacy code in
the repository is migration debt, not a compatibility commitment.

## Readable application code

This example runs today:

```vb
Module Program
    Sub Main()
        Dim Numbers = Array(1, 2, 3, 4, 5)
        Dim Number As Integer
        For Each Number In Numbers
            Console.WriteLine(Number * Number)
        Next
    End Sub
End Module
```

## Value types in the same language

Structures and ByRef already work in the interpreter. This does not yet imply
stack allocation or zero-overhead native representation:

```vb
Structure Vec3
    Public X As Float32
    Public Y As Float32
    Public Z As Float32
End Structure

Sub Translate(ByRef Position As Vec3, ByVal Delta As Vec3)
    Position.X += Delta.X
    Position.Y += Delta.Y
    Position.Z += Delta.Z
End Sub
```

`Integer = Int32`, `Long = Int64`, `Short = Int16`, `Byte = UInt8`,
`UInteger = UInt32`, `ULong = UInt64`, `Single = Float32`, `Double = Float64`,
and `Boolean = Bool`. These aliases have fixed meanings across targets.
Int8/SByte, UInt16/UShort, ISize/USize and Char still need implementation.

## Future low-level code

**Design example only; typed pointers and Unsafe are not implemented:**

```vb
Unsafe Sub Fill(Destination As Pointer(Of Byte), Value As Byte, Length As USize)
    Dim I As USize = 0
    While I < Length
        Destination(I) = Value
        I += 1
    End While
End Sub
```

The loop also handles zero length without subtracting from an unsigned zero.

**Future GPU design; no GPU execution backend exists:**

```vb
<GpuKernel>
Sub AddVectors(A As DeviceBuffer(Of Float32), B As DeviceBuffer(Of Float32),
               Output As DeviceBuffer(Of Float32))
    Dim I = Gpu.GlobalId.X
    If I < Output.Length Then
        Output(I) = A(I) + B(I)
    End If
End Sub
```

## Build and run the Rust toolchain

```sh
cargo build --release
cargo run -p valo_cli -- run examples/hello.valo
cargo run -p valo_cli -- check examples/hello.valo
cargo run -p valo_cli -- build examples/native/native_control_flow.valo --release
cargo run -p valo_cli -- repl
cargo test --workspace
```

`cargo build` builds the Rust toolchain. `valo build` now has an experimental
LLVM native backend for a restricted primitive, aggregate and String subset. It needs `clang`, `opt`
and `llc` on `PATH`, or `VALO_LLVM_BIN` pointing to their directory. Native
`build` accepts `--emit=llvm-ir|obj|exe`, `--release` and `-o <path>`; normal
`valo run` remains interpreter-based. See the
[backend support matrix](docs/architecture/llvm-backend.md) before using it
for larger programs. Save sources as `.valo`; identifiers remain case-insensitive and source
casing is retained in the syntax tree and diagnostics.

An existing project manifest can be as small as:

```toml
[package]
name = "my-game"
version = "0.1.0"
entrypoint = "main.valo"
```

There is no compatibility-mode setting or implemented dependency resolver.

## Breaking changes

Exported `.bas`/`.cls` loading, implicit sibling imports, ANSI fallback,
compatibility manifest modes, core COM/OLE, `CreateObject`, `GetObject`, `DoEvents`,
`PtrSafe`, `LongPtr` and `Set` assignment are removed. Use normal assignment and
modern property setters. Parameters now default to ByVal, and VBA6/VBA7 are no longer predefined. Pin native ABI fields and parameters to their actual
widths; the old `Integer`/`Long` widths were incorrect for the VB.NET baseline.

## Documentation and contributions

- [Documentation](docs/README.md)
- [Native migration audit and architecture](docs/architecture/native-migration.md)
- [Language reference](docs/language/README.md)
- [Native FFI](docs/language/ffi.md)
- [Examples](examples/README.md) and [games](game/README.md)
- [Contributing](CONTRIBUTING.md)

Valo makes systems programming readable. Preserve the VB.NET surface, document
costs and unsupported capabilities, and test changes across frontend and execution.

Licensed under the [MIT license](LICENSE).
