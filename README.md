# Valo

[![Status](https://img.shields.io/badge/status-experimental-orange)]()
[![Rust](https://img.shields.io/badge/built%20with-Rust-b7410e)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/uesleibros/valo?style=social)](https://github.com/uesleibros/valo/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/uesleibros/valo)](https://github.com/uesleibros/valo/issues)
[![GitHub Last Commit](https://img.shields.io/github/last-commit/uesleibros/valo)](https://github.com/uesleibros/valo/commits/main)

<img align="right" src="assets/valo-mascot.png" height="120px" alt="Valo mascot">

> [!WARNING]
> Valo is experimental. The language syntax, runtime behavior, and internal architecture may change frequently.

**Valo** is a VBA-inspired language runtime written in Rust.

It provides a standalone execution environment for a Basic-style language with typed variables, structured control flow, functions, records, arrays, classes, semantic validation, and runtime diagnostics.

The long-term goal is to explore what a modern VBA-like runtime could look like outside Microsoft Office: cross-platform, lightweight, modular, and designed for modern tooling.

## Why Valo exists

VBA has been used for decades to build automation, internal tools, business logic, and productivity workflows.

The problem is not only the language. The problem is the host.

Valo explores a different direction: a familiar Basic-style language running as a standalone runtime, without depending on Excel, Access, Word, COM, or the Office macro environment.

Valo is not intended to be a perfect VBA clone. It is a modern runtime inspired by VBA and Basic-style programming.

## Quick start

Create a file called `hello.valo`:

```vb
Sub Main()
    Console.WriteLine("Hello from Valo")
End Sub
```

Run it:

```sh
valo run hello.valo
```

Output:

```txt
Hello from Valo
```

## Installation

Valo is currently distributed as a standalone executable during early testing.

On Windows:

```powershell
valo.exe run examples\hello.valo
```

If `valo.exe` is available in your `PATH`:

```powershell
valo run examples\hello.valo
```

## Build from source

Requirements:

- [Rust](https://www.rust-lang.org/)
- Cargo

Clone the repository:

```sh
git clone https://github.com/uesleibros/valo.git
cd valo
```

Build:

```sh
cargo build --release
```

Run the test suite:

```sh
cargo test
```

Run an example:

```sh
target/release/valo run examples/hello.valo
```

## Example

```vb
Class User
    Public Name As String
    Private Age As Integer
    Private Active As Boolean

    Public Sub Initialize(ByVal name As String, ByVal age As Integer)
        Me.Name = name
        Me.Age = age
        Me.Active = True
    End Sub

    Public Function IsAdult() As Boolean
        Return Me.Age >= 18
    End Function

    Public Sub Deactivate()
        Me.Active = False
    End Sub

    Public Function IsActive() As Boolean
        Return Me.Active
    End Function
End Class

Sub Main()
    Dim user As User
    user = New User("Valo", 1)

    Console.WriteLine(user.Name)
    Console.WriteLine(user.IsAdult())
    Console.WriteLine(user.IsActive())

    user.Deactivate()
    Console.WriteLine(user.IsActive())
End Sub
```

Run it:

```sh
valo run examples/classes.valo
```

## Current language support

Valo currently supports:

- `Sub Main`
- `Dim` declarations
- `String`, `Integer`, `Boolean`, and `Variant`
- `If`, `ElseIf`, `Else`, `End If`
- `While` / `Wend`
- `For` / `Next` / `Step`
- `Function` and `Return`
- callable `Sub`
- `ByVal` and `ByRef`
- `And`, `Or`, `Not`, and `Mod`
- user-defined `Type` records
- fixed-size arrays
- `Class`
- `New`
- `Me`
- public and private class members
- instance methods
- `Console.WriteLine`
- semantic validation
- runtime diagnostics

## Current limitations

Valo does not currently support:

- imports or modules
- standard library modules
- properties
- module-level visibility
- `Select Case`
- `Do` / `Loop`
- `Exit` statements
- dynamic arrays
- `ReDim`
- multidimensional arrays
- `For Each`
- async/await
- FFI / `Declare`
- bytecode compilation
- package management
- language server
- formatter

## Runtime model

The current implementation uses a tree-walking interpreter while the language core is being designed and stabilized.

The long-term runtime direction is:

```txt
source.valo
  -> lexer
  -> parser
  -> semantic validation
  -> bytecode compiler
  -> stack-based virtual machine
```

The bytecode compiler and VM are not implemented yet.

## Planned direction

Valo aims to grow toward:

- local modules and imports
- standard library modules such as `fs`, `path`, `process`, and `http`
- FFI through `Declare` statements
- async/await for non-blocking I/O
- bytecode execution
- formatter
- language server
- editor integrations
- WebAssembly playground

## Design principles

Valo is guided by a few principles:

**Familiar syntax.** Basic-style code is readable, approachable, and familiar to many developers.

**Standalone runtime.** Valo code should run outside the Office environment.

**Modern runtime capabilities.** Modules, async I/O, FFI, diagnostics, tooling, and cross-platform execution are core goals.

**Simple language surface.** Valo should remain easy to read and write, even as the runtime becomes more capable.

## Project status

Valo is in early active development.

The current focus is correctness, language semantics, diagnostics, and stabilizing the core runtime behavior. Breaking changes are expected during the experimental stage.

## Contributing

Valo is still evolving quickly.

Contributions, examples, bug reports, and design discussions are welcome. Before working on large language features, please open an issue or discussion so the language direction can stay consistent.

## License

[MIT](LICENSE)
