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
imported source declarations in canonical-path order. Each Function and Sub
keeps a source-unit index. Its HIR body uses the owning file's `Option Strict`,
`Option Explicit`, `Option Infer` and `Option Compare` settings. Mixed settings
are allowed. The interpreter continues to use each module's own Program.

Declarations retain source spans and owner paths. Native HIR canonicalizes
resolved type references and disambiguates declarations that would collide in
the combined view. A simple imported name resolves only when its visible
candidate is unique; explicit qualification such as `A.Vector` selects the
owner. Equal namespace paths in separate files contribute to one namespace;
true duplicate types or call signatures are errors. Source display names are
not native symbol identity. Function/field IDs remain compilation-local, and
MIR and LLVM never load sources or resolve imports. The combined declaration
view is still transitional: it is not a separate-compilation or persistent
symbol database, and unsupported runtime/foreign ABI constructs remain
ineligible.

Entry resolution runs on the semantic Compilation before MIR. Exactly one
parameterless `Sub Main()` or `Function Main() As Integer` is accepted,
including a declaration in an imported file. Private Main is accepted.
The platform wrapper returns zero after Sub or returns the Function result.
Duplicate Main candidates and unsupported signatures receive entry-point
diagnostics. String-array argument forms await native String/array ABI support.

A future manifest can replace the present directory-based discovery policy
with explicit source roots, entry, dependencies, native libraries and target.
It need not change HIR/MIR or the LLVM backend. Separate object compilation
and incremental source invalidation remain future work.

Stage 3.4C native managed values do not alter source discovery or Compilation
identity. Class, Collection and Variant values cross files through resolved
HIR/MIR types and the private native ABI; LLVM still never reads Imports or
source files. Whole-unit eligibility means an unsupported method or
module-level storage construct anywhere in a project can stop a native build
before other functions are emitted.
