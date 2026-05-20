# Valo

<p align="center">
  <img src="assets/valo-mascot.png" width="200" alt="Valo Logo">
</p>

<p align="center">
  <a href="https://github.com/uesleibros/valo/actions"><img src="https://img.shields.io/github/actions/workflow/status/uesleibros/valo/test.yml?branch=main" alt="Build Status"></a>
  <a href="https://github.com/uesleibros/valo/blob/main/LICENSE"><img src="https://img.shields.io/github/license/uesleibros/valo" alt="License"></a>
</p>

Valo is a modern, Basic-inspired programming language and runtime designed for high-integrity automation and professional scripting. It provides a clean, robust environment that bridges the gap between classic VBA compatibility and modern, type-safe development.

## 🚀 Key Features

*   **Dual-Mode Runtime**: Seamlessly run modern `.valo` code or integrate legacy `.bas` / `.cls` VBA files.
*   **Modern Syntax**: Full support for native features like `Try/Catch/Finally`, multidimensional arrays, and strong typing.
*   **Advanced Type System**: From standard numeric types (`Byte`, `Int64`, `Decimal`) to modern system types like `Ptr` and `FuncPtr`.
*   **Professional Diagnostics**: Rich, actionable error messages with source mapping.
*   **Modular Architecture**: Designed for maintainability, with built-in modules, import systems, and a professional runtime interface.

## 📦 Installation

Valo requires Rust. Build it from source:

```bash
git clone https://github.com/uesleibros/valo.git
cd valo
cargo build --release
```

## 💻 CLI Usage

The Valo CLI provides a professional toolset for development and experimentation.

```bash
# Run a file
valo run examples/hello.valo

# Start the interactive REPL
valo repl

# Validate a file for errors
valo check examples/types.valo
```

## 📖 Documentation

Explore the following to get started:

- [Getting Started](docs/getting-started.md)
- [Language Reference](docs/language/README.md)
- [VBA Compatibility Guide](docs/language/vba-compat.md)

## 🛠️ Showcase

### Modern Native (`.valo`)
```vb
Try
    Dim matrix(1 To 3, 1 To 2) As Integer
    matrix(1, 1) = 42
    Console.WriteLine("Matrix(1, 1): " & matrix(1, 1))
Catch ex As Error
    Debug.Print "Error: " & ex.Message
End Try
```

### VBA Compatibility (`.bas`)
```vb
Public Sub Main()
    On Error Resume Next
    Dim parts
    parts = Split("A,B,C", ",")
    Debug.Print Join(parts, "-")
End Sub
```

## 🛣️ Roadmap

- [x] VBA Runtime Compatibility
- [x] Multidimensional Arrays
- [x] Modern Type System Expansion
- [x] Interactive REPL
- [ ] Collection / Dictionary Libraries
- [ ] FFI Layer
- [ ] FileSystem Standard Library

---
Valo is licensed under the [MIT License](LICENSE).
github.com/uesleibros/valo
