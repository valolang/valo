<div align="center">

  <img src="assets/valo-mascot.png" width="140" alt="Valo mascot">

  # The Valo Programming Language

  A modern Basic-inspired language and runtime with first-class VBA compatibility

</div>

<p align="center">
  <a href="#what-is-valo">What is Valo?</a> |
  <a href="#why-build-valo">Why?</a> |
  <a href="#installation">Installation</a> |
  <a href="#getting-started">Getting Started</a> |
  <a href="#documentation">Documentation</a> |
  <a href="#contributing">Contributing</a>
</p>

## Installation

Valo is a modern, Basic-inspired runtime. You can install it using one of the following commands:

### Linux / macOS
```bash
curl -fsSL https://raw.githubusercontent.com/valolang/valo/main/scripts/install.sh | bash
```

### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/valolang/valo/main/scripts/install.ps1 | iex
```

### Setup Directory
Valo installs its runtime components under `~/.valo/` (or `%USERPROFILE%\.valo` on Windows):
- `bin/`: The Valo CLI executable.
- `cache/`: Downloaded packages and build artifacts.
- `packages/`: Global library dependencies.
- `toolchains/`: Versioned runtime environments.

## Getting Started

1. **Verify Installation:**
   ```bash
   valo version
   ```

2. **Quick Start:**
   Create a `hello.valo` file:
   ```vb
   Sub Main()
       Console.WriteLine("Hello, Valo!")
   End Sub
   ```
   Run your program:
   ```bash
   valo run hello.valo
   ```

3. **Explore:**
   Start an interactive REPL:
   ```bash
   valo repl
   ```

---

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
- modules, imports, namespaces, and VB.NET-style `Module ... End Module` blocks
- classes, interfaces, inheritance, structures, properties, events, and lifecycle hooks
- generics for classes, structures, functions, and methods, including nested generic type names
- parser support for VB.NET-style generic variance and constraint syntax
- structures and classic VBA `Type`
- deterministic cleanup and `Using`
- VBA-compatible error handling
- a diagnostics engine
- a REPL and CLI
- native FFI support
- Windows COM/OLE Automation support through late-bound `Object`, `CreateObject`, and default-property dispatch
- VBA compatibility runtime features
- cross-platform release packaging

Valo is not yet a full production compiler. There is currently no bytecode VM, package manager, formatter, or complete standard library. Some compatibility features are intentionally pragmatic and still evolving.

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
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for more information.

## License

Valo is licensed under the [MIT License](LICENSE).
