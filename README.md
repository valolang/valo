<div align="center">

  <img src="assets/valo-mascot.png" width="140" alt="Valo mascot">

  # The Valo Programming Language

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
  <a href="https://github.com/valolang/valo/actions/workflows/ci.yml">
    <img src="https://github.com/valolang/valo/actions/workflows/ci.yml/badge.svg" alt="CI Status">
  </a>
  <a href="https://github.com/valolang/valo/actions/workflows/release.yml">
    <img src="https://github.com/valolang/valo/actions/workflows/release.yml/badge.svg" alt="Release Status">
  </a>
  <a href="https://github.com/valolang/valo/releases">
    <img src="https://img.shields.io/github/v/release/valolang/valo?include_prereleases&label=release" alt="Release">
  </a>
  <a href="https://github.com/valolang/valo/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/valolang/valo" alt="License">
  </a>
  <img src="https://img.shields.io/badge/runtime-Rust-orange" alt="Runtime">
  <img src="https://img.shields.io/badge/status-experimental-blue" alt="Status">
</p>

> [!NOTE]
> Valo is experimental and not production-ready yet. APIs, syntax, runtime behavior, and compatibility details may change quickly.

## What is Valo?

**Valo** is a modern Basic-inspired programming language and runtime built in Rust.

The original idea behind Valo was simple: **what if VBA had its own Node.js moment?**

JavaScript was once mostly tied to the browser. Node.js made it possible to use JavaScript almost anywhere: servers, CLIs, automation, tooling, desktop workflows, build systems, and more.

Valo is exploring a similar direction for VBA-style programming.

It is designed as a standalone evolution path for Basic/VBA-style development: familiar enough for VBA, VB6, and Visual Basic developers, but with modern language features, a clean runtime, professional diagnostics, modules, FFI, a REPL, release packaging, and a growing standard runtime surface.

Valo supports two complementary modes:

- **`.valo`**: native Valo syntax for modern Basic-style development.
- **`.bas` / `.cls`**: VBA compatibility mode for migrating and running existing Basic-style modules and classes.

Valo is not Office automation. It is a standalone language/runtime that gives Basic-style programming a modern foundation outside of Excel, Access, COM, and the VBA editor.

## Why build Valo?

![Building with Valo](assets/building.png)

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

The long-term goal is simple:

> Take the productivity of VBA out of Office and let it run anywhere.

## Language goals

Valo is designed around the following goals:

- **Familiar Basic-style syntax** for developers coming from VBA, VB6, or VB.NET.
- **First-class VBA compatibility** for `.bas` and `.cls` migration.
- **Modern native syntax** for new `.valo` projects.
- **Standalone runtime** independent of Office, COM, or the VBA editor.
- **Professional diagnostics** with explicit diagnostic codes and source spans.
- **Modular project structure** with imports and qualified symbols.
- **Strong runtime foundations** for arrays, objects, variants, errors, structures, classes, and native types.
- **Experimental native interop** through VBA-style `Declare`, `PtrSafe`, `LongPtr`, callbacks, and `AddressOf`.
- **A practical migration path** from legacy automation code to a modern runtime.
- **A clean implementation architecture** suitable for future tooling, bytecode, and compiled targets.

Valo is still experimental, but its direction is clear: a modern Basic-family language with compatibility as a bridge, not a cage.

## Project status

Valo is currently an experimental language/runtime in active development.

It already includes a substantial interpreter, parser, semantic validator, module loader, diagnostics engine, REPL, CLI, compatibility runtime, native FFI layer, and cross-platform release packaging.

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
- `Interface`, `Implements`, `Shared`, and `Friend`
- classic VBA function return assignment semantics
- modern `Return` syntax
- native `Iterator Function` and `Yield`
- `Try / Catch / Finally`
- VBA `On Error`, `Err`, `Resume`, and `Erl`
- advanced arrays, including multidimensional and jagged patterns
- `Array`, `Split`, `Join`, and `Filter`
- native and VBA-compatible type system
- `Variant`, `Object`, `Empty`, `Null`
- VBA-compatible implicit `Variant` defaults
- `CallByName`
- `Debug.Print`
- `VBA.` namespace fallback
- experimental native FFI through `Declare`, `PtrSafe`, `LongPtr`, and `AddressOf`
- callbacks and native function pointers
- `VarPtr`, `StrPtr`, and `ObjPtr`
- platform-aware native library loading
- structure and array write-back for supported FFI cases
- interactive REPL
- professional diagnostics
- release packaging for Linux, Windows, and macOS
- clippy-clean Rust codebase

