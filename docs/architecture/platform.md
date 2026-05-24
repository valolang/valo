# Platform Architecture

Valo's long-term platform direction is "Modern Basic outside Office": a standalone runtime, package ecosystem, tooling surface, and interoperability layer for Basic-family code.

## Package Identity

`valo.toml` is the project identity root. The initial manifest is intentionally small:

```toml
[package]
name = "sample"
version = "0.1.0"
entrypoint = "src/main.valo"
authors = ["Valo Developer"]
compatibility = "mixed"
target_platforms = ["windows", "linux", "macos"]

[dependencies]
```

The CLI can resolve a directory or `valo.toml` to its configured entrypoint. Registry resolution and lockfiles are deferred until local package identity, project validation, and public API indexing are stable.

## Semantic IDs and HIR

Project analysis now has stable identity wrappers:

- `ModuleId`
- `TypeId`
- `FunctionId`
- `MemberId`
- `SymbolId`

The first HIR layer is a project index, not bytecode. It records module ownership, namespaces, visibility, and qualified symbol names so future tooling does not need to rediscover identity from string maps in the interpreter.

## Namespaces

`Namespace ... End Namespace` is the syntax foundation for decoupling logical identity from filenames. Native Valo packages should use namespaces for public APIs. VBA compatibility imports can continue using file/module names to preserve migration behavior.

## Runtime Services

Standard library domains should move behind runtime services instead of living permanently inside interpreter dispatch. The initial service boundary covers console, environment, process, and native collection foundations. Future stdlib modules can grow into package-backed runtime libraries without changing the interpreter API every time.

## Callable Values

Delegates, lambdas, event handlers, native callbacks, async continuations, and collection callbacks all need a shared runtime shape. Valo now has callable metadata types that are independent of AST nodes; syntax and execution lowering are deferred until the semantic model can resolve callable references cleanly.

## COM Interop

COM is Windows-native. Valo should not fake cross-platform COM. The planned approach is:

1. Late-bound `ComObject`/`IDispatch` support.
2. `Import COM` syntax and type-library mapping into namespaces.
3. Generated wrappers for constants, enums, interfaces, and coclasses.
4. Explicit ownership for `BSTR`, `SAFEARRAY`, `VARIANT`, and `IUnknown`.

The current codebase contains architecture types for COM import and type-library metadata only; it does not claim COM runtime execution yet.
