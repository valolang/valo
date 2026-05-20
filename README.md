<div align="center">
  
  <img src="assets/valo-mascot.png" width="140" alt="Valo mascot">

  # Valo
  
  A modern Basic-inspired language and runtime with first-class VBA compatibility

</div>

<p align="center">
  <a href="#why-build-valo">Why?</a> |
  <a href="#language-goals">Goals</a> |
  <a href="#project-status">Status</a> |
  <a href="#getting-started">Getting started</a> |
  <a href="#vba-compatibility">VBA compatibility</a> |
  <a href="#contributing">Contributing</a>
</p>

<p align="center">
  <a href="https://github.com/uesleibros/valo/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/uesleibros/valo/test.yml?branch=main" alt="Build status">
  </a>
  <a href="https://github.com/uesleibros/valo/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/uesleibros/valo" alt="License">
  </a>
  <img src="https://img.shields.io/badge/runtime-Rust-orange" alt="Runtime">
  <img src="https://img.shields.io/badge/status-experimental-blue" alt="Status">
</p>

**Valo** is a modern Basic-inspired programming language and runtime built in Rust.

It is designed as a standalone evolution path for VBA-style development: familiar enough for VBA and Visual Basic developers, but with modern language features, a clean runtime, professional diagnostics, modules, a REPL, and a growing standard runtime surface.

Valo supports two complementary modes:

- **`.valo`**: native Valo syntax for modern development.
- **`.bas` / `.cls`**: VBA compatibility mode for migrating and running existing Basic-style code.

Valo is not Office automation. It is a standalone language/runtime that gives Basic-style programming a modern foundation outside of Excel, Access, COM, and the VBA editor.

## Why build Valo?

VBA remains one of the most productive programming environments ever shipped. It is simple, readable, approachable, and deeply useful for automation and business logic.

But VBA is also trapped inside a legacy ecosystem:

- tied to Office and COM
- difficult to package and distribute
- missing modern tooling
- missing a standalone runtime
- lacking modern diagnostics and project structure
- hard to evolve without breaking decades of assumptions

Valo takes a successor-language approach.

Instead of trying to clone VBA forever, Valo provides:

- a modern native language mode
- a compatibility bridge for existing `.bas` and `.cls` code
- a standalone runtime written in Rust
- a path toward modern tooling, packages, REPL workflows, FFI, and eventually compiled targets

This is similar in spirit to successor language projects:

- JavaScript → TypeScript
- Java → Kotlin
- C++ → Carbon
- VBA → **Valo**

Valo keeps the productivity and readability of Basic-style languages while building a runtime that can grow beyond the limitations of the original VBA environment.

## Language goals

Valo is designed around the following goals:

- **Familiar Basic-style syntax** for developers coming from VBA, VB6, or VB.NET.
- **First-class VBA compatibility** for `.bas` and `.cls` migration.
- **Modern native syntax** for new `.valo` projects.
- **Standalone runtime** independent of Office, COM, or the VBA editor.
- **Professional diagnostics** with explicit diagnostic codes and source spans.
- **Modular project structure** with imports and qualified symbols.
- **Strong runtime foundations** for arrays, objects, variants, errors, and native types.
- **A practical migration path** from legacy automation code to a modern runtime.
- **A clean implementation architecture** suitable for future tooling.

Valo is still experimental, but its direction is clear: a modern Basic-family language with compatibility as a bridge, not a cage.

## Project status

Valo is currently an experimental language/runtime in active development.

It already includes a substantial interpreter, parser, semantic validator, module loader, diagnostics engine, REPL, CLI, and compatibility runtime.

Implemented today:

- `.valo`, `.bas`, and `.cls` file support
- exported VBA `.cls` compatibility
- modules and imports
- classes, properties, events, and object lifecycle
- native `Sub New` / `Sub Terminate`
- deterministic cleanup with `Sub Dispose` and `Using`
- VBA `Class_Initialize` / `Class_Terminate`
- native `Structure` value types with methods, properties, and constructors
- VBA-compatible fields-only `Type`
- default properties and indexer-style access
- native `Iterator Function` and `Yield`
- `Try / Catch / Finally`
- VBA `On Error`, `Err`, `Resume`, and `Erl`
- advanced arrays, including multidimensional arrays
- `Array`, `Split`, `Join`, and `Filter`
- native and VBA-compatible type system
- `Variant`, `Object`, `Empty`, `Null`
- `CallByName`
- `Debug.Print`
- `VBA.` namespace fallback
- interactive REPL
- professional diagnostics
- clippy-clean Rust codebase

Valo is not yet a full production compiler. There is currently no bytecode VM, package manager, LSP, formatter, FFI layer, or complete standard library.

