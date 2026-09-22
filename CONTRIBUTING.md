# Contributing to Valo

Valo is a native systems language with VB.NET-inspired syntax, implemented in Rust.
Preserve the implemented VB.NET surface. Remove legacy-only compatibility code,
and make native costs and unsupported features explicit.

Read the [audit and migration plan](docs/architecture/native-migration.md) before
changing language semantics. Official VB.NET documentation is the authority for
deciding whether existing syntax belongs to the protected baseline. Runtime design
and the native ABI are Valo's responsibility.

## Development

```sh
cargo build
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```

CI runs on Windows, Linux and macOS. Native FFI tests must declare actual C widths,
not assume Long means a 32-bit C int. Avoid platform assumptions in the frontend.

## Structure

- `core/src/frontend`: preprocessing, lexer/parser, AST, source types, semantics,
  symbol index, modules and manifests.
- `core/src/runtime`: interpreter values, operations, diagnostics and helpers.
- `core/src/backend/interpreter`: execution and libffi calls.
- `cli`: command-line tooling.
- `core/tests`: cross-stage regressions and example transcripts.

## Tests and changes

Provide a minimal .valo reproduction, expected behavior, diagnostics and relevant
platform details. Preserve existing tests for VB.NET functionality. Port overlapping
tests when removing legacy syntax; add negative tests for deliberately removed
features. Do not delete a whole test file because its name mentions VBA.

Example transcripts live in `examples/golden`. Use `VALO_BLESS=1` only after
reviewing an intentional output change. Keep planned features labelled as planned;
parser support alone does not establish execution, ownership or performance guarantees.

Use idiomatic Rust, explicit compiler stages, source-aware diagnostics and minimal
unsafe code. Core COM/OLE and source-compatibility modes are not part of Valo.
