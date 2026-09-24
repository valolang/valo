# Native systems migration

Valo is a native systems language with VB.NET-inspired syntax, implemented in Rust.
The native execution model is the direction of development; the current executable
backend is still an interpreter. This audit records that distinction explicitly.

## Audit and baseline

The pre-migration Windows workspace suite passed: 653 core unit tests, the example
integration test, and 15 runtime integration tests. There were no CLI unit tests or
Rust doctests. Existing uncommitted interpreter/FFI and raycaster changes were
present before this migration and have been preserved.

| Area | Existing implementation | Migration implications |
| --- | --- | --- |
| Workspace | `core` and `cli`, Rust 2024 | Preserve Rust and the workspace |
| Preprocessor | Conditional compilation, line continuation, host target constants | Retain source ergonomics; target information must eventually come from compilation options rather than the host |
| Lexer | Handwritten scanner with spans, VB keywords and type characters | Retain case-insensitive names and original source spelling; remove legacy-only tokens incrementally |
| Parser | Recursive descent, separate declaration/expression/statement modules | Extensive VB.NET support is an asset, not a rewrite target |
| AST | Untyped, owned Rust data structures; modern and legacy declarations mixed together | Keep spans and syntax separate from execution and ownership |
| Names/types | Registries, overload resolution, generic constraints, imports and class validation | Type descriptions now live in `frontend/type_model.rs`; remaining runtime dependencies need separation |
| HIR | Project declaration index plus `typed_hir.rs` scalar function bodies | Typed lowering now retains locals, conversions, arithmetic signatures and selected calls; broader bodies and control flow remain planned |
| Interpreter | Tree walking, frames, `Rc`/`RefCell`, copy-on-write records/arrays, closures, exceptions and native calls | Preserve for tests/REPL; these representations are not a native ABI or ownership specification |
| Values | Tagged Rust `Value`, dynamic values and implicit coercions | Dynamic defaults, null/empty semantics and coercions still require redesign |
| Native calls | libffi, target library loading, callbacks and record packing | Preserve C interoperability; concrete ABI types must replace automation marshalling |
| Code generation | No AOT/JIT/object-code backend | `valo build` must not pretend to generate native code |
| CLI | `run`, `check`, `repl`, `version`, `help` | Keep these working; other commands remain planned |
| Packages | Small TOML-subset reader for `valo.toml`, entrypoint and dependency strings | No resolver, registry, lockfile, package downloader or reproducible build graph yet |
| Library | Intrinsic registry, interpreter builtins, native collections, file I/O | Many Basic runtime functions overlap VB.NET; classify individually before removal |
| Tests | Parser, semantic and interpreter tests plus recursive example transcripts | Preserve modern tests; port overlap tests and replace removed-feature positives with negatives |
| Examples | Modern language examples, native FFI, games, old export/automation samples | Remove export/automation samples; pin native layouts to explicit widths |
| Documentation | Detailed language pages mixed with obsolete compatibility promises | README and this migration/status document are authoritative during transition |
| Platform assumptions | COM/OLE, ANSI fallback, pointer opt-in flags, host-preprocessor constants, Windows dialogs | COM/OLE, ANSI fallback and pointer opt-in removed; optional dialog bindings remain platform-specific runtime code |

## Protected VB.NET regression surface

Existing tests are retained for classes, inheritance and dispatch, access control,
partial/shared members, constructors, properties, events/delegates and handlers,
structures and value copying, interfaces, enums, modules/namespaces/imports,
generics/constraints/variance syntax, overloads, extension methods, operators,
optional/named/ByRef parameters, lambdas/closures, tuples/anonymous objects,
initializers/query expressions, nullable values, casts, type tests, `NameOf`,
interpolation, compound assignments, shifts, loops, iterators, exceptions and
`Using`. `tests/type_system.rs`, `classes.rs`, `calls.rs`, `expressions.rs`,
`iterators.rs`, `modernization.rs`, `lifecycle.rs`, `modules.rs` and the parser tests
provide the existing coverage; `core/tests/native_foundation.rs` adds cross-stage
native-direction regressions. A passing test for a subset is not a claim of full
VB.NET conformance.

`Async`/`Await` currently execute synchronously. Generic variance and some modifiers
have parser/validation support without a full native implementation. `Option Infer`
does not yet have its own implemented directive; ordinary local inference exists.
`GetType` currently produces a runtime description, not a native metadata system.
Bare array literals (`{1, 2, 3}`) and implicit For Each variable declarations are
not implemented; the introductory example uses Array(...) and declares its loop
variable explicitly.

Authority for overlap decisions:

