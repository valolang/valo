# Compilation units

`load_project` is the shared source-discovery path for `run`, `check`, and
`build`. Its root is the requested source file. Imports currently locate
case-insensitive `.valo` files relative to the importing file; this filename
convention is a source-discovery rule, not the semantic meaning of `Imports`.
Each canonical path is parsed once, transitive imports are followed, and an
import cycle or ambiguous case-only path produces a diagnostic. The resulting
`Project` retains every parsed program, import edge, file identity and source
map. Project validation collects declarations before checking bodies, using
each source file's import scope. Diagnostics retain the originating file span.

The experimental native path validates that same Project, then creates a
deterministic whole-module `Compilation` view: root declarations first, then
imported source declarations in canonical-path order. This gives HIR one set
of globally unique function and field IDs; MIR and LLVM never load sources or
resolve imports. A native build currently lowers all functions in that view.
The combined view is a transitional native subset: source modules with
colliding unqualified declarations or functions needing a runtime/foreign ABI
may remain ineligible even when the interpreter accepts qualified calls.
Files with differing `Option Strict`, `Option Explicit` or `Option Compare`
settings are rejected for native compilation until HIR lowering retains
per-source options; project validation and interpreter execution still use
each file's own settings.
`Function Main() As Integer` remains the native entry form; interpreter
`Sub Main()` remains valid Valo but cannot yet be native entry.

A future manifest can replace the present directory-based discovery policy
with explicit source roots, entry, dependencies, native libraries and target.
It need not change HIR/MIR or the LLVM backend. Separate object compilation
and incremental source invalidation remain future work.
