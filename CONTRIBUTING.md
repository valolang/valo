# Contributing to Valo

Thank you for helping improve Valo. The project is experimental, but we keep changes disciplined: small reproductions, focused tests, clear diagnostics, and cross-platform behavior matter.

## Ways to Contribute

### Bug Reports

Open an issue with:

- A minimal `.valo`, `.bas`, or `.cls` reproduction.
- The expected behavior and the actual output or diagnostic.
- Your OS, Rust version, and how you ran Valo.
- Any platform-specific context, especially for COM, native FFI, paths, or file I/O.

Good reports include the smallest program that fails. For parser and semantic bugs, a short source snippet is usually enough.

### Feature Requests

Feature requests are welcome when they include:

- The syntax or API being proposed.
- A concrete example program.
- Whether the feature is meant to match VBA, VB6, VB.NET, or native Valo behavior.
- Known compatibility tradeoffs.

Valo is a modern Basic-family language with strong VBA compatibility, but compatibility is a bridge, not a requirement to preserve every legacy limitation.

### Pull Requests

Before opening a PR:

1. Fork the repository.
2. Create a focused branch.
3. Keep unrelated refactors out of the PR.
4. Add or update tests for language behavior changes.
5. Update documentation when user-facing syntax, CLI behavior, diagnostics, or compatibility changes.
6. Run the full validation commands below.

Use clear commit messages, for example:

```txt
Fix COM default property access with string keys
Add parser coverage for generic class headers
Document Module block semantics
```

## Development Setup

Valo is a Rust workspace using the stable toolchain.

```sh
git clone https://github.com/valolang/valo
cd valo
cargo build
```

The workspace contains:

- `core/`: lexer, parser, AST, semantic validation, runtime, interpreter, FFI, COM bridge, and tests.
- `cli/`: the `valo` command-line interface.
- `examples/`: executable examples used by the examples integration test.
- `docs/`: language and architecture documentation.
- `research/`: experimental and real-world compatibility material.

## Validation

Run these before submitting a PR:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

For release-sensitive changes, also run:

```sh
cargo build --release
```

CI runs on multiple platforms. Avoid adding tests that only pass on one OS unless they are explicitly guarded with `#[cfg(...)]` or the test harness skips them intentionally.

## Test Guidance

Choose tests based on the behavior you changed:

- Lexer/parser syntax: add tests under `core/src/frontend/parser/tests.rs` or parser-specific test modules.
- Semantic validation: add targeted validation tests near the relevant semantic/runtime test group.
- Interpreter behavior: add tests under `core/src/backend/interpreter/tests/`.
- Module/import behavior: use `core/src/backend/interpreter/tests/modules.rs`.
- VBA compatibility: use `core/src/backend/interpreter/tests/vba_compat.rs`.
- Examples: add or update files in `examples/` only when the example should be runnable by the integration test.

The examples integration test runs `.valo` and `.bas` files in `examples/`. Windows-only examples, such as COM automation, need platform-aware handling.

## Platform-Specific Features

### COM/OLE Automation

COM support is Windows-only. Code using `CreateObject`, `IDispatch`, or COM default properties must:

- Compile on non-Windows through `#[cfg(windows)]` and non-Windows stubs.
- Produce clear diagnostics when runtime behavior is unavailable off Windows.
- Include semantic tests that can run cross-platform when possible.
- Include Windows behavior tests only when the environment dependency is stable enough for CI.

### Native FFI

FFI behavior varies by OS, architecture, library availability, and calling convention. Prefer small tests that use stable system libraries and guard platform-specific cases.

## Code Style

- Follow standard Rust idioms and `rustfmt`.
- Keep diagnostics actionable: include source spans, clear messages, and suggestions when useful.
- Preserve existing parser and interpreter patterns unless a broader refactor is necessary.
- Prefer focused changes over sweeping rewrites.
- Keep comments short and useful; avoid comments that repeat the code.
- Do not weaken validation just to make a runtime path pass. If the language should allow a construct, add a semantic rule and a regression test.

## Documentation

Update docs when you change:

- Language syntax or semantics.
- CLI commands or output.
- Diagnostics users may see.
- VBA compatibility behavior.
- Platform-specific behavior.
- Examples or getting-started workflows.

Useful starting points:

- [README.md](README.md)
- [Language docs](docs/language/README.md)
- [Architecture docs](docs/architecture/README.md)
- [Examples](examples/README.md)

## Current Priorities

Good contribution areas include:

- Real-world `.bas` and `.cls` compatibility cases.
- Parser and semantic regression tests.
- Diagnostics and suggestions.
- COM/OLE Automation behavior on Windows.
- Native FFI coverage.
- Module, namespace, and project-system behavior.
- Standard runtime functions.
- Documentation and examples.
- Future VM/compiler preparation.

If a change touches a core language rule, include both a positive test and, when appropriate, a rejection test.
