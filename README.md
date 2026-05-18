# Valo

[![Status](https://img.shields.io/badge/status-experimental-orange)]()
[![Rust](https://img.shields.io/badge/built%20with-Rust-b7410e)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/uesleibros/valo?style=social)](https://github.com/uesleibros/valo/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/uesleibros/valo)](https://github.com/uesleibros/valo/issues)
[![GitHub Last Commit](https://img.shields.io/github/last-commit/uesleibros/valo)](https://github.com/uesleibros/valo/commits/main)

<img align="right" src="assets/valo-mascot.png" height="120px" alt="Valo mascot">

> [!WARNING]
> Valo is in the earliest stages of development. This repository is being actively built. The language syntax, runtime behavior, and APIs will change frequently.

**Valo** is an experimental runtime for VBA-inspired code, written in Rust.

The vision: liberate VBA from Microsoft Office and give it a modern execution environment with async/await, modules, FFI, and true cross-platform support.

## Why Valo exists

VBA taught millions to program. It powers business-critical automation in countless organizations. But it's trapped inside Office, bound to Windows, and hasn't evolved in decades.

Valo explores what VBA could become: a standalone, cross-platform runtime with modern features while keeping the familiar Basic syntax people already know.

## Planned features

Valo aims to support:

**Modern language features**
- Async/await for non-blocking I/O
- Module system with local and remote imports
- Generics (List, Dictionary)
- Lambdas and LINQ
- Try/Catch error handling

**Runtime capabilities**
- Cross-platform execution (Windows, Linux, macOS)
- FFI via Declare statements
- Bytecode compilation
- Stack-based virtual machine
- Standard library (http, fs, path, process)

**Developer experience**
- Language server protocol
- Code formatter
- Package manager
- Clear diagnostics

## Example syntax

This is what Valo code will look like:

```vb
Import "http"

Async Sub Main()
    Dim server = http.CreateServer()
    
    server.Get("/", Async Function(req, res)
        Await res.Send("Hello from Valo!")
    End Function)
    
    Console.WriteLine("Server running at http://localhost:3000")
    Await server.Listen("localhost", 3000)
End Sub
```

> [!NOTE]
> This example represents the target syntax. Implementation is in progress.

## Project status

Valo is currently in the foundation stage. Active work includes:

- Language design and syntax finalization
- Lexer and parser implementation
- Runtime architecture
- Core type system
- Semantic validation

This is an experiment in giving VBA the runtime it deserves. Breaking changes are expected as the language evolves.

## Philosophy

Valo is guided by a few principles:

**Familiarity over novelty.** Millions know VBA. Evolution beats starting from scratch.

**Modern features are essential.** Async, modules, and FFI aren't optional in 2025.

**Cross-platform by design.** Windows-only is a non-starter.

**Keep it simple.** VBA's accessibility is a strength, not something to fix.

## Contributing

> [!IMPORTANT]
> Valo is in very early development. The best way to contribute right now is to watch the repository and participate in discussions as development progresses.

If you're interested in shaping the language, open a discussion or check back as the project evolves.

## License

[MIT](LICENSE)
