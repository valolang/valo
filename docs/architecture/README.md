# Architecture Overview

> Native systems migration: see the [audit and status](native-migration.md) and the [first MIR milestone](mir.md). Existing VB.NET syntax is preserved; native compilation is planned.


Valo's architecture is divided into three primary layers to ensure modularity, portability, and future-readiness.

## 1. Frontend (`core/src/frontend/`)
The Frontend is responsible for transforming raw source code into a validated, semantically-rich Abstract Syntax Tree (AST). It is entirely independent of how the code is eventually executed.

*   **Preprocessor:** Handles conditional compilation (`#If`) and source transformations.
*   **Lexer:** Tokenizes source code into discrete elements.
*   **Parser:** Performs recursive descent to build the AST.
*   **AST:** The structural representation of Valo code.
*   **Semantics:** Validates symbols, types, and control flow.
*   **Module System:** Discovers and resolves module dependencies.

Learn more in **[Frontend Architecture](frontend.md)**.

## 2. Runtime (`core/src/runtime/`)
The Runtime defines the core data model and behavior of the Valo language. It is shared by both the Frontend (for type-checking) and all Backends (for execution).

*   **Value System:** Defines `Value`, `ObjectValue`, and type coercion rules.
*   **Diagnostics:** Provides source-aware error reporting.
*   **Operations:** Centralized logic for arithmetic, comparison, and coercion.
*   **Interpreter Resource Behavior:** Executes existing `Using` and explicit `Dispose`; native deterministic Drop remains unimplemented.

Learn more in **[Runtime Architecture](runtime.md)**.

## 3. MIR (`core/src/mir/`)

The first backend-neutral MIR lowers a verified subset of typed HIR into typed
locals, temporaries, basic blocks, and explicit control-flow/cleanup edges. It
has a verifier and deterministic debug output. It is not executable yet. See
the [MIR design and coverage](mir.md).

## 4. Backend (`core/src/backend/`)
The Backend is the execution engine that consumes the validated AST (or future intermediate representations) and performs the actual work.

*   **Interpreter:** The current reference execution engine (tree-walking).
*   **Future VM:** A planned bytecode virtual machine for higher performance.
*   **Future Backends:** Potential WASM and Native compilation targets.

Learn more in **[Backend Architecture](backend.md)**.

## 5. Platform
The Platform layer is the emerging project/package, namespace, tooling, standard-library, and interop architecture that turns Valo from a single-file language runtime into an ecosystem.

*   **Package Identity:** `valo.toml` project roots and entrypoint resolution.
*   **Semantic IDs/HIR:** Project declaration identities plus typed scalar function bodies with resolved locals, conversions, arithmetic signatures and selected calls. Broader lowering remains in progress.
*   **Namespaces:** Logical API identity decoupled from filenames.
*   **Runtime Services:** Standard-library boundaries shared by interpreter, VM, and embedders.
*   **Interop:** Native FFI, with platform libraries kept outside language semantics.

Learn more in **[Platform Architecture](platform.md)**.

## Cross-cutting: name folding

Basic is case-insensitive, so `Total`, `total`, and `TOTAL` are one name. Every
symbol table in the compiler and runtime is therefore keyed by a case-folded
name, and all three layers must agree on what folding means.

That rule lives in one place, [`core/src/runtime/naming.rs`](../../core/src/runtime/naming.rs).
The preprocessor, the semantic analyzer, and the interpreter all delegate to it
rather than each folding names their own way. `with_folded` is the form to reach
for on hot paths: it folds into a stack buffer instead of allocating, which
matters because identifier lookup is the single most frequent operation the
interpreter performs.

## Cross-cutting: validation

Both entry points (`validate` for a single program, `validate_project` for a
loaded project) run the same body validation over every procedure, function,
structure, and class. The project path additionally brings imported types,
callables, and extension methods into scope, and completes a `Partial Class` from
its halves in other modules.

Keeping the two paths on one implementation is deliberate. They diverged once,
with project validation stopping before procedure bodies, and the result was that
`valo check` reported success on programs that failed the moment they ran.

Learn more in **[Performance](performance.md)** and **[Diagnostics](diagnostics.md)**.

## Cross-cutting: the builtin registry

A builtin has a name, an arity, a result type, and an implementation. Those used
to live in three places, with nothing tying them together: name lists in
`runtime::builtins`, arity and result type as `if` branches in the analyzer, and
the implementation as more `if` branches in the interpreter. Adding a builtin
meant three edits in three files, and nothing detected it when they disagreed.

[`runtime::builtins`](../../core/src/runtime/builtins.rs) is now the single
declaration. Each row states the name, arity, result type, whether the builtin
may stand alone as a statement, and which dispatcher handles it:

