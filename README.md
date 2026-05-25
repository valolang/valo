<div align="center">

  <img src="assets/valo-mascot.png" width="140" alt="Valo mascot">

  # The Valo Programming Language

  A modern Basic-inspired language and runtime with first-class VBA compatibility

</div>

<p align="center">
  <a href="#what-is-valo">What is Valo?</a> |
  <a href="#why-build-valo">Why?</a> |
  <a href="#language-goals">Goals</a> |
  <a href="#project-status">Status</a> |
  <a href="#getting-started">Getting started</a> |
  <a href="#vba-compatibility">VBA compatibility</a> |
  <a href="#documentation">Documentation</a> |
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

The original idea behind Valo was simple:

> What if VBA had its own Node.js moment?

JavaScript was once mostly tied to the browser. Node.js made it possible to use JavaScript almost anywhere: servers, CLIs, automation, tooling, desktop workflows, build systems, and more.

Valo explores a similar direction for VBA-style programming.

It is designed as a standalone evolution path for Basic/VBA-style development: familiar enough for VBA, VB6, and Visual Basic developers, but with modern language features, a clean runtime, professional diagnostics, modules, FFI, a REPL, release packaging, and a growing standard runtime surface.

Valo supports two complementary modes:

| Mode | Purpose |
|---|---|
| `.valo` | Native Valo syntax for modern Basic-style development |
| `.bas` / `.cls` | VBA compatibility mode for migrating existing modules and classes |

Valo is not tied to Office or the VBA editor. It is a standalone runtime designed to modernize Basic-style development while remaining highly compatible with real-world VBA codebases.

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
- a path toward modern tooling, packages, REPL workflows, FFI, and future compiled targets

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

- Familiar Basic-style syntax for developers coming from VBA, VB6, or VB.NET
- First-class VBA compatibility for `.bas` and `.cls` migration
- Modern native syntax for new `.valo` projects
- Standalone runtime independent of the VBA editor
- Professional diagnostics with explicit diagnostic codes and source spans
- Modular project structure with imports and qualified symbols
- Strong runtime foundations for arrays, objects, variants, errors, structures, classes, and native types
- Experimental native interop through VBA-style `Declare`, `PtrSafe`, `LongPtr`, callbacks, `AddressOf`, and related APIs
- A practical migration path from legacy automation code to a modern runtime
- A clean implementation architecture suitable for future tooling and VM/compiler backends

Valo is still experimental, but its direction is clear: a modern Basic-family language with compatibility as a bridge, not a cage.

## Project status

Valo is currently an experimental language/runtime in active development.

It already includes:

- a parser and semantic validator
- a tree-walking interpreter
- modules and imports
- classes, interfaces, inheritance, and generics
- properties, events, and object lifecycle support
- structures and classic VBA `Type`
- deterministic cleanup and `Using`
- VBA-compatible error handling
- a diagnostics engine
- a REPL and CLI
- native FFI support
- VBA compatibility runtime features
- cross-platform release packaging

Valo is not yet a full production compiler. There is currently no bytecode VM, package manager, formatter, or complete standard library.

## Native Valo and VBA compatibility

Valo intentionally separates modern native syntax from compatibility syntax.

### Native Valo

Native `.valo` files use clean, modern Basic-style syntax:

```vb
Class Box(Of T)
    Public Value As T
End Class

Sub Main()
    Dim message As New Box(Of String)

    message.Value = "Hello, Valo"

    Console.WriteLine(message.Value)
End Sub
```

### VBA compatibility

VBA `.bas` and `.cls` files can use compatibility syntax:

```vb
Attribute VB_Name = "Module1"
Option Explicit

Sub Main()
    Debug.Print "Hello from VBA compatibility mode"
End Sub
```

The goal is not to force modern Valo code to use legacy metadata. Instead, Valo accepts VBA metadata where needed for migration while offering cleaner syntax for new code.

For more examples:

- [Examples](examples/README.md)
- [Language docs](docs/language)

## Feature highlights

### Modern control flow

```vb
Try
    DangerousOperation()
Catch ex As Error
    Console.WriteLine(ex.Message)
Finally
    Console.WriteLine("cleanup")
End Try
```

### COM Automation

Valo supports late-bound COM automation, allowing you to control external Windows applications like Excel or FSO just like in VBA.

