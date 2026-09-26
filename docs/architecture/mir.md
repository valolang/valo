# Valo MIR: first CFG milestone

MIR is a backend-neutral, typed control-flow representation. It consumes a
verified typed HIR body and does not read the AST or interpreter state. The
Stage 3.4B path includes managed String constants, clone, concat, comparison,
length, formatting, replacement and explicit Drop. `DropCandidate` is a
transient scope-exit obligation; `drop_elaboration` expands it before MIR
verification. A verifier rejects an unelaborated candidate or a reachable
owned String temporary used other than exactly once. `Replace` models the
order RHS evaluation, old-value release, new-value store. Scope-exit
obligations are emitted for Return, normal branches, loop iterations and
normal Finally/Using CFG. CFG ownership analysis rejects Drop unless the
whole local is definitely available on every incoming path; conditional
Drop flags and projected managed ownership remain future work. Native
String Drop is distinct from Dispose and does not cover exceptional unwind.
See [native-runtime.md](native-runtime.md).

The
current module is `core/src/mir`: `ir` defines data, `lower` constructs CFGs,
`verify` checks their structure and types, and `debug` prints deterministic
text for tests. `lower_body` verifies HIR and runs the existing ownership pass
before lowering, then verifies the resulting MIR. `lower_module` collects
individually lowered functions; globals and inter-function linking are future
work.

The first MIR is **non-SSA**. HIR locals become typed MIR storage locations;
each computed result gets a deterministic typed temporary. Basic blocks have
an instruction list and exactly one terminator. `Goto`, Boolean `Branch`,
`Return`, `Trap`, and `Unreachable` are implemented. The initial instructions
are constants (including typed aggregate zero values), tuple construction,
fixed array initialization/length, Place loads/stores, typed
arithmetic/comparisons/casts, and calls with resolved function identity,
signature and argument passing modes. MIR Places retain local roots and
resolved field, tuple-field and indexed projections. An indexed Place load or
store retains Valo's bounds-check obligation; bounds-check elimination is not
implemented.

Lowering currently covers HIR's primitive expressions and conversions,
assignments, resolved calls, If/ElseIf/Else, While, all typed Do forms, For,
and the typed fixed-array For Each subset. For evaluates start, end and step
once; a zero step reaches a typed MIR trap. Exit and Continue use the HIR loop
identity to jump to the appropriate loop exit or next-test/step destination.
The fixed-array For Each lowering now snapshots the element sequence once at
entry, then indexes that snapshot. This matches the interpreter's enumeration
snapshot when the source array is changed in the body. The source subset is
still one-dimensional, zero-based and fixed; plain Structure elements can
enter the typed native subset, while generic iteration remains unresolved.
The For bound and Step
expressions are evaluated once, and zero Step reaches a trap.

Cleanup chains are expanded **during HIR-to-MIR lowering**, not retained as
magical MIR annotations. Return expressions are evaluated into temporaries
before a sequence of explicit cleanup blocks. Each handler becomes its own
block: Finally lowers its already typed body, while explicit Dispose lowers to
a call carrying its resolved Dispose ID and receiver Place. The same chain
lowering handles Return, Exit and Continue; normal fallthrough through a
Try/Finally or Using uses the owning scope's handler chain. Cleanup block
duplication is intentional for this initial, simple CFG. Native String Drop
is now elaborated for normal exits; Dispose does not imply unique ownership.

The MIR verifier checks entry/block/local/temp identity, one definition per
temp, definition on every reachable path before use, a terminator for every
block, branch targets and Boolean conditions,
return types, Place projection types, load/store types, numeric operation and
cast types, and call arity, modes and argument/result types. It is not yet a
dominance or full ownership verifier. Later CFG dataflow passes must add
projected move paths, general borrow liveness, reference escape analysis,
native Drop insertion and exceptional cleanup verification.

## Stage 3.1 dataflow foundation

`analysis::cfg` derives deterministic successors, predecessors, entry
reachability and reverse postorder exclusively from terminators. Structurally
valid unreachable blocks remain permitted. `analysis::dataflow` provides a
finite-lattice forward worklist solver with stable block order. A second
verifier analysis intersects temporary definitions at joins; a temp defined
on only one branch cannot be used after the join.

`analysis::ownership` runs after MIR verification during HIR lowering. It
tracks whole-local states on reachable paths. Parameters start available;
other locals start uninitialized. A whole-local Store makes a local available;
internal Move of a known non-Copy local makes it moved; Move of a known Copy
local leaves it available; internal Drop makes a known droppable local dropped.
The join is the union of possible states, so `Available + Moved` is
`MaybeUnavailable`, and reading, borrowing, moving or dropping it is rejected.
The same applies to partial initialization at a branch or loop join.
Reassignment of a known droppable available local requires an explicit
replacement operation or Drop after RHS evaluation. String, managed
Structures/tuples, Class references, Variant boxes and Collections use
`Replace` at supported mutable Places.

Internal `Move`, `Drop`, `BorrowStart` and `EndBorrow` are separate MIR
instructions. They currently support compiler/test-only ownership scenarios;
source `Move` and native resources remain gated. Drop is distinct from a
resolved Dispose call. Explicit borrows remain live until EndBorrow, while
ordinary ByRef call arguments are checked together and end at that Call.
Whole-local, distinct-field and constant-index overlap follows conservative
Place rules; unknown indices may overlap. The analysis rejects use after
Move/Drop, double Drop, move while borrowed and conflicting live borrows. It
does not solve partial moves, long-lived references, general NLL, escaped
borrows, native unwind paths or conditional Drop flags. Unknown ownership
contracts are not silently classified as Copy or native droppable.

Current order: verified HIR, HIR ownership checks, MIR lowering,
type-driven Drop elaboration, structural MIR verification, CFG/dataflow analysis.
Future work includes backward local liveness, projected move paths,
conditional Drop flags and managed-array Drop elaboration, reference escape, exceptional
edges and stronger lifetime analysis. The restricted LLVM backend consumes
this verified MIR; ownership-sensitive calls and exceptional cleanup still
need work.

Typed HIR Catch is **not MIR-lowerable** because native exception dispatch and
unwind edges do not exist. `lower_body` returns an explicit unsupported error
instead of inventing a normal edge. Internal HIR Move likewise remains gated
on truthful ownership semantics. Restricted Class references and native
Collection/Variant operations now have backend-neutral MIR forms; their native
layout lives entirely in LLVM/runtime. Resolved Dispose calls still lack a
native implementation. MIR itself is not executable and contains no LLVM or
machine-code dependency.

Native backends must consume verified MIR and accept only semantics their
current subset can lower. LLVM types, exception ABI and object format must not
define Valo semantics.

An experimental backend now consumes the **eligible subset** of
verified MIR. Its local allocas, SSA temporaries, target data layout and
LLVM tool invocations live entirely in `backend/llvm`, not in MIR. Internal
Ownership-sensitive public Move and managed arrays remain gated. Managed
String/aggregate/reference Drop on normal paths is supported. See
[llvm-backend.md](llvm-backend.md).
