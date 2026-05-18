# Valo

[![Status](https://img.shields.io/badge/status-experimental-orange)]()
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/uesleibros/valo?style=flat)](https://github.com/uesleibros/valo/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/uesleibros/valo)](https://github.com/uesleibros/valo/issues)
[![GitHub Last Commit](https://img.shields.io/github/last-commit/uesleibros/valo)](https://github.com/uesleibros/valo/commits/main)

<img align="right" src="assets/valo-mascot.png" width="120px" alt="Valo mascot">

**Valo** is an experimental VBA-inspired language runtime written in Rust.

Valo provides a standalone execution environment for a Basic-style language with typed variables, structured control flow, functions, records, arrays, classes, properties, semantic validation, and runtime diagnostics.

> [!WARNING]
> Valo is experimental. Syntax, semantics, runtime behavior, and internal architecture may change while the language is being designed.

## Why Valo exists

VBA has been used for decades for automation, internal tools, business logic, and productivity workflows. Its biggest limitation is often the host environment rather than the language style.

Valo explores a standalone runtime for familiar Basic-style programming without depending on Excel, Access, Word, COM, or the Office macro environment.

Valo is not a VBA clone. It is a modern runtime inspired by VBA, with a focus on portability, clear diagnostics, and tooling-friendly language design.

## Quick start

Create `hello.valo`:

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

Valo is currently built from source during early development.

Requirements:

- Rust
- Cargo

Clone and build:

```sh
git clone https://github.com/uesleibros/valo.git
cd valo
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

On Windows:

```powershell
target\release\valo.exe run examples\hello.valo
```

## Example

```vb
Class User
    Private mName As String
    Private mAge As Integer

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value
    End Property

    Public Property Get Age() As Integer
        Return Me.mAge
    End Property

    Public Property Let Age(ByVal value As Integer)
        If value < 0 Then
            Me.mAge = 0
        Else
            Me.mAge = value
        End If
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()

    user.Name = "Valo"
    user.Age = -1

    Console.WriteLine(user.Name)
    Console.WriteLine(user.Age)
End Sub
```

Run it:

```sh
valo run examples/properties.valo
```

Expected output:

```txt
Valo
0
```

## Project structure

The repository uses a Deno-inspired runtime layout:

- `cli/` contains the CLI package and the `valo` executable.
- `core/` contains the language and runtime core.
- `examples/` contains Valo examples.
- `assets/` contains mascot and logo files.
- `docs/`, `editors/`, and `tests/` exist for future use.

## Current language support

Valo currently supports:

- `Sub Main`
- `Dim` declarations
- `String`, `Integer`, `Boolean`, and `Variant`
- `If` / `ElseIf` / `Else` / `End If`
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
- `Public` and `Private` class members
- instance methods
- `Property Get`, `Property Let`, and `Property Set`
- `Console.WriteLine`
- semantic validation
- runtime diagnostics

## Current limitations

Valo does not currently support:

- imports or modules
- standard library modules
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
- bytecode VM
- package management
- language server
- formatter

## Runtime status

Valo currently uses a tree-walking interpreter. The implemented pipeline is:

```txt
source.valo
  -> lexer
  -> parser
  -> semantic validation
  -> interpreter
```

The runtime includes semantic checks and diagnostics for common language errors before execution. A bytecode compiler and virtual machine are design goals, but they are not implemented.

## Design goals

- Keep Basic-style syntax readable and approachable.
- Run Valo programs outside the Office macro environment.
- Provide clear parser, semantic, and runtime diagnostics.
- Keep the runtime modular, with a small CLI and a reusable language core.
- Grow toward modern tooling, editor integration, formatting, modules, and package management.

## Contributing

Valo is in early active development. Contributions, examples, bug reports, and design discussions are welcome.

Before working on large language features, please open an issue or discussion so implementation work stays aligned with the language direction.

## License

[MIT](LICENSE)