```vb
Sub Main()
    Dim dict As Object
    Set dict = CreateObject("Scripting.Dictionary")
    
    ' Using default property Item (dict("Key") is same as dict.Item("Key"))
    dict("Name") = "Valo"
    
    Console.WriteLine("Dictionary name: " & dict("Name"))
End Sub
```

### VBA-style error handling

```vb
Sub Main()
    On Error GoTo Handler

    Err.Raise 1001, "Example", "Something failed"

    Exit Sub

Handler:
    Debug.Print Err.Description
    Resume Next
End Sub
```

### Generics and inheritance

```vb
MustInherit Class Animal
    Public MustOverride Sub Speak()
End Class

Class Dog Inherits Animal
    Public Overrides Sub Speak()
        Console.WriteLine("Woof")
    End Sub
End Class
```

### Native FFI

```vb
Declare PtrSafe Function strlen Lib "libc" CDecl (
    ByVal value As String
) As Long

Sub Main()
    Debug.Print strlen("Valo")
End Sub
```

Additional examples are available in the `examples/` directory.

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
valo check examples/generic_box.valo
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

```txt
valo> Dim x As Integer
valo> x = 10
valo> Console.WriteLine(x)
10
```

## Getting started

Valo is currently experimental, but prebuilt releases are available for supported platforms.

### Install with script

Linux and macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/valolang/valo/main/scripts/install.sh | bash
```

After installation:

```sh
export PATH="$HOME/.valo/bin:$PATH"
```

Verify:

```sh
valo version
valo help
```

### Install a specific version

```sh
VALO_VERSION="v0.1.0-2026.05.21" curl -fsSL https://raw.githubusercontent.com/valolang/valo/main/scripts/install.sh | bash
```

### Manual download

Releases:

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

Windows example:

```powershell
.\valo.exe version
.\valo.exe run examples\hello.valo
```

### Build from source

Requirements:

- Rust stable

```sh
git clone https://github.com/valolang/valo
cd valo
cargo build --release
```

Run the CLI:

```sh
./target/release/valo version
```

Windows:

```powershell
.\target\release\valo.exe version
```

### Quality checks

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
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

Valo includes professional diagnostics with:

- explicit diagnostic codes
- source spans
- suggestions
- stack traces
- import-cycle diagnostics
- semantic/member suggestions

Example:

```txt
error[V1100]: Cannot assign String value to Integer variable
 --> examples/demo.valo:4:9
  |
4 |     age = "twenty"
  |         ^^^^^^^^^ expected Integer
```

Diagnostics are designed to be actionable, stable, and suitable for future tooling.

## VBA compatibility

Valo supports a growing set of VBA-compatible features.

Currently supported areas include:

- `.bas` and `.cls` parsing
- exported class modules
- `Attribute VB_Name`
- `Attribute VB_UserMemId`
- classic function assignment semantics
- `On Error`
- `Err`
- `Resume`
- `Erl`
- `Debug.Print`
- `Declare`, `PtrSafe`, `LongPtr`
- `AddressOf`
- `VarPtr`, `StrPtr`, `ObjPtr`
- `Variant`, `Object`, `Empty`, `Null`
- file I/O compatibility runtime
- `Dir`, `EOF`, `LOF`, `FreeFile`
- `Input #`, `Line Input #`
- `Print #`, `Write #`
- `Get #`, `Put #`
- Random/Binary file modes
- imported `.bas` / `.cls` compatibility improvements

Compatibility is pragmatic and growing.

Valo already supports a substantial VBA compatibility surface, including `.bas` / `.cls` modules, classic runtime behavior, and native interop primitives.

Broader COM/OLE automation support, Office object models, and advanced interoperability layers are planned future directions, but are not yet fully implemented.

Experimental native FFI is supported through VBA-style `Declare`, `PtrSafe`, `LongPtr`, `AddressOf`, callbacks, and platform-aware library loading.

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

## Documentation

Start here:

- [Getting started](docs/getting-started.md)
- [Language syntax](docs/language/syntax.md)
- [Expressions](docs/language/expressions.md)
- [Classes and objects](docs/language/classes.md)
- [Inheritance](docs/language/inheritance.md)
- [Generics](docs/language/generics.md)
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

## Contributing

Valo is in active development.

Good areas to contribute:

- language tests
- VBA compatibility cases
- documentation
- examples
- diagnostics
- runtime builtins
- CLI and REPL improvements
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
