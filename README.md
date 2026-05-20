# Valo

Valo is a modern, Basic-inspired programming language and runtime designed for high-integrity automation and professional scripting. It balances the approachability of VBA with modern language features, modularity, and a robust diagnostics system.

## Key Features

- **Modern Basic Syntax**: A clean, readable syntax that feels familiar to Basic developers but includes modern constructs.
- **Dual Mode Support**:
    - `.valo`: Native mode with modern features like `Try/Catch/Finally`.
    - `.bas` / `.cls`: Compatibility mode for traditional VBA source code.
- **Robust Object Model**:
    - Support for Classes, Events, and Properties.
    - Deterministic lifecycle management with `Constructor` and `Terminate`.
    - Default properties and intuitive indexer syntax.
- **Advanced Error Handling**: Supports both traditional `On Error` and modern `Try/Catch/Finally` blocks.
- **Powerful Array System**: Full support for multidimensional arrays, dynamic resizing with `ReDim Preserve`, and built-ins like `Array()`, `Split()`, `Join()`, and `Filter()`.
- **Modular Architecture**: Comprehensive support for modules, imports, and qualified symbol access.
- **Professional Diagnostics**: Rich, descriptive error messages with source code highlighting and helpful suggestions.

## Installation

Valo is written in Rust. To build it, ensure you have the [Rust toolchain](https://rustup.rs/) installed.

```bash
git clone https://github.com/valo-lang/valo.git
cd valo
cargo build --release
```

## Usage

You can run Valo files using the CLI:

```bash
./target/release/valo run examples/hello.valo
```

To run a project with multiple modules:

```bash
./target/release/valo run examples/modules/main.valo
```

## Examples

### Modern Try/Catch (`.valo`)

```vb
Sub Main()
    Try
        Dim data = Array(1, 2, 3)
        Console.WriteLine("Element 0: " & data(0))
    Catch ex As Error
        Console.WriteLine("Error: " & ex.Message)
    Finally
        Console.WriteLine("Cleanup performed")
    End Try
End Sub
```

### VBA Compatibility (`.bas`)

```vb
Attribute VB_Name = "LegacyModule"

Public Sub Main()
    On Error GoTo ErrorHandler
    Dim parts
    parts = Split("A,B,C", ",")
    Debug.Print parts(1)
    Exit Sub

ErrorHandler:
    MsgBox Err.Description
End Sub
```

## Documentation

- [Language Reference](docs/language/README.md)
- [Architecture Overview](docs/architecture/README.md)
- [Roadmap](docs/architecture/roadmap.md)

## Quality and Integrity

Valo is committed to high engineering standards:

- **Comprehensive Testing**: Hundreds of unit and integration tests covering parser, semantics, and runtime.
- **Linted Codebase**: Maintained as `clippy-clean` with zero warnings.
- **Deterministic Runtime**: Reliable execution and resource management.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
