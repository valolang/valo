# Valo

[![Status](https://img.shields.io/badge/status-experimental-orange)]()
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/uesleibros/valo?style=flat)](https://github.com/uesleibros/valo/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/uesleibros/valo)](https://github.com/uesleibros/valo/issues)
[![GitHub Last Commit](https://img.shields.io/github/last-commit/uesleibros/valo)](https://github.com/uesleibros/valo/commits/main)

<img align="right" src="assets/valo-mascot.png" width="140px" alt="Valo mascot">

**Valo** is a modern, high-performance VBA-inspired language and runtime written in Rust. It brings the familiar, productive programming model of Basic to a portable, standalone environment with advanced type safety, structured control flow, and professional developer tooling.

Valo is designed for developers who appreciate the simplicity of Basic but demand the reliability and performance of modern runtimes.

> [!WARNING]
> Valo is currently experimental. While already surprisingly capable, syntax and internal architecture are subject to refinement as we work towards a stable 1.0.

## Philosophy

VBA has powered business automation for decades, yet it remains trapped within host applications and legacy environments. Valo liberates this productive paradigm:

- **Portable Core:** A standalone runtime that runs anywhere Rust does.
- **Modern Tooling:** High-quality diagnostics, semantic validation, and a focus on developer experience.
- **Familiar but Better:** Keeps the practical parts of VBA while removing legacy friction and adding modern features like first-class properties, structured error handling, and conditional compilation.
- **Performance:** Built with Rust for safety and speed, evolving towards a high-performance bytecode VM.

## Key Features

- **Object-Oriented:** Classes with `Public`/`Private` visibility, constructors (`Initialize`), and first-class `Property Get/Let/Set`.
- **Dynamic Memory:** Robust support for dynamic arrays with `ReDim` and `ReDim Preserve`.
- **Advanced Control Flow:** `For Each` iteration, `With` blocks for ergonomic member access, and `Select Case` with ranges and comparisons.
- **Modern Basic:** `Enum` support, module-level variables/constants, and `Option Base`/`Option Compare` for flexible behavior.
- **Reliable Error Handling:** `On Error GoTo` and `Resume` support, integrated with modern runtime diagnostics.
- **Metaprogramming:** Built-in `#Const` and `#If` conditional compilation for cross-platform or feature-gated code.
- **Professional Diagnostics:** Rich, descriptive error messages inspired by Rust and Zig.

## Showcase

### Classes & Properties
Valo features a modern class system that feels familiar to VBA developers but operates with strict semantic validation.

```vb
Class User
    Private mName As String
    Public Age As Integer

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value
    End Property

    Public Sub Birthday()
        Me.Age = Me.Age + 1
    End Sub
End Class

Sub Main()
    Dim u As User
    Set u = New User()
    With u
        .Name = "Valo"
        .Age = 1
        Call .Birthday()
        Console.WriteLine(.Name & " is " & .Age)
    End With
End Sub
```

### Dynamic Arrays
Manage memory efficiently with dynamic arrays and preservation.

```vb
Sub Main()
    Dim values() As Integer
    ReDim values(2)
    
    values(0) = 10: values(1) = 20: values(2) = 30

    ' Resize while keeping existing data
    ReDim Preserve values(4)
    values(3) = 40: values(4) = 50

    For Each v In values
        Console.WriteLine(v)
    Next v
End Sub
```

### Conditional Compilation
Use `#If` to tailor your code for different environments or configurations.

```vb
#Const Target = "WEB"

Sub Main()
#If Target = "WEB" Then
    Console.WriteLine("Running in Browser")
#Else
    Console.WriteLine("Running Native")
#End If
End Sub
```

### Error Handling & Diagnostics
Valo provides a robust error handling model combined with world-class error reporting.

```vb
Sub Main()
    On Error GoTo Handler
    Dim x As Integer: x = 1 / 0 ' Triggers error
    Exit Sub

Handler:
    Console.WriteLine("Error: " & Err.Description)
    Resume Next
End Sub
```

**Diagnostic Example:**
```txt
error[V1100]: Cannot assign String value to Integer variable
  --> script.valo:3:3
   |
3 |   x = "string"
   |   ^^^^^^^^^^^^ expected Integer, found String
   |
help: change the variable type or assign a value with the expected type
```

## Getting Started

### Installation
Valo is currently built from source using the Rust toolchain.

```bash
# Clone the repository
git clone https://github.com/uesleibros/valo.git
cd valo

# Build the project
cargo build --release

# Run an example
./target/release/valo run examples/hello.valo
```

### Quick Start
Create a file named `main.valo`:

```vb
Sub Main()
    Console.WriteLine("Hello, Valo")
End Sub
```

Run it with:
```bash
valo run main.valo
```

## VBA Compatibility

Valo is **VBA-inspired**, not a bug-for-bug compatible clone. Our goal is to preserve the high productivity and familiar syntax of the Basic programming model while building a modern foundation:

- **Strict Validation:** Valo performs semantic analysis before execution, catching type mismatches and scope errors that VBA might only find at runtime.
- **Standalone:** No dependency on Excel, Access, or Windows-only APIs.
- **Modern Semantics:** Cleanup of legacy Basic quirks while keeping the "spirit" of the language intact.

## Architecture

Valo is built as a pipeline of specialized stages to ensure correctness and performance:

1. **Lexer:** Scans source into tokens.
2. **Parser:** Builds an Abstract Syntax Tree (AST), supporting complex Basic constructs.
3. **Preprocessor:** Handles conditional compilation directives (`#If`, `#Const`).
4. **Semantic Validation:** Performs rigorous type checking and symbol resolution before execution.
5. **Interpreter:** A high-fidelity tree-walking interpreter (evolving toward a Bytecode VM).

## Development

Valo is backed by an extensive integration test suite that verifies every example in the repository.

```bash
cargo test
```

## Roadmap

- [ ] Bytecode Compiler & Virtual Machine
- [ ] Standard Library (File I/O, Networking, JSON)
- [ ] Language Server Protocol (LSP) support
- [ ] Formatter and Linter
- [ ] FFI / `Declare` support for native interop

## Contributing

We welcome contributions! Whether it's reporting bugs, suggesting features, or submitting pull requests, your help is appreciated. Please see our [Contributing Guide](CONTRIBUTING.md) for more details.

## License

Valo is released under the [MIT License](LICENSE).
