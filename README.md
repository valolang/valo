# Valo

[![Status](https://img.shields.io/badge/status-alpha-orange)]()
[![Rust](https://img.shields.io/badge/built%20with-Rust-b7410e)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

<img align="right" src="assets/valo-mascot.png" height="150px" alt="Valo mascot">

**Valo** is an experimental VBA-inspired runtime written in Rust.

It provides a standalone execution environment for a Basic-style language with
typed variables, structured control flow, functions, records, classes, arrays,
semantic validation, and runtime diagnostics.

Valo is currently in early alpha and is not production-ready.

## Overview

Valo explores a modern execution model for a familiar Basic-like syntax outside
the Microsoft Office host environment.

The project currently focuses on the language core. The runtime is implemented
as a tree-walking interpreter while the syntax, semantic model, and runtime
behavior are being stabilized.

Long-term goals include bytecode compilation, a stack-based virtual machine,
local modules, tooling, and a standard library.

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

- Rust
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

Run:

```sh
valo run examples/classes.valo
```

## Supported language features

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

## Current limitations

The following features are not implemented yet:

- imports and modules
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

## Project status

Valo is under active development.

The current focus is correctness, language semantics, diagnostics, and runtime
behavior. Breaking changes are expected during the alpha stage.

## License

[MIT](LICENSE)