```rust
f("Mid", 2, 3, BuiltinReturn::String),
grouped(f("DateAdd", 3, 3, BuiltinReturn::Date), BuiltinGroup::DateTime),
```

The analyzer looks the name up, checks the arity, and reports the result type.
The interpreter routes on the group. Neither keeps its own list.

A handful of builtins constrain an argument beyond counting it. `LBound`,
`UBound`, and `Filter` take an array; `IsMissing` takes an optional parameter;
`IsArray` accepts anything by design. Those are named explicitly in the
analyzer, so the exceptions are visible rather than buried in a chain of a
thousand lines.

Tests in the interpreter's builtin module hold the registry to its promise:
every declared builtin is reachable through the analyzer, and the arity a row
declares is the arity actually enforced. It is checked once, before dispatch,
so an implementation may index the arguments its own entry guarantees.

### Dispatching to an implementation

The interpreter used to find an implementation by walking a chain of name
comparisons, which meant the set of builtins a module implemented could only be
learned by reading its code. Modules are being converted to handler tables
instead, where the table *is* the dispatch:

```rust
pub(super) const HANDLERS: &[(&str, ValueFn)] = &[
    ("Round", round),
    ("Sgn", sgn),
    ("Pmt", financial),
];
```

There are two kinds of handler. Most builtins receive evaluated arguments. A few
need them unevaluated. `IIf`, `Choose`, and `Switch` must not evaluate the
branch they do not take, and `VarPtr`, `StrPtr`, and `ObjPtr` need the storage an
argument names rather than its value. Those are dispatched first, from their own
table.

Handlers receive the name they were reached by, so one implementation can back
several registered names. The financial functions use this, as do the numeric
conversions, which differ only in their target type and are generated by a small
macro rather than written out ten times.

Six tests keep the arrangement sound: a handler may only name a builtin the
registry declares; a table may not register a name twice; no two modules may
claim the same name; a builtin may not be both lazily and eagerly dispatched;
**every declared builtin has an implementation**; and no exemption from that last
check is stale.

That fifth test is the one the registry existed for. It was not checkable while
dispatch was a chain of name comparisons, because an implementation could only be
found by running it. Its exemption list, the record of what was still unchecked,
is now empty: every builtin the language declares is reachable through a table.

Some builtins are routed by group rather than by name: the file, date/time, and
file-system dispatchers each own a table, and dispatch picks the table from the
group recorded in the registry. A test asserts each group and its table name
exactly the same builtins, since a disagreement would send a builtin somewhere
with no handler for it.

## Cross-cutting: well-known names

The implementation depends on a set of identifiers of its own: the implicit
receiver `Me`, the lifecycle hooks, the pseudo-objects a call site can name
(`VBA`, `Err`, `Console`, `Debug`), and the built-in type names. Each appeared
as a bare string literal wherever it was needed, with `"me"` alone in more than
fifty places across the lexer, analyzer, and interpreter.

Written that way they were invisible to the compiler. A typo in one comparison
would simply stop matching, with nothing to catch it, and a rule spread across
files could be updated in one place and missed in another.

[`runtime::well_known`](../../core/src/runtime/well_known.rs) names them once.
Because names are case-insensitive, the ones used as symbol-table keys are
stored both as written and folded, with a test asserting the two agree.

It also owns the rules that were previously restated at each use:

- `find_constructor` and `find_destructor` try both spellings of a lifecycle
  hook. `Initialize` and `Class_Initialize` name the same thing, and every
  lookup has to know that.
- `return_slot` builds the frame key holding a function's implicit result. The
  analyzer records it and the interpreter reads it back, so the two must derive
  it identically.

The lexer's keyword table is the deliberate exception: a keyword is *defined*
there, and a literal reads better among the other keywords than a constant would.

## Cross-cutting: built-in types

`Collection` has no source declaration, so its signature has to be written in
Rust. It used to be 131 lines of nested struct literals (`HashMap::new()`,
`Vec::new()`, and field-by-field initialization) sitting inside the routine
that collects *user* types, which is not where a reader looks for it.

[`semantics::builtin_types`](../../core/src/frontend/semantics/builtin_types.rs)
holds it now, behind small builders that carry the defaults:

```rust
sig.subs.insert(key("Add"), sub("Add", vec![
    required("Item", TypeName::Variant),
    optional("Key", TypeName::String),
    optional("Before", TypeName::Variant),
    optional("After", TypeName::Variant),
]));
```

Each type reads as its shape rather than as the boilerplate around it, and
adding another built-in type is a handful of lines instead of a transcription
exercise.