## Native Valo and VBA compatibility

Valo intentionally separates modern native syntax from compatibility syntax.

### Native Valo

Native `.valo` files use clean, modern Basic-style syntax:

```vb
Class Counter
    Private value As Integer

    Public Sub New()
        value = 0
    End Sub

    Public Sub Increment()
        value = value + 1
    End Sub

    Public Default Property Get Item() As Integer
        Return value
    End Property

    Public Sub Terminate()
        Debug.Print "counter disposed"
    End Sub
End Class

Public Structure Point
    Public X As Integer
    Public Y As Integer

    Public Sub Constructor(ByVal x As Integer, ByVal y As Integer)
        X = x
        Y = y
    End Sub

    Public Function Sum() As Integer
        Return X + Y
    End Function
End Structure

Sub WriteBytes()
    Dim data() As Byte
    ReDim data(0 To 15)
    data(0) = CByte(255)
End Sub

Sub Main()
    Dim counter As New Counter

    counter.Increment()
    counter.Increment()

    Console.WriteLine(counter)
End Sub
```

### VBA compatibility

VBA `.bas` and `.cls` files can use compatibility syntax:

```vb
VERSION 1.0 CLASS
BEGIN
  MultiUse = -1
END

Attribute VB_Name = "Counter"
Option Explicit

Private value As Long

Private Sub Class_Initialize()
    value = 0
End Sub

Private Sub Class_Terminate()
    Debug.Print "counter disposed"
End Sub

Public Property Get Item() As Long
Attribute Item.VB_UserMemId = 0
    Item = value
End Property
```

The goal is not to force modern Valo code to use legacy metadata. Instead, Valo accepts VBA metadata where needed for migration while offering cleaner syntax for new code.

## Feature highlights

### Modern control flow

```vb
Try
    DangerousOperation()
Catch ex As Error
    Console.WriteLine("Error " & ex.Number & ": " & ex.Message)
Finally
    Console.WriteLine("cleanup")
End Try
```

### VBA-style error handling

```vb
Sub Main()
    On Error GoTo Handler

    Err.Raise 1001, "Example", "Something failed"

    Exit Sub

Handler:
    Debug.Print Err.Number
    Debug.Print Err.Description
    Resume Next
End Sub
```

### Modules and imports

```vb
Import Math
Import Models As M

Sub Main()
    Console.WriteLine(Math.Add(10, 20))

    Dim user As New M.User("Valo")
    Console.WriteLine(user.Name)
End Sub
```

### Multidimensional arrays

```vb
Sub Main()
    Dim matrix(1 To 3, 1 To 2) As Integer

    matrix(1, 1) = 42
    matrix(3, 2) = 99

    Console.WriteLine(matrix(1, 1))
    Console.WriteLine(matrix(3, 2))
End Sub
```

### Array builtins

```vb
Sub Main()
    Dim parts As Variant

    parts = Split("A,B,C", ",")

    Console.WriteLine(parts(0))
    Console.WriteLine(Join(parts, "-"))
    Console.WriteLine(VBA.Join(parts, "/"))
End Sub
```

### Native type system

```vb
Sub Main()
    Dim b As Byte
    Dim i As Integer
    Dim l As Long
    Dim x As Int64
    Dim u As UInt64
    Dim d As Double
    Dim when As Date

    b = CByte(255)
    x = 9223372036854775807
    u = 18446744073709551615

    Console.WriteLine(TypeName(x))
    Console.WriteLine(VarType(d))
End Sub
```

## Supported file types

| Extension | Mode | Purpose |
|---|---|---|
| `.valo` | Native Valo | Modern Basic-inspired development |
| `.bas` | VBA module compatibility | Legacy standard modules |
| `.cls` | VBA class compatibility | Exported class modules |

## CLI

Valo ships with a command-line interface for running, checking, and experimenting with code.

```sh
valo run examples/hello.valo
valo check examples/types_showcase.valo
valo repl
valo version
valo help
```

### Run a file

```sh
valo run examples/hello.valo
```

### Check a file

```sh
valo check examples/modules/main.valo
```

### Start the REPL

```sh
valo repl
```

The REPL supports interactive experimentation with persistent state across snippets.

```txt
valo> Dim x As Integer
valo> x = 10
valo> Console.WriteLine(x)
10
```

## Getting started

Valo is currently built from source.

You need Rust installed.

```sh
git clone https://github.com/uesleibros/valo
cd valo
cargo build --release
```

Run the CLI:

```sh
./target/release/valo version
./target/release/valo run examples/hello.valo
```

On Windows:

```powershell
.\target\release\valo.exe version
.\target\release\valo.exe run examples\hello.valo
```