Valo is not yet a full production compiler. There is currently no bytecode VM, package manager, LSP, formatter, or complete standard library.

Native FFI is already available experimentally through VBA-style `Declare`, `PtrSafe`, `LongPtr`, callbacks, `AddressOf`, and platform-aware native library loading.

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

    Public Sub New(ByVal x As Integer, ByVal y As Integer)
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

### Classic and modern function returns

```vb
Function Add(ByVal a As Long, ByVal b As Long) As Long
    Add = a + b
End Function

Function AddModern(ByVal a As Long, ByVal b As Long) As Long
    Return a + b
End Function
```

Object returns support classic `Set` semantics:

```vb
Function CreateUser() As User
    Dim u As New User("Valo")
    Set CreateUser = u
End Function
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

### Interfaces, Implements, Shared, and Friend

```vb
Interface IUpdatable
    Sub Update()
End Interface

Class Player Implements IUpdatable
    Friend Shared Count As Long

    Public Sub Update() Implements IUpdatable.Update
        Count = Count + 1
        Debug.Print "Updating player"
    End Sub
End Class

Sub Main()
    Dim p As New Player
    p.Update()

    Debug.Print Player.Count
End Sub
```

### Structures with methods and constructors

```vb
Structure Vec2
    X As Double
    Y As Double

    Public Sub New(ByVal x As Double, ByVal y As Double)
        X = x
        Y = y
    End Sub

    Public Function LengthSquared() As Double
        Return (X * X) + (Y * Y)
    End Function
End Structure