- [VB.NET language reference](https://learn.microsoft.com/en-us/dotnet/visual-basic/language-reference/)
- [Primitive widths](https://learn.microsoft.com/en-us/dotnet/visual-basic/language-reference/data-types/index)
- [ByVal default](https://learn.microsoft.com/en-us/dotnet/visual-basic/language-reference/modifiers/byval)
- [Native Declare](https://learn.microsoft.com/en-us/dotnet/visual-basic/language-reference/statements/declare-statement)
- [On Error](https://learn.microsoft.com/en-us/dotnet/visual-basic/language-reference/statements/on-error-statement)
- [Source options and namespaces](https://learn.microsoft.com/en-us/dotnet/visual-basic/reference/language-specification/source-files-and-namespaces)

The final VB.NET-preservation instruction takes precedence over the earlier list
of candidate removals. In particular, `On Error`, `Err`, `Resume`, `Call`, `ReDim`,
type characters, conversion functions and the implemented VB.NET options are not
removed merely because they also existed in VBA. Structured exceptions remain the
preferred error model; a native `Result` type is planned. Core COM intrinsics are
an explicit exception required by the new Valo direction.

## Changes implemented in this migration

- Corrected omitted parameter modifiers to ByVal, as in VB.NET; explicit ByRef remains supported.
- Removed predefined VBA6/VBA7 conditional-compilation flags.
- Removed compatibility manifest modes and their public enum/field. Old settings
  now diagnose the required migration instead of being ignored.
- Removed `.bas`/`.cls` loading, implicit sibling project discovery and ANSI fallback.
  Imports remain explicit; UTF-8 and BOM-marked UTF-16 are supported.
- Removed core COM values, activation, property/method/default dispatch,
  enumeration, OLE conversion, and libffi `VARIANT` storage. No COM/OLE dependency
  features remain. Dynamic ABI values fail clearly.
- Removed `CreateObject`, `GetObject` and `DoEvents` intrinsics. These identifiers
  may still name user-defined functions.
- Removed `Set` assignment end to end, including its AST node. Normal assignment
  and modern property setter syntax remain supported.
- Removed `PtrSafe` syntax, AST metadata and runtime gate. Native `Declare` works
  without a VBA architecture switch. `LongPtr` no longer resolves as a primitive;
  existing native examples use the experimental `Ptr` type.
- Moved language type descriptions out of runtime storage and into the frontend.
  Fixed the `Integer`/`Long` aliases, conversion functions, identifier type characters
  and integer inference. Added explicit supported native aliases and width queries.
- Ported native game/sample declarations to explicit storage widths, preserving
  existing layouts and user edits. Removed automation/export-only examples.

## Remaining obsolete implementation inventory

This is an **initial implementation**, not a claim that Phase 1 is finished. The
following active code still needs migration; none is a commitment to legacy source
compatibility or a dormant compatibility mode:

| Remaining legacy concern | Location and required work |
| --- | --- |
| Export metadata/envelopes | Removed: no export envelope parser, AttributeDecl, or metadata-selected default/enumerator members |
| Legacy record syntax | `TypeKind::Type`, `Type ... End Type`; port record/FFI fixtures to `Structure`, remove legacy AST distinctions |
| Legacy properties | Removed: standalone Get/Let/Set forms rejected, setters unified; modern blocks and interface declarations preserved |
| Dynamic defaults | `TypeName::Variant`, parser defaults, semantic inference, `Value::Empty/Null`; separate Object/dynamic values from inference holes before enforcing static defaults |
| Lifecycle hooks | `well_known` constructor/destructor aliases and class tests; remove `Class_Initialize`/`Class_Terminate` magic while preserving `Sub New` and resource cleanup |
| Array lower bounds | `Option Base` removed; omitted bounds are zero. Explicit nonzero bounds and runtime array indexing remain for further review; preserve VB.NET `ReDim Preserve` behavior |
| Legacy I/O syntax | File-number statements and `file_io.rs`; replace statement-level compatibility with an explicit IO library |
| Runtime helpers/constants | `runtime/vba.rs`, intrinsic catalog and builtin tests; retain VB.NET overlaps, remove VBA-only helpers and namespace qualification |
| Pointer escape intrinsics | `VarPtr`, `StrPtr`, `ObjPtr`, `Any`, untyped `Ptr`, callback lifetime state | Replace with typed pointers and checked unsafe contexts; do not claim memory safety today |
| Numeric debt | Legacy hex/octal sign rules, numeric coercions, Boolean native ABI, overflow and Decimal/Currency behavior | Specify and test each boundary; fixed aliases alone do not finish this work |
| Host facilities | Windows MsgBox binding, shell/settings stubs, host-preprocessor constants | Move OS facilities into explicit platform libraries; remove silent emulation |

The tests still named `vba_compat.rs`/`vba_parity.rs` contain both this remaining debt
and protected VB.NET/library behavior. They must be split by behavior, not deleted
wholesale. Existing legacy examples that remain exercise that debt, not supported
macro migration.

## Migration architecture and next acceptance gates

1. **Finish frontend separation and legacy removal.** Port the inventory above,
   preserving the protected regression suite. Separate unresolved/inferred types
   from explicit dynamic/Object values. Every removed construct gets a negative
   diagnostic test; no runtime fallback should invent behavior.
2. **Resolved, typed HIR.** Reuse stable IDs; add typed expressions, calls with
   selected overload IDs, places (locals/fields/elements), explicit conversions,
   declaration identities and source spans. Validation must return this artifact,
   rather than the interpreter repeating resolution. Retain AST only for source
   tooling and diagnostics.
3. **Ownership analysis.** Distinguish values, owned storage, immutable/mutable
   borrows and raw pointers. `ByRef` describes a mutable borrow; `ByRef ReadOnly`
   describes an immutable borrow. Record ownership transfer and scope exits in HIR.
   Diagnose conflicting aliases, use after move and escaping references before
   lowering. Existing interpreter ByRef is not a borrow checker.
4. **Typed control-flow IR.** Basic blocks, explicit branches/calls, typed operands,
   moves/copies, cleanup edges and drops. Define cleanup on return, throw and loop
   exits, including reverse declaration order. Preserve structure value semantics;
   interpreter copy-on-write is not the native representation.
5. **Target layout and unsafe checking.** Add Int8/UInt16/ISize/USize/Char and typed
   `Pointer(Of T)` with target-provided widths/alignment. Never substitute host
   `usize` during cross compilation. Unsafe operations carry effects; reject them
   outside explicit unsafe contexts. Define String encoding/views, nullability,
   overflow, and C ABI layout before code generation.
6. **Backend contract.** Consume verified IR plus an explicit target and layout;
   return artifacts or unsupported-capability diagnostics. Choose LLVM/Cranelift
   after lowering requirements are clear. Keep interpreter, AOT and possible JIT
   consumers separate. No backend marker type should claim to emit native code.
7. **Systems extensions.** Monomorphization, explicit allocation, Result/Option,
   SIMD, parallelism and async state machines. GPU lowering will need address
   spaces, device-safe calls and effect restrictions; keep these representable
   without baking CPU pointers or host runtime objects into HIR. Compile-time
   execution should consume typed code under an explicit capability policy.

Native binaries, borrow checking, deterministic owner destruction, zero-cost
abstractions, SIMD, GPU execution and compile-time evaluation remain planned.
`Using` and structure copy behavior work today, but are not proof of those guarantees.

## Verification of this migration

Verified on Windows with the Rust workspace toolchain:

- `cargo test --workspace --quiet`: 641 core tests, 8 native-foundation tests,
  15 runtime integration tests, and the example suite running 113 examples pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: passes.
- `cargo fmt --all -- --check` and `git diff --check`: pass.
- `cargo build --release --workspace`: produces the Rust toolchain successfully.
- The rebuilt release CLI validates both `game/raycaster/main.valo` and
  `game/breakout/main.valo`. Interactive gameplay was not exercised.

Linux and macOS execution were not tested in this workspace; the existing CI
matrix retains those platforms. Removed automation/export tests were replaced or
ported as appropriate; remaining VB.NET and runtime coverage stays enabled.


## Stage 2 progress: declaration inference and semantic ownership

Implemented in the first Stage 2 increment:

- The parser preserves an omitted variable type as `None`, rather than inserting
  `Variant`. Local, static and module declarations now reach semantic inference.
  Missing both a type and an initializer produces a source-located diagnostic
  with explicit-type and initializer suggestions, including multi-declarations.
- Shared `As` clauses follow VB.NET grouping: `Dim A, B As Integer` declares
  both variables as Integer. An equals initializer on a shared group is rejected;
  initialize each variable separately. This intentionally supersedes the former
  VBA per-declarator test. Type-character coverage remains, without an implicitly
  dynamic variable in its fixture.
- Tuple records retain their resolved structural type instead of reconstructing
  a user-defined type from their display text. This preserves inferred tuples,
  anonymous records and tuple copies. Existing closure regression tests remain.
- Overload ranking is owned by `frontend/semantics/overloads.rs`. Semantic analysis
  and the transitional interpreter share it; no runtime overload module remains.

This is partial static-typing work, not completion of Stage 2. Explicit Object
and Variant, omitted callable parameter/return types, inferred closure signatures,
field inference, undeclared-variable handling, and dynamic builtin return types
still need separation. Primitive binary arithmetic now uses the shared frontend
rule described below; other operators and dynamic coercions still need review.

The declaration-inference increment did not yet introduce typed bodies. The later
arithmetic/HIR increment below does so for scalar functions. Ownership analysis,
MIR and native backend interfaces remain planned.

The shared declaration rule follows the official
[VB.NET Dim reference](https://learn.microsoft.com/en-us/dotnet/visual-basic/language-reference/statements/dim-statement).

Validation of this increment on Windows: all 670 workspace tests pass, including
five new semantic regression tests and the suite executing 113 examples. Clippy
with warnings denied, formatting, whitespace checks, and the release workspace
build pass. The rebuilt release CLI passes static checks for raycaster and
breakout; interactive gameplay was not tested.


## Stage 2 progress: remove module-wide array and visibility options

Implemented:

- Removed `Option Base` parsing, its AST field, interpreter state and declaration
  plumbing. Omitted array bounds and ParamArray arrays start at zero. `ReDim` and
  `ReDim Preserve` remain supported. Existing explicit bounds are still accepted;
  their eventual systems-language design remains separate work.
- Removed `Option Private Module`, its unused AST metadata and its parser helper.
  Public/Private declarations and explicit imports remain the visibility model.
- Both removed directives produce migration diagnostics. The old Option Base
  example is now `zero_based_arrays.valo`, with an updated expected transcript.
  Module import regression fixtures retain their substantive behavior.

The module-option increment only audited properties. The subsequent work below
removes legacy property/exported metadata paths and introduces typed bodies;
ownership analysis and MIR remain planned.


Reference: the [VB.NET Dim specification](https://learn.microsoft.com/en-us/dotnet/visual-basic/language-reference/statements/dim-statement)
defines zero-based declarations; [Option Private](https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/option-private-statement)
is an Office/VBA host-project visibility facility.

Validation of the module-option removal on Windows: 672 workspace tests and all
113 runnable examples pass. Formatting, Clippy with warnings denied, whitespace
checks and release workspace build pass. Both raycaster and breakout pass static
checks using the rebuilt release CLI. Interactive gameplay was not tested.


## Stage 2: property migration, arithmetic semantics and typed bodies

### Implemented

- Removed the exported class envelope parser, `AttributeDecl`, exported module/
  class metadata fields and metadata-based default/enumerator dispatch. Modern
  angle-bracket attributes remain intact. Old positive fixtures use `Default`
  and `Iterator`/`Yield`; exported metadata has negative tests.
- Removed standalone `Property Get`, `Property Let` and `Property Set` syntax,
  including interface variants. The AST, semantic registry and interpreter now
  have a getter and one setter kind/table, with no object-only setter path.
  Modern block/auto properties, indexed overloads, ReadOnly/WriteOnly, default
  properties and structure properties are retained. Modern interface properties
  and explicit property `Implements` clauses are covered by tests. Named-module
  auto properties retain shared backing storage and work through module-qualified
  access. Bare unqualified module-property lookup is not expanded by this work.
- `semantics/arithmetic.rs` owns signatures for addition, subtraction,
  multiplication, division, integer division, remainder and power. Validation,
  typed HIR, execution and interpreter arithmetic argument hints use this rule.
  Unsuffixed integer literal selection is also shared by semantic/generic/
  overload analysis, preventing the old Int16 inference from reappearing.
- `semantics::lower_function_body(&Program, function_index)` returns a typed
  body after source validation. It records body-local identities, parameter
  indices, canonical types, explicit local places/loads/stores, conversions,
  arithmetic signatures, selected function IDs and ByRef borrow expressions.
  Output is deterministic and contains no runtime `Value` objects.

### Intentional arithmetic changes

Small integer operands (Byte/Short/Integer) compute as Int32; Long promotes
these to Int64. UInt32 with signed Int32/Int16 promotes to Int64. For integer-result operations, UInt64 with a
signed integer requires an explicit conversion. UInt32/UInt64 combinations retain
the wider unsigned type. Single arithmetic retains Float32 unless a Double operand is
present. `/` and `^` return Double. Integer division on floating operands truncates
them to Int64 first. `Mod` supports floating remainders. Integer add/subtract/
multiply wrap at the selected width; minimum signed integer divided by -1 also
wraps, avoiding a Rust panic. Division/remainder by zero remain diagnostics.
This is a defined transitional overflow policy, not configurable overflow modes.
Decimal/Currency/Date retain their interpreter floating-arithmetic behavior;
native layout and exact arithmetic for those types are not claimed.

### Partially implemented: typed HIR

The lowering subset is non-generic scalar Functions: parameters, top-level scalar
Dim declarations/default initialization, local assignments, primitive arithmetic
and comparisons, `If`/`ElseIf`/`Else` branches, Return and calls to declared
functions. Each accepted path must return; unreachable statements are rejected.
Implicit numeric conversions and ByRef local arguments are explicit nodes.
Calls reuse semantic overload selection and store the selected function identity.
`BodyFunctionId` indexes the input Program's function list; LocalId indexes the
body. These are intentionally distinct from project declaration-index IDs.

Unsupported lowering returns a source-located diagnostic. It never leaves opaque
AST nodes in a supposedly typed body. At this earlier increment, branch-local
declarations and loops were not yet lowered; the scoped-control-flow increment
below adds a supported subset. Fields/properties, closures,
generic specialization, async/iterators, named/optional/ParamArray arguments,
reference conversions and non-scalar types still execute through the existing
source interpreter; their frontend support has not been removed. HIR currently
has no CLI emission mode and does not replace interpreter execution.

### Groundwork and next priorities

ByVal value storage, ByRef mutable-reference storage, place/value categories and
borrow nodes are represented. An initial separate HIR ownership pass rejects two
simultaneous mutable `ByRef` arguments that borrow the same local in one call.
This does not provide general borrow checking: aliases through fields or other
references, lifetime escapes, moves and cleanup/drop analysis are not yet implemented.
No MIR or native backend has been added. Next work should extend typed body
coverage and make resolved call/type information reusable by execution, then add
control-flow and ownership analysis before defining the MIR/backend contract.
Further broad compatibility cleanup is no longer the main Stage 2 activity.


Validation of the property/arithmetic/HIR increment on Windows: all **684 tests**
pass, including the suite running **113 examples**. Formatting, Clippy with warnings
denied, whitespace checks and the release workspace build pass. Both raycaster
and breakout pass static checks using the final rebuilt release CLI. Interactive
gameplay and other operating systems were not tested in this workspace.

The next HIR increment added typed `If`/`ElseIf`/`Else`, primitive comparisons,
return-path checking and a call-scoped mutable `ByRef` alias check. Its Windows
validation passes **689 workspace tests**, including **113 examples**, plus
formatting, Clippy with warnings denied, whitespace checks, release builds and
static raycaster/breakout checks. The interpreter does not consume these HIR bodies
yet, and the ownership check is limited to direct local arguments.

## Stage 2: scoped control flow and the MIR boundary

The current HIR lowering now assigns a stable `ScopeId` to the function body,
every `If`/`ElseIf`/`Else` body, and each loop body. Each scope records its parent,
span and locals in declaration order. Local resolution is lexical within HIR;
branch-local names are removed from the active binding table at scope exit.
Source validation still rejects same-name shadowing in nested blocks, so that
case is not yet a supported source program. Scope records retain enough data to
visit owned locals in reverse declaration order during future cleanup insertion.

`While`, all current pre/post `Do` conditions, counted `For`, and one-dimensional
zero-based fixed-array `For Each` have typed HIR forms. `For` and `For Each`
require an existing loop variable, matching the current parser. Bounds, step and
iterable expressions are represented separately and evaluated at loop entry;
`While` and `Do` conditions retain their test position. `Exit` and `Continue`
carry a resolved `LoopId` and the exact nested scopes they leave. `Return` records
all scopes it leaves, inner first. Nested blocks can declare locals. Array HIR
currently only constructs a fixed, one-dimensional, zero-based array of scalar
elements; multidimensional/dynamic arrays and general enumerable objects remain
source-interpreter features.

The HIR distinguishes `Initialize` from `Store`. Parameters start initialized;
locals start uninitialized and become initialized at their declaration statement.
The ownership pass checks use before initialization, direct-local `ByRef` alias
conflicts, mutation through an immutable borrow, and use after an internally
represented move. It merges states across branches and conservatively across
loops. The later increment below exposes a narrow `ByRef ReadOnly` source subset;
`Move` remains internal. This is groundwork, not general lifetime safety. Alias paths through fields, arrays,
closures, indirect references, calls and loops need stronger analysis. No resource
destructor or actual drop is emitted yet.

`verify_hir::verify_body` checks scope parents, local membership, and the unwind
lists on returns and loop control before ownership analysis. The proposed MIR
lowering boundary accepts only a verified, ownership-checked `TypedBody`; it must
not consult the AST or interpreter. The next MIR representation should have typed
local slots, SSA-like temporaries or explicit virtual registers, basic blocks,
primitive operations, conversions, calls, loads/stores and explicit terminators
for branches, jumps and returns. Each normal scope edge and non-local exit must
insert cleanup for initialized owned locals in reverse declaration order; borrowed
parameters are not owners. Return values must be evaluated before their cleanup
edges. Counted-loop bounds and steps must be evaluated once before the header;
`Do` post-tests run after the body scope exits. A future `For Each` lowering needs
an explicit iterator/array-view ownership decision before it can promise native
semantics. MIR design should carry source spans for diagnostics and avoid backend
types. There is no MIR lowering or native code generation implementation yet.

Validation of this scoped-control-flow increment on Windows: **698 workspace
tests** pass, including the suite executing **113 examples**. Formatting,
Clippy with warnings denied, whitespace checks, and the release workspace build
pass. The rebuilt release CLI validates both raycaster and breakout entry points.
Interactive gameplay and other operating systems were not tested here.

## Stage 2: resolved call modes and narrow immutable borrowing

Implemented in this increment:

- Ordinary untyped parameters and Functions with omitted return types are
  rejected with guidance to add `As`. Untyped lambda parameters remain accepted
  when their types can be inferred from context. Explicit `Object` and `Variant`
  remain available; this change does not erase existing dynamic runtime values.
- Source `ByRef ReadOnly` is parsed and checked for scalar numeric, Boolean and
  String parameters. It permits simultaneous direct-local immutable borrows and
  rejects overlapping direct-local mutable/immutable borrows in the typed HIR
  subset. Direct assignment to such a parameter is rejected during validation;
  the interpreter also guards assignments through its read-only alias binding.
  Structure, Class and array parameters are rejected for now because field,
  element and escaping-reference analysis is not sound yet. Native `Declare`
  calls with this mode fail explicitly. This is partial source support, not a
  general borrow checker or lifetime guarantee.
- Resolved HIR calls store canonical parameter types, parameter passing modes,
  return type and each argument's value or borrow intent. The HIR verifier checks
  these against the call expression and checks scalar conversions, arithmetic,
  comparisons, conditions, stores and returns. A deterministic HIR debug printer
  exposes scope IDs, locals, call modes and control flow for tests.

Still partial or internal-only:

- `Move` and use-after-move analysis are internal HIR capabilities. No public
  `Move(Value)` syntax is accepted. Copy-versus-move classification for native
  resources and reference escape analysis have not been designed sufficiently
  to expose it safely.
- Typed bodies still cover a scalar/function subset. The source interpreter
  continues to execute fields, properties, non-scalar values, closures,
  iterators, general arrays and many calls. It also owns current `Using` and
  `Try`/`Catch`/`Finally` execution. Neither construct has a typed HIR form yet.
- Scope identity, declaration order and exited-scope lists permit future cleanup
  planning. There is no reliable `requires_drop` classification, user-visible
  destructor contract, exception cleanup edge or emitted `Drop`. No native
  deterministic destruction is claimed.
- `For Each` typed HIR is still limited to one-dimensional, zero-based fixed
  arrays of scalar elements. General iterator protocol resolution is pending.
- No MIR data model, lowering pass, verifier or backend exists. The HIR-to-MIR
  boundary remains a design contract: HIR resolves source semantics and lexical
  lifetime intent; MIR will own basic blocks, explicit CFG, storage operations,
  cleanup edges and drops. MIR lowering must never ask the AST or interpreter to
  decide types, overloads, passing modes or scope exits.

Stage 2 is **not complete**. Before MIR, typed bodies need general semantic
places, `Using` and `Try`/`Finally` cleanup structure, explicit drop requirements,
sound source-level ownership transfer, broader borrow/lifetime analysis and a
resolved general iteration protocol. Class allocation and lifetime semantics
remain interpreter-oriented and unresolved for native execution. Numeric
arithmetic signatures are centralized for the covered primitive operators;
dynamic coercions, Decimal/Currency/Date, overflow modes and full conversion
semantics are not yet centralized in the native type model.

Validation of this increment on Windows: **703 workspace tests** pass, including
the suite executing **113 examples**. Formatting, Clippy with warnings denied,
the release workspace build and whitespace checks pass. The rebuilt release CLI
validates both raycaster and breakout entry points. Interactive gameplay and
other operating systems were not tested here.

## Stage 2: semantic place and cleanup-order groundwork

The HIR now represents an addressable place separately from an arbitrary value.
Its root is a stable local ID; its projection vocabulary includes fields, tuple
fields and indices. Place overlap is conservative: distinct roots and distinct
known fields or constant indices are disjoint, whole-object/field paths overlap,
and unresolved index relationships are unknown and therefore conflict for a
mutable borrow. The call-scoped ownership check uses this comparison. The
frontend currently **lowers only local places**. Projected places cannot pass
HIR verification until member/index resolution supplies real field identities,
types and evaluated index operands. The debug printer displays projections for
inspection. This is infrastructure, not source-level field/index borrowing.

`TypedBody::owned_exit_locals` derives candidate owned locals from verified
exit-scope chains, inner scope first and reverse declaration order. This works
for Return, Exit and Continue without revisiting the AST. It intentionally does
not call these candidates drops: type-level Copy/drop properties, moved state on
each edge, resource initialization and exceptional exits are unresolved. Return
expressions must later be evaluated into temporaries before this cleanup order
is emitted by MIR. Reassignment of a drop-requiring place must evaluate the new
value before destroying the old one; self-move requires an explicit rule.

Public `Move(Value)` remains deferred. The current typed-body subset has no
specified non-Copy native resource type or deterministic destructor behavior,
so exposing `Move` through the interpreter would create an unsound promise.
Internal HIR moves still support dataflow tests. `ByRef ReadOnly` remains limited
to scalars; temporary call borrows end with the call, but stored/escaping
references are not analyzed. Copy-versus-move classification, recursive
Structure drop queries, partial moves, projected mutation, resource-aware
`Using`, `Try`/`Finally` HIR and exception cleanup edges are blockers for MIR.

Runtime coercion audit: `runtime/coerce.rs` still performs checked numeric
rounding/range conversion, structural tuple element conversion, nullable
normalization of `Nothing`/`Null`/`Empty`, explicit `Variant` passthrough and
interpreter object/delegate acceptance. Primitive arithmetic in the typed HIR
has frontend signatures and explicit conversions; the other runtime paths have
not yet been made native semantic rules. Explicit `Variant` uses a tagged
interpreter `Value` with dynamic conversion and dispatch cost; native `Variant`
layout and compilation are not defined. `Object` remains an explicitly named
runtime reference abstraction. Neither is an implicit declaration default.

For future native execution, `Using` cleanup is an obligation attached to an
owned resource scope; a user `Dispose` call and a compiler `Drop` are distinct.
The required normal/return ordering is: evaluate the expression, run nested
`Finally` bodies, clean inner resources, clean outer resources, then transfer
the saved return value. Exception paths need the same scope ordering once the
exception ABI is defined. This is a documented contract, **not** a claim that
current HIR or MIR implements these edges. A future `For Each` protocol must
resolve iterator acquisition, advancement, current value and cleanup in HIR;
its present array-only form cannot define general resource iteration.

Validation of the place/cleanup-order increment on Windows: **706 workspace
tests** pass, including the suite running **113 examples**. Formatting, Clippy
with warnings denied, release workspace build and whitespace checks pass. The
rebuilt release CLI validates both game entry points. No MIR verifier exists yet.

## Stage 2: resolved projected places and path-sensitive cleanup reports

The scalar typed-body subset now lowers real source Places for locals, fields of
declared Structures (including nested fields), named/positional tuple elements,
and one-dimensional array elements. Field IDs are stable within the body and
the HIR carries a resolved field table with owner and result types. Index
projections retain their typed index expression and element type. HIR verification
walks each projection, checks field IDs/owner types, tuple bounds, array base and
integral index types, and checks the final Place type. Property setters, Class
fields, multidimensional arrays and general indexers are not projected Places
yet. Source access validation still runs before lowering; MIR will use the HIR
field table rather than revisit declarations.

Call-scoped mutable/immutable borrow checks now compare these Places. Distinct
Structure fields and distinct constant array indices can be proved disjoint;
whole object versus field overlaps. Dynamic indices remain uncertain and are
rejected when either borrow is mutable. Two read-only borrows of the same array
element are accepted. `ByRef ReadOnly` is still source-limited to scalar
parameter types; read-only Structure parameters and stored reference escapes
remain unsupported. The borrow analysis does not yet model aliases through
heap references, class members or closures.

`type_properties.rs` provides separate conservative Copy and `requires_drop`
queries. Primitive values and enums are known Copy/no-drop. A Structure is
classified recursively from its declared fields; tuples and nullable values
likewise use their elements. Cycles, Classes, Strings, arrays, generics and
types without a native ownership contract are `Unknown`, never silently
treated as Copy or drop-free. The current source-level typed HIR has no defined
non-Copy resource type with a confirmed destructor, so a `Yes` drop requirement
is exercised only by internal HIR tests. This is classification groundwork,
not an implemented resource-destructor system.

Every HIR local records its type properties. The separate ownership pass now
returns a deterministic report for normal branch/loop exits, Return, Exit and
Continue. It lists candidate locals in reverse declaration order and records
`Drop`, `NoDrop`, `SkipMoved`, `SkipUninitialized` or `Unresolved` after evaluating
the exit expression. Reassignment reports whether the old whole-local value
would need cleanup **after** the RHS succeeds. A moved whole local can be
reinitialized; internal Move of a Copy value leaves it available, while internal
Move of a known non-Copy value invalidates it. Self-move of a known non-Copy
place is rejected. Field/index replacement remains `Unresolved`, and partial
moves are rejected rather than treated as whole-local moves. The report is
inspectable with the deterministic debug printer. These are semantic plans,
not executed drops or MIR instructions.

At this increment, `Using` and `Try`/`Catch`/`Finally` still executed through the
interpreter and were not typed-HIR constructs. Consequently normal/exceptional resource cleanup,
Finally ordering, moved resources inside Using and native destruction remain
unproved. Public `Move(Value)` is still gated on a real non-Copy/drop contract,
Using cleanup and sound replacement rules. Stage 2 remains **incomplete**; MIR
has not begun. Before MIR, the frontend needs resource-aware Using and Finally
regions, class/resource ownership decisions, field/index overwrite state,
reference-escape analysis, exception cleanup edges and a verifier for their
resulting obligations. Native code generation remains out of scope.

Validation of this increment on Windows: **722 workspace tests** pass, including
the suite executing **113 examples**. Formatting, Clippy with warnings denied,
release workspace build and whitespace checks pass. The rebuilt release CLI
validates both game entry points. No MIR tests exist because MIR has not begun.

## Stage 2: typed normal-flow Try/Finally, resource gate remains closed

Typed HIR now has a structured `TryFinally` node with distinct lexical scopes
and fully lowered bodies for the existing scalar-function subset. The verifier
checks both scopes and rejects non-local exits within them; ownership analysis
records normal Try-scope cleanup before analyzing Finally, then records normal
Finally-scope cleanup. The debug printer shows both regions in source order.
This is **normal-flow support only**. Catch clauses, Return/Exit/Continue inside
Try or Finally, exception cleanup edges and native unwinding still produce an
explicit unsupported-HIR diagnostic. The existing interpreter continues to run
those source programs; no VB.NET syntax was removed. The eventual unwind plan
must evaluate a return value, clean inner scopes, run Finally, then clean outer
scopes without consulting the AST. The current report does not yet encode that
interleaving, so MIR remains gated.

An audit of the interpreter's `Value` representation found pervasive `Clone`
and `Rc` sharing of objects, arrays, records and strings. Marking one of those
existing values as uniquely owned would not establish deterministic native
destruction or prevent duplication. There is therefore **no source-level native
OwnedBuffer/resource type in this increment**. A real allocation type must have
a unique-owner representation and statically enforced Copy/ByVal behavior
before `requires_drop = Yes` is assigned to source code. The internal tests
still exercise known-non-Copy/drop states by constructing typed HIR directly;
they are compiler-infrastructure proofs, not executable Valo resource examples.
At this increment, `Using` remained interpreter-owned and tied to a Class `Dispose` method, which
must not be conflated with native Drop. Public `Move` remains gated because a
source resource could otherwise be copied or dropped twice. Stage 2 is still
incomplete and MIR has not started.

## Stage 2: resolved explicit-Dispose Using regions

Typed HIR now lowers the existing `Using R` form when `R` is an addressable
Class value with a statically resolved, parameterless `Dispose` Sub. The region
captures the resource value in a hidden local at entry, so later rebinding of
`R` does not change which object is disposed. The HIR stores the selected
Dispose method ID and a cleanup handler on the lexical Using scope; the
verifier checks the resource local, owner type, initialization and method
identity. Ownership reports include an explicit **Dispose** action on normal
scope exit, Return, Exit and Continue. Nested Using regions report inner Dispose
before outer Dispose. This is different from native Drop and is shown separately
in debug output. Existing source execution remains interpreter-backed.

`Using R As New Resource(...)` and other Using declarations are not yet in
typed HIR because Class construction and allocation still have interpreter
semantics. The supported `Using R` form is an explicit disposal scope for an
already existing Class object; it does not prove unique ownership, automatic
destruction or `requires_drop = Yes`. Exception-path disposal is retained in
the structured HIR scope policy but the ownership report has no executable
exception edge or native unwind plan yet. A normal-flow `Try/Finally` nested
inside Using reports Try completion, Finally completion, then Dispose; a
Return or exception inside Try remains unsupported by typed HIR. No source
resource is classified as native non-Copy/requires-Drop yet, so public Move
remains gated. Stage 2 is **not complete** and MIR has not started.

Validation of this increment on Windows: **732 workspace tests** pass, including
the suite executing **113 examples**. Formatting, Clippy with warnings denied,
release workspace build and whitespace checks pass. The rebuilt release CLI
validates both game entry points. No new source examples were needed; existing
`using_dispose.valo` remains part of the example suite.

## Stage 2: structured cleanup transfers (current increment)

Typed HIR now attaches both `ExplicitDispose` and `FinallyRegion` handlers to
their owning lexical scopes. `Return`, `Exit` and `Continue` carry an ordered
cleanup chain derived from their exited scopes. The chain is innermost first,
with handlers in reverse registration order within one scope. A Return keeps
its typed expression on the transfer node: future CFG lowering must evaluate
that expression once, preserve its value, execute the chain, then return it.
The HIR verifier independently recomputes the chain from scope ownership and
rejects missing, extra, duplicated or reordered handlers. The ownership
report and stable HIR printer expose the chain for tests and debugging.

`Try`/`Finally` now supports Return, Exit and Continue from the Try body in
the typed subset. Nested Finally and Using handlers interleave by lexical
nesting: a Return from Try inside Using runs Finally before Dispose; a Continue
from Using inside Try runs Dispose before Finally. Normal fallthrough has the
same scope-handler ordering. A transfer *out of* Finally remains rejected by
typed lowering; this follows modern Visual Basic's restriction. Source
execution still uses the interpreter, and no MIR cleanup blocks exist yet.

The parser currently has one optional Catch clause, with an optional `Error`
variable and no filter. Typed HIR now represents that Catch in its own scope,
marks the exception local initialized on entry, preserves Try/Catch/Finally
ordering, and includes a Finally handler in both Try and Catch exit chains.
The handler is structural: exception matching, exceptional edges, propagation
and native unwinding are not yet lowered. The verifier checks Catch scope,
local initialization and matching handler attachment. Catch is not presented
as general VB.NET exception coverage; typed access to exception members and
multiple typed Catch clauses are still outside the present subset.

`Using R As Resource = Existing` now lowers to a typed resource local with a
resolved initializer and Dispose handler. Existing `Using R` remains supported.
`Using R As New Resource()` and other construction/collection forms remain
outside typed HIR because Class allocation and construction still depend on
the interpreter. One Using statement currently has one parsed resource;
multiple declarations in one statement are not claimed. Explicit Dispose is
not native Drop and does not imply unique ownership.

The cleanup chain describes transfer ordering but is not an executable CFG.
In particular, a Finally body that mutates ownership state can affect outer
cleanup decisions; path-sensitive ownership across exceptional and nested
cleanup execution remains to be solved before native Drop lowering. General
reference escape, native Class ownership, public Move, a truthful native
resource contract, and native exception unwinding remain deferred. Stage 2 is
incomplete. MIR has not started; the structured transfer representation is a
prerequisite, not proof that all ownership semantics are final.

Validation for this increment on Windows: **742 workspace tests** pass,
including the suite that executes **113 examples**. Formatting, Clippy with
warnings denied, release workspace build and whitespace checks pass. The
release CLI statically validates both game entry points. No new examples were
added; the new cases are HIR and verifier regression tests.

## Stage 3: first backend-neutral MIR (current increment)

Stage 3 has begun while Stage 2 remains open. The first typed, non-SSA MIR and
HIR-to-MIR CFG lowering are described in [mir.md](mir.md). It currently lowers
the verified primitive/function subset, structured branches and loops, and
normal/non-local Finally and explicit Dispose cleanup into blocks. Catch
requires exception dispatch and is rejected by MIR lowering with a controlled
unsupported result. Native Drop, public Move, native Class ownership and full
lifetime checking remain Stage 2 gaps; MIR does not pretend to solve them.
LLVM and native code generation have not started.

Validation for the first MIR increment on Windows: **760 workspace tests**
pass, including **113 executable examples**. Formatting, Clippy with warnings
denied, release workspace build and whitespace checks pass. The release CLI
statically validates both game entry points. No examples were added; the new
coverage lives in MIR lowering, cleanup CFG and verifier tests.

### Stage 3.1: MIR dataflow (current increment)

MIR now has deterministic CFG predecessor/successor, reachability and forward
worklist analysis. The verifier checks temporary definition along all reachable
paths. A whole-local, path-sensitive availability pass runs after MIR lowering:
it tracks uninitialized, available, moved and dropped possibilities, rejects
uses when any incoming path is unavailable, and validates internal Move/Drop
and explicit/call-scoped borrow conflicts. Known droppable replacements require
RHS evaluation followed by old-value Drop before Store; Drop insertion remains
future work. These operations are internal testable MIR, not public Move,
native ownership or native Drop semantics. For Each on the supported fixed
array subset now snapshots elements at entry, matching interpreter iteration
visibility. See [mir.md](mir.md) for the exact state model and limitations.

Stage 2 remains open: no truthful native owned resource or public Move;
native Class ownership, reference escape, general lifetimes, exception
dispatch/unwinding, generic iteration and runtime-only coercions remain.
Stage 3 likewise still needs projected move paths, backward liveness, drop
elaboration, conditional cleanup, exception CFG and native ABI decisions.
At the Stage 3.1 checkpoint, LLVM/JIT/machine-code work had not begun.

## Stage 3.2: experimental LLVM executable path

An optional LLVM command-line backend now translates the primitive, verified
MIR subset to host LLVM IR, verifies it with `opt`, optionally runs LLVM O2,
emits an object with `llc`, and links a native executable through `clang`.
`valo build` exposes this path without changing interpreter `valo run`.
Native execution tests cover constants, arithmetic, signedness, branches,
loops, direct/recursive calls, primitive ByRef and normal/return Finally CFG.
See [llvm-backend.md](llvm-backend.md) for the precise ABI, support matrix,
toolchain requirements and unsupported cases. Stage 2 ownership and lifetime
gaps and Stage 3.1 drop/liveness gaps remain open; the LLVM success applies
only to the documented restricted subset.

## Stage 3.3: native value semantics

The experimental backend now lowers plain Copy/no-Drop Structures, nested
field Places, tuple construction and reads, and fixed zero-based arrays with
checked indexing. Structure ByVal, return, mutable ByRef and readonly ByRef
use a documented Valo-internal aggregate ABI. Fixed-array For Each copies its
entry snapshot into separate native storage, matching interpreter mutation
visibility. Native tests build, link and execute each of the four native
examples plus focused aggregate cases. This does not close Stage 2: real
owned resources, Drop elaboration, public Move, native Class semantics,
reference lifetimes and exception unwinding remain separate work.

## Project source sets and multiline syntax

The native CLI now loads the same transitive Project as `check` and `run`
before HIR lowering. A deterministic combined declaration view supports
native calls and plain Structures from imported files. The frontend's
import-scoped validation remains authoritative; native compilation still has
a restricted entry form and whole-unit eligibility. See
[compilation.md](compilation.md).

The parser now handles structurally continued calls, declarations, generic
lists, tuple/initializer lists and expressions without `_`, while preserving
statement-ending newlines. The rule and remaining ambiguities are in
[parser.md](parser.md).

## Stage 3.4A: project semantic completeness

Native entry resolution now selects an exact semantic callable before MIR.
Parameterless `Sub Main()` exits with zero; parameterless `Function Main() As
Integer` keeps its result. Ambiguous and unsupported entry signatures are
diagnosed. The combined native Compilation retains a source index for each
callable, so HIR lowering uses its own file's Strict, Explicit, Infer and
Compare settings rather than the root file's settings. Source validation and
interpreter execution retain their existing per-file behavior.

Declaration spans carry owner paths. Cross-file namespace members can share a
namespace without collapsing equal simple names from different owners. Native
symbols encode owner and signature rather than a declaration index, making
names deterministic across equivalent compilations. The combined HIR view is
still transitional; it is not separate object compilation or a stable public
ABI. Stage 2 ownership, native Class/String/runtime semantics, Drop and
exception gaps remain open. Stage 3 still needs deeper ownership/liveness,
conditional Drop, projected moves and exception CFG.