Run the full test suite:

```sh
cargo test
```

Run quality checks:

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo build --release
```

## Your first Valo program

Create `hello.valo`:

```vb
Sub Main()
    Console.WriteLine("Hello, Valo")
End Sub
```

Run it:

```sh
valo run hello.valo
```

Output:

```txt
Hello, Valo
```

## Diagnostics

Valo includes professional, Rust-inspired diagnostics with explicit diagnostic codes and source spans.

Example:

```txt
error[V1100]: Cannot assign String value to Integer variable
 --> examples/demo.valo:4:9
  |
4 |     age = "twenty"
  |         ^^^^^^^^^ expected Integer
  |
  = help: use an explicit conversion if the value is intended
```

Diagnostics are designed to be actionable, stable, and suitable for future editor tooling.

## Architecture

Valo is implemented in Rust and organized around a clean language pipeline:

```txt
source
  ↓
preprocessor
  ↓
lexer
  ↓
parser
  ↓
AST
  ↓
semantic validation
  ↓
module loader
  ↓
tree-walking interpreter
  ↓
runtime values and builtins
```

Major components include:

- `lexer`: tokenization
- `parser`: recursive descent parser
- `ast`: syntax tree structures
- `semantics`: validation and name/type resolution
- `modules`: multi-file project loading
- `runtime`: values, diagnostics, spans, and runtime types
- `interpreter`: execution engine
- `builtins`: modular runtime builtins
- `cli`: command-line interface and REPL

The interpreter is currently AST-based. A bytecode VM is a future direction, not the current execution model.

## VBA compatibility

Valo supports a growing set of VBA-compatible features:

- `.bas` and `.cls` parsing
- exported `.cls` envelopes
- `Attribute VB_Name`
- `Attribute VB_UserMemId = 0`
- `Class_Initialize`
- `Class_Terminate`
- `Type / End Type`
- `On Error`
- `Err`
- `Resume`
- `Erl`
- `Debug.Print`
- `CallByName`
- `VBA.Join`, `VBA.Split`, `VBA.TypeName`, and other namespace fallbacks
- `Variant`, `Object`, `Empty`, `Null`
- `Array`, `Split`, `Join`, `Filter`
- `LBound`, `UBound`, `ReDim`, `Erase`
- `VarType`, `TypeName`, `IsObject`, `IsArray`, `IsEmpty`, `IsNull`

Compatibility is pragmatic and growing. Valo is not a COM runtime and does not currently implement Office automation, IDispatch, or full COM interop.
`Declare`/`PtrSafe` and native FFI remain future work.

## Current limitations

Valo is still experimental.

Not implemented yet:

- package manager
- bytecode VM
- LSP
- formatter
- full standard library
- filesystem APIs
- full Date/Time API
- COM interop
- native FFI
- inheritance and interfaces
- generics
- full Collection / Dictionary runtime
- async runtime

Some compatibility areas are intentionally pragmatic today:

- `Currency` and `Decimal` may use simplified mixed arithmetic paths.
- `Rnd` does not yet perfectly replicate every VBA edge case.
- `.cls` metadata is parsed for compatibility, but COM semantics are not implemented.

## Roadmap

Near-term priorities:

- Collection and Dictionary runtime types
- Date and Time standard library
- FileSystem APIs
- richer string/runtime builtins
- compatibility-driven stabilization using real VBA modules
- syntax highlighting
- improved release packaging

Medium-term priorities:

- formatter
- LSP
- package manifest
- project-level module configuration
- benchmark suite
- standard library organization
- bytecode IR exploration

Long-term directions:

- bytecode VM
- FFI layer
- WASM/native backend groundwork
- editor integrations
- migration tooling for VBA projects

## Documentation

Start here:

- [Getting started](docs/getting-started.md)
- [Language syntax](docs/language/syntax.md)
- [Classes and objects](docs/language/classes.md)
- [Modules and imports](docs/language/modules.md)
- [Error handling](docs/language/error-handling.md)
- [Types](docs/language/types.md)
- [VBA compatibility](docs/language/vba-compat.md)
- [REPL](docs/repl.md)
- [Examples](examples/README.md)

Architecture docs:

- [Runtime](docs/architecture/runtime.md)
- [Parser](docs/architecture/parser.md)
- [Diagnostics](docs/architecture/diagnostics.md)
- [Roadmap](docs/architecture/roadmap.md)

## Contributing

Valo is in active development.

Good areas to contribute:

- language tests
- VBA compatibility cases
- documentation
- examples
- diagnostics
- standard library design
- CLI and REPL improvements
- runtime builtins

Before submitting changes, run:

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for more information.

## License

Valo is licensed under the [MIT License](LICENSE).