Sub Main()
    Dim v As New Vec2(3#, 4#)

    Debug.Print v.LengthSquared()
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

### Experimental native FFI

Valo can call native libraries through VBA-style `Declare`.

```vb
Declare PtrSafe Function puts Lib "libc" CDecl (
    ByVal value As String
) As Long

Declare PtrSafe Function strlen Lib "libc" CDecl (
    ByVal value As String
) As Long

Private Sub Main()
    puts("Hello from native Valo FFI!")

    Debug.Print strlen("Valo")
End Sub
```

On Windows, Valo can call Win32 APIs:

```vb
Declare PtrSafe Function MessageBoxA Lib "user32" Alias "MessageBoxA" StdCall (
    ByVal hwnd As LongPtr,
    ByVal text As String,
    ByVal caption As String,
    ByVal flags As Long
) As Long

Private Sub Main()
    MessageBoxA(0, "Hello from Valo", "Valo Win32", 0)
End Sub
```

FFI is experimental. Complex COM/OLE interop, full BSTR/SAFEARRAY ownership semantics, mutable string buffers, and Office automation are not currently implemented.

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

The REPL currently supports interactive experimentation. Some declaration-heavy workflows are still evolving.

```txt
valo> Dim x As Integer
valo> x = 10
valo> Console.WriteLine(x)
10
```

## Getting started

Valo is currently experimental, but prebuilt releases are available for supported platforms.

### Install with script

On Linux and macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/valolang/valo/main/scripts/install.sh | bash
```

After installation, add Valo to your PATH if the installer asks you to:

```sh
export PATH="$HOME/.valo/bin:$PATH"
```

Then verify:

```sh
valo version
valo help
```

### Install a specific version

```sh
VALO_VERSION="v0.1.0-2026.05.21" curl -fsSL https://raw.githubusercontent.com/valolang/valo/main/scripts/install.sh | bash
```

### Manual download

You can also download prebuilt binaries from the releases page:

```txt
https://github.com/valolang/valo/releases
```

Available release assets may include:

| Platform | Asset |
|---|---|
| Linux x64 | `valo-linux-x64.tar.gz` |
| Linux x86 | `valo-linux-x86.tar.gz` |
| macOS ARM64 | `valo-macos-arm64.tar.gz` |
| macOS x64 | `valo-macos-x64.tar.gz` |
| Windows x64 | `valo-windows-x64.zip` |
| Windows x86 | `valo-windows-x86.zip` |

Windows users can download the `.zip`, extract it, and run:

```powershell
.\valo.exe version
.\valo.exe run examples\hello.valo
```

### Build from source

You can also build Valo from source.

You need Rust installed.

```sh
git clone https://github.com/valolang/valo
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
runtime values, builtins, and FFI
```

Major components include:

- `frontend`: source processing, lexer, parser, AST, semantic validation, module loading
- `runtime`: values, diagnostics, spans, type names, coercion, numeric operations, comparisons
- `backend`: execution backends
- `backend/interpreter`: current tree-walking interpreter
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
- `Declare`, `PtrSafe`, `LongPtr`, `LongLong`
- `AddressOf`
- `VarPtr`, `StrPtr`, `ObjPtr`
- `VBA.Join`, `VBA.Split`, `VBA.TypeName`, and other namespace fallbacks
- `Variant`, `Object`, `Empty`, `Null`
- implicit `Variant` defaults
- classic function return assignment
- `Array`, `Split`, `Join`, `Filter`
- `LBound`, `UBound`, `ReDim`, `Erase`
- `VarType`, `TypeName`, `IsObject`, `IsArray`, `IsEmpty`, `IsNull`

Compatibility is pragmatic and growing. Valo is not a COM runtime and does not currently implement Office automation, IDispatch, or full COM interop.

Experimental native FFI is supported through VBA-style `Declare`, `PtrSafe`, `LongPtr`, `AddressOf`, callbacks, and platform-aware library loading. Complex COM/OLE interop, full BSTR/SAFEARRAY ownership semantics, and Office automation are still outside the current scope.

## Research directory

The `research/` directory contains real-world VBA/VB-style modules, parser edge cases, runtime stress tests, FFI demos, interoperability experiments, and prototypes used during Valo development.

These files are used to:

- validate VBA compatibility
- reproduce parser/runtime bugs
- stress the interpreter and FFI layer
- benchmark performance
- explore future language features
- prototype APIs and module-system behavior

Files in this directory are experimental and may change frequently as the language evolves.

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
- Office automation
- inheritance
- generics
- full Collection / Dictionary runtime
- async runtime
- complete REPL declaration workflow

Some compatibility areas are intentionally pragmatic today:

- `Currency` and `Decimal` may use simplified mixed arithmetic paths.
- `Rnd` does not yet perfectly replicate every VBA edge case.
- `.cls` metadata is parsed for compatibility, but COM semantics are not implemented.
- FFI intentionally rejects unsafe ownership cases such as mutable string buffers, complex COM/OLE Variant pointers, and nested non-blittable structure layouts.

## Roadmap

Near-term priorities:

- Collection and Dictionary runtime types
- Date and Time standard library
- FileSystem APIs
- richer string/runtime builtins
- compatibility-driven stabilization using real VBA modules
- syntax highlighting
- improved release packaging
- stronger REPL workflows
- import-system refinement

Medium-term priorities:

- formatter
- LSP
- package manifest
- project-level module configuration
- benchmark suite
- standard library organization
- bytecode IR exploration
- field/member dispatch optimization

Long-term directions:

- bytecode VM
- deeper FFI stabilization
- embedding API
- WASM/native backend groundwork
- editor integrations
- migration tooling for VBA projects

## Documentation

Start here:

- [Getting started](docs/getting-started.md)
- [Language syntax](docs/language/syntax.md)
- [Expressions](docs/language/expressions.md)
- [Classes and objects](docs/language/classes.md)
- [Modules and imports](docs/language/modules.md)
- [Error handling](docs/language/error-handling.md)
- [Types](docs/language/types.md)
- [VBA compatibility](docs/language/vba-compat.md)
- [FFI](docs/language/ffi.md)
- [REPL](docs/repl.md)
- [Examples](examples/README.md)

Architecture docs:

- [Frontend](docs/architecture/frontend.md)
- [Backend](docs/architecture/backend.md)
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
- import-system design
- real-world `.bas` / `.cls` compatibility cases

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
