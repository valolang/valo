# Valo

[![Status](https://img.shields.io/badge/status-experimental-orange)]()
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/uesleibros/valo?style=flat)](https://github.com/uesleibros/valo/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/uesleibros/valo)](https://github.com/uesleibros/valo/issues)
[![GitHub Last Commit](https://img.shields.io/github/last-commit/uesleibros/valo)](https://github.com/uesleibros/valo/commits/main)

<img align="right" src="assets/valo-mascot.png" width="120px" alt="Valo mascot">

**Valo** is an experimental VBA-inspired language runtime written in Rust. It provides a standalone execution environment for a Basic-style language with typed variables, structured control flow, records, arrays, classes, properties, semantic validation, and runtime diagnostics.

> [!WARNING]
> Valo is experimental. Syntax, semantics, runtime behavior, and internal architecture may change while the language is being designed.

## Why Valo exists

VBA remains useful for automation, internal tools, and small business workflows, but it is tightly coupled to host applications and legacy runtime assumptions.

Valo explores a familiar Basic-style language as an independent runtime. The goal is not to clone VBA exactly, but to keep the practical parts of the programming model while building a portable core with clear diagnostics and room for modern tooling.

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

Valo is currently built from source.

Requirements:

- Rust
- Cargo

Build:

```sh
git clone https://github.com/uesleibros/valo.git
cd valo
cargo build --release
```

Run tests:

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
    Private mActive As Boolean

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value
    End Property

    Public Property Get Active() As Boolean
        Return Me.mActive
    End Property

    Public Property Let Active(ByVal value As Boolean)
        Me.mActive = value
    End Property
End Class

Sub Main()
    Dim user As User
    Set user = New User()

    user.Name = "Valo"
    user.Active = True

    Select Case user.Name
        Case "Valo": Console.WriteLine(user.Name)
        Case Else: Console.WriteLine("unknown")
    End Select

    If Not (user Is Nothing) Then
        Console.WriteLine(user.Active)
    End If
End Sub
```

Output:

```txt
Valo
True
```

## Language features

Valo currently supports:

- Program entry with `Sub Main`.
- `Dim` declarations.
- `String`, `Integer`, `Boolean`, and `Variant`.
- `If` / `ElseIf` / `Else` / `End If`.
- `While` / `Wend`.
- `For` / `Next` / `Step`, including optional `Next i`.
- `Select Case` with values, multiple values, ranges, `Case Is` comparisons, `Case Else`, and single-line `Case ...: statement` bodies.
- `Do While` / `Loop`, `Do Until` / `Loop`, `Do` / `Loop While`, `Do` / `Loop Until`, and `Do` / `Loop`.
- `Exit Sub`, `Exit Function`, `Exit For`, `Exit While`, and `Exit Do`.
- `Function`, `Return`, and callable `Sub`.
- `ByVal` and `ByRef` parameters.
- `And`, `Or`, `Not`, and `Mod`.
- user-defined `Type` records.
- fixed-size arrays.
- `Class`, `New`, `Me`, `Public`, and `Private`.
- instance methods.
- `Property Get`, `Property Let`, and `Property Set`.
- object reference assignment with `Set`.
- `Nothing`.
- `Is` comparisons for object identity and `Nothing` checks.
- `Console.WriteLine`.
- semantic validation and runtime diagnostics.

## Current limitations

Valo does not currently support:

- imports or modules
- standard library modules
- module-level visibility
- dynamic arrays
- `ReDim`
- multidimensional arrays
- `For Each`
- global colon-separated statements
- `Continue`
- `GoTo` or labels
- `On Error`
- async/await
- FFI / `Declare`
- bytecode VM
- package management
- language server
- formatter

## Repository layout

- `cli/` contains the Valo CLI package.
- `core/` contains the language and runtime core.
- `examples/` contains `.valo` examples.
- `assets/` contains mascot and logo files.
- `docs/`, `editors/`, and `tests/` exist for future use.

## Runtime status

The current runtime is a tree-walking interpreter:

```txt
source.valo
  -> lexer
  -> parser
  -> semantic validation
  -> interpreter
```

Current verification status:

- `cargo test` passes with 154 tests.
- `cargo build --release` passes.
- all `.valo` examples pass.
- `examples/hello.bas` passes when present.

A bytecode compiler and stack-based VM are planned, but not implemented.

## Contributing

Valo is in early active development. Contributions, examples, bug reports, and design discussions are welcome.

Before working on large language features, please open an issue or discussion so implementation work stays aligned with the language direction.

## License

[MIT](LICENSE)
