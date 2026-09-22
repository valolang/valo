# Valo MIR: first CFG milestone

MIR is a backend-neutral, typed control-flow representation. It consumes a
verified typed HIR body and does not read the AST or interpreter state. The
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
are constants, fixed array initialization/length, Place loads/stores, typed
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
The first For Each lowering indexes its array source directly and evaluates
array length once. Full iterator semantics and reconciliation with the
interpreter's array enumeration behavior remain open.

Cleanup chains are expanded **during HIR-to-MIR lowering**, not retained as
magical MIR annotations. Return expressions are evaluated into temporaries
before a sequence of explicit cleanup blocks. Each handler becomes its own
block: Finally lowers its already typed body, while explicit Dispose lowers to
a call carrying its resolved Dispose ID and receiver Place. The same chain
lowering handles Return, Exit and Continue; normal fallthrough through a
Try/Finally or Using uses the owning scope's handler chain. Cleanup block
duplication is intentional for this initial, simple CFG. Native Drop is not
implemented, and Dispose does not imply unique ownership.

The MIR verifier checks entry/block/local/temp identity, one definition per
temp, a terminator for every block, branch targets and Boolean conditions,
return types, Place projection types, load/store types, numeric operation and
cast types, and call arity, modes and argument/result types. It is not yet a
dominance or full ownership verifier. Later CFG dataflow passes must add
definite initialization, path-sensitive moves, borrow liveness, reference
escape analysis, native Drop insertion and exceptional cleanup verification.

Typed HIR Catch is **not MIR-lowerable** because native exception dispatch and
unwind edges do not exist. `lower_body` returns an explicit unsupported error
instead of inventing a normal edge. Internal HIR Move likewise remains gated
on truthful ownership semantics. Class values and resolved Dispose calls can
appear as opaque MIR values/calls for CFG tests; their native layout and ABI
are not specified. MIR is not executable, and the source interpreter remains
the existing execution path. No LLVM or machine-code dependency exists in MIR.

Future backends should consume verified MIR after ownership/dataflow and Drop
insertion are sound. LLVM is a possible optimized native backend, but its
types, exception ABI and object format must not define Valo semantics.
