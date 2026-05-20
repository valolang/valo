# Valo

[![Status](https://img.shields.io/badge/status-active-brightgreen)]()
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/uesleibros/valo/actions/workflows/rust.yml/badge.svg)](https://github.com/uesleibros/valo/actions)

<img align="right" src="assets/valo-mascot.png" width="140px" alt="Valo mascot">

**Valo** is a modern, high-performance Basic-inspired language and runtime built in Rust, featuring first-class VBA compatibility.

It bridges the gap between the productive, familiar world of Basic and the performance and safety requirements of modern software development. Valo provides a standalone, portable runtime that can execute modern `.valo` source files or existing VBA `.bas` and `.cls` projects with advanced semantic validation and professional tooling.

## Why Valo?

VBA has been one of the most productive programming models for decades, yet it remains tethered to host applications and legacy environments. Valo modernizes this paradigm:

-   **Standalone Runtime:** Decouples Basic from Microsoft Office and Windows, running anywhere Rust does.
-   **Modern Language Features:** Adds structured imports, native constructors/destructors, first-class properties, and robust error handling.
-   **Strict Semantic Validation:** Catches type mismatches and logical errors before execution, reducing runtime bugs.
-   **Professional Tooling:** Features world-class diagnostics inspired by Rust and Zig, designed for developer efficiency.

## Native Valo vs. VBA Compatibility

Valo offers a dual-path strategy for development:

| Feature | **Modern Native (`.valo`)** | **VBA Compatibility (`.bas`/`.cls`)** |
| :--- | :--- | :--- |
| **Lifecycle** | `Constructor()` / `Terminate()` | `Class_Initialize` / `Class_Terminate` |
| **Defaults** | `Public Default Property Get Item()` | `Attribute Item.VB_UserMemId = 0` |
| **Metadata** | Not needed | Full `Attribute VB_*` support |
| **Imports** | Explicit `Import Math` | Automatically shared namespace |

## Key Features

-   **Modular System:** Structured module resolution and dependency management via `Import`.
-   **Object-Oriented:** Comprehensive class support with `Public`/`Private` visibility, events, and properties.
-   **Indexer Ergonomics:** Native support for default properties allowing `collection(index)` style access.
-   **Memory Management:** Automatic reference counting for predictable object lifetimes.
-   **Error Handling:** Full `On Error GoTo` and `Resume` support, integrated with `Err` object.
-   **Metaprogramming:** Built-in `#If` / `#Const` preprocessor for conditional compilation.
-   **Diagnostic Engine:** Descriptive, colorized error messages with searchable codes and help hints.

## Showcase

### Modern Native Syntax (`.valo`)
```vb
Public Class Rectangle
    Private m_width As Double
    Private m_height As Double

    Public Constructor(ByVal w As Double, ByVal h As Double)
        Me.m_width = w
        Me.m_height = h
    End Constructor

    Public Default Property Get Area() As Double
        Return Me.m_width * Me.m_height
    End Property
End Class

Sub Main()
    Dim r As New Rectangle(5, 10)
    Console.WriteLine("Area: " & r) ' Calls default property
End Sub
```

### VBA Compatibility (`.cls`)
```vb
VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "LegacyItem"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False

Private Sub Class_Initialize()
    ' Runs on construction
End Sub

Public Property Get Value() As String
Attribute Value.VB_UserMemId = 0
    Value = "Compatibility Works"
End Property
```

### Professional Diagnostics
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
Valo is built using the Rust toolchain.

```bash
# Clone the repository
git clone https://github.com/uesleibros/valo.git
cd valo

# Build in release mode
cargo build --release

# Run an example
./target/release/valo run examples/native_default_property.valo
```

## Project Documentation

Detailed documentation is available in the `docs/` directory:

-   **[Language Reference](docs/language/README.md):** Syntax, Classes, Modules, and Error Handling.
-   **[Architecture Guide](docs/architecture/README.md):** Deep dives into the Parser, Runtime, and Diagnostics system.
-   **[VBA Compatibility Guide](docs/language/vba-compat.md):** Understanding the bridge layer and migration path.

## Project Status

Valo is under active development. Current focus is on stabilizing the core runtime and expanding the standard library.

### Roadmap
- [ ] Transition to Bytecode VM for performance
- [ ] Expanded Standard Library (JSON, File I/O)
- [ ] Language Server Protocol (LSP) for IDE integration
- [ ] Automated Formatter and Linter
- [ ] FFI Support for native interop

## Contributing

We welcome contributions of all kinds! Please see our [Contributing Guide](CONTRIBUTING.md) to get started.

## License

Valo is released under the [MIT License](LICENSE).
