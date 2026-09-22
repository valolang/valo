# Roadmap

Valo's authoritative direction is native systems programming with the existing
VB.NET-inspired surface preserved. The implementation remains Rust.

The [repository audit and migration plan](native-migration.md) records current
architecture, completed changes, remaining legacy debt, regression coverage and
acceptance gates.

1. Finish legacy-only removal while preserving VB.NET regression tests.
2. Complete the native primitive model and strict, explicit conversions.
3. Produce resolved typed HIR with stable symbol identities and source spans.
4. Analyze value ownership, moves, borrows, lifetimes and cleanup.
5. Lower to typed control-flow IR with explicit effects and target layouts.
6. Add a native backend and build artifacts; retain the interpreter for tooling.
7. Develop specialization, compile-time execution, SIMD, CPU parallelism, async
   state machines and a GPU-safe subset with explicit address spaces.

No compatibility mode, mandatory managed runtime or tracing GC is planned.
Embedding, JIT and hot reload remain future possibilities, not implemented tools.
