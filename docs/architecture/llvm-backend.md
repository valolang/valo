# Experimental LLVM native backend

The backend consumes verified, dataflow-checked MIR. It never reads the AST,
re-resolves a name, or embeds interpreter `Value` objects. This is a restricted
native compilation path, not a replacement for Valo's interpreter.

Stage 3.4B adds the private native String handle and automatically linked C
runtime described in [native-runtime.md](native-runtime.md). Native String
values are LLVM `ptr` handles, not ordinary bitwise-copy aggregates. MIR
`CloneString`, `Replace`, and explicit `Drop` determine retain/release points.

## Toolchain and target

The first integration uses the **official LLVM command-line tools** (`clang`,
`opt`, `llc`) behind `core/src/backend/llvm`. This avoids linking LLVM libraries
into `valo_core` and keeps normal Cargo builds and the existing three-platform
CI independent of a system LLVM installation. The tested toolchain is
[LLVM 22.1.8 for Windows x64](https://github.com/llvm/llvm-project/releases/tag/llvmorg-22.1.8)
with MSVC 14.44 and the Windows SDK. No Rust dependency was added. LLVM's
Apache-2.0 WITH LLVM-exception license applies to the separate toolchain;
the compiler remains MIT-licensed.

Set `VALO_LLVM_BIN` to the directory containing those tools, or put all three
on `PATH`. `clang` supplies the host triple and data layout from an LLVM IR
target probe. Cross-compilation, CPU feature selection, alternative target
ABIs and LLVM version compatibility beyond the tested release are not yet
promised. `llc` uses the host target machine to emit `.obj` on Windows or `.o`
on Unix-like hosts. The `clang` driver links that object using the host linker;
on the tested Windows machine it discovers Visual Studio's MSVC linker.

`valo build examples/native/native_control_flow.valo --release` creates
`target/valo-native/native_control_flow.exe` on Windows. `--emit=llvm-ir` and
`--emit=obj` preserve the corresponding artifact; `-o <path>` chooses an
output path. The default is an executable. Build intermediates live in an
isolated temporary directory and are removed on success or failure. Existing
`valo run` remains interpreter-based. `build`, `check` and `run` use the same
transitive project loader; native build constructs one combined MIR module.

## Translation contract

The backend first collects native signatures for every MIR function, then
lowers their bodies. Symbols encode the source-resolved owner, callable name,
passing modes, parameter types and return type as hexadecimal UTF-8 following
`valo_`; they do not depend on file enumeration or function index. Overloads
and equal display names in different namespaces remain distinct. The semantic
Compilation resolves one exact entry ID before MIR. The platform `main`
wrapper calls that ID: parameterless `Sub Main()` returns process code zero,
while parameterless `Function Main() As Integer` returns its Int32 result.
Ambiguous or unsupported Main declarations are rejected before codegen.
Module initialization and a stable external Valo ABI are deferred.

| Current canonical Valo type | LLVM representation |
| --- | --- |
| Byte | `i8` |
| Int16 | `i16` |
| Int32, UInt32 | `i32` |
| Int64, UInt64 | `i64` |
| Single, Double | `float`, `double` |
| Boolean | `i1` within this experimental Valo-to-Valo ABI |

LLVM integer types have no signedness; every divide, remainder, comparison
and widening cast uses the resolved Valo type. The frontend does not yet have
distinct canonical `Int8`, `UInt16`, `ISize`, or `USize` variants, so the
backend does not claim source support for them. Source `Sub` bodies
are represented by typed-HIR/MIR Void returns and direct Void calls.
FFI/extern ABI, including Boolean layout, is not
supported by this backend.

MIR value locals become entry-block LLVM allocas. ByVal parameters are stored
into independent slots; ByRef and ByRef ReadOnly parameters point to the
caller's verified Place. MIR temporaries become LLVM SSA values or inline
constants. Local, Structure-field, tuple-field and indexed Places lower through
one address-walking routine. MIR `Goto`, `Branch`, `Return` and `Unreachable` become
LLVM terminators. `Trap` calls `llvm.trap` and does not masquerade as a Valo
exception. The backend sees Try/Finally only as MIR CFG blocks. A primitive
Finally body can therefore run natively on normal and return paths; Dispose
still requires unresolved native Class semantics and is rejected.

## Stage 3.3 native value layout

Plain Copy/no-Drop Structures have distinct named LLVM types in source
declaration order (`%valo_t0`, `%valo_t1`, ...), including empty Structures.
Fields retain declaration order. Resolved FieldIds map to positions within the
owner's field list; chained `getelementptr` operations address nested fields.
LLVM's target data layout determines padding, alignment and offsets. There is
no packing or promised C ABI. Default Structure locals receive
`zeroinitializer`. Loads/stores copy the value. ByVal arguments and returns use
direct LLVM aggregate values in the experimental Valo-internal ABI; ByRef and
ByRef ReadOnly pass the actual Place address. Recursive value Structures and
fields with unknown ownership/layout are rejected.

Plain tuples use anonymous LLVM structs in element order. HIR and MIR carry
tuple construction; positional and named reads resolve to the same field
index. Tuple loads and assignments copy the aggregate value, and supported
readonly tuple borrows pass its address. The source validator does not permit
tuple-field assignment, so native compilation does not add that behavior.

`TypeName::Array` records an element type but no fixed bound. Native eligibility
derives each local and temporary's fixed length from verified MIR initializers,
loads, stores and snapshots. The native representation is an internal
`{ i64 length, ptr data }` descriptor; each fixed local and array-producing
temporary has distinct entry-block `[N x T]` backing storage. Initialization
zeroes elements at the declaration point, including each loop execution.
Indexing checks bounds after signed/unsigned extension to i64, then traps on
failure before the GEP. Array stores copy all elements into the destination's
own backing storage. For Each snapshots elements once into separate backing
storage at entry, matching interpreter mutation visibility. Current source
typing does not treat a whole array variable as an ordinary scalar value, so
general whole-array assignment, array parameters/returns, array fields,
dynamic arrays and ReDim remain unsupported. The fixed native subset has a
current stack limit of one million elements per array and no zero-element
source declaration. Arrays of plain Copy Structures work for indexed access.

Boolean uses LLVM `i1` for registers, locals, addressable Structure fields,
tuple items and fixed-array elements. Target data layout determines physical
allocation; the tested host allocates one byte per addressable `i1`. This is
only a Valo-internal experimental ABI. Aggregate copying is limited to values
with a known Copy/no-Drop contract. Compiler-managed String Drop works on
normal CFG paths; other native Drop and ownership-sensitive Move remain
unsupported.

Add/subtract/multiply emit LLVM integer operations without `nsw` or `nuw`,
preserving the frontend's wrapping integer model. Floating operations use
LLVM floating instructions. Integer and floating division check for zero;
signed division also traps on minimum-value divided by `-1` rather than
letting LLVM's undefined division case reach code generation. Integer
remainder has the same guards. Power and floating remainder remain
unsupported. Float comparisons are ordered except `NotEqual`, which is
unordered so NaN compares unequal. This is the backend's currently tested
IEEE-style predicate choice; broader Valo floating semantics need additional
differential tests.

Checked casts lower only where semantics are known: integer widening, integer
to float, Single to Double, and equal-width signed/unsigned conversion with
a runtime range guard. Narrowing and float-to-integer casts are rejected
until the frontend/backend contract can preserve checked overflow and Valo's
round-to-even conversion semantics. This is deliberate; a bare LLVM `trunc`
or `fptosi` would be wrong for those cases.

`opt -passes=verify` checks generated LLVM before object emission. Release
builds run `opt -passes=default<O2>` and verify again; debug builds retain
unoptimized IR. `llc -filetype=obj` emits the object, and the `clang` driver
links it. Tool exit status and stderr are reported by stage. No LLVM type or
handle enters HIR or MIR.

## Support matrix

| Source feature | Interpreter | typed HIR / MIR | LLVM executable |
| --- | --- | --- | --- |
| Primitive locals, arithmetic, comparison, supported casts | Yes | Yes / Yes | Yes, restricted casts |
| If, While, Do, For, Exit, Continue, Return | Yes | Yes / Yes | Yes |
| Direct functions, recursion, primitive ByVal/ByRef | Yes | Yes / Yes | Yes |
| ByRef ReadOnly primitive calls | Yes | Yes / Yes | Yes, pointer ABI |
| Plain Copy Structure, nested Structure, field Place | Yes | Yes / Yes | Yes |
| Structure ByVal, return, ByRef, ByRef ReadOnly | Yes | Yes / Yes | Yes, internal aggregate ABI |
| Tuple construction, copy, field read, readonly borrow | Yes | Yes / Yes | Yes |
| Fixed primitive array index and mutation | Yes | Yes / Yes | Yes, checked bounds |
| Fixed array of plain Structure | Yes | Yes / Yes | Yes, indexed access |
| Fixed-array For Each | Yes | Yes / Yes | Yes, entry snapshot |
| Whole-array scalar assignment, array argument/return | Restricted | No / partial | No |
| Try/Finally without exception dispatch | Yes | Yes / Yes | Yes for primitive bodies |
| Using with Class Dispose | Yes | Partial / Yes | No |
| Catch and exception unwind | Yes | Partial / No dispatch | No |
| Structure methods, aggregate constructors, dynamic arrays | Yes | Partial | No |
| String literals, copy, ByVal/ByRef, return, `&`, comparison, `Len` | Yes | Yes / Yes | Yes, Binary comparison |
| Fixed numeric interpolation formats | Yes | Yes / Yes | Yes, restricted formats |
| Option Compare Text String comparison | Yes | Yes / Yes | Controlled unsupported |
| String in Structure/tuple/fixed array | Yes | Partial | No managed aggregate contract |
| Class, Variant/dynamic, async | Yes | Partial | No |
| Public Move, unique resources | Not public | Internal only | No |
| Compiler-managed String Drop | Interpreter Rc | Yes / Yes | Yes, normal CFG only |

The current eligibility policy is **whole compilation unit**: every function
must lower to MIR and be native-eligible, even if unreachable from Main.
Unsupported features return stage-specific diagnostics before LLVM object
emission. Native tests are unconditional Rust tests but skip toolchain-dependent
execution when `clang`, `opt`, or `llc` cannot be discovered; frontend and MIR
tests always run. The next backend milestone should add checked narrowing and
float-to-integer conversions, native call-graph eligibility and more
aggregate initialization coverage. Native exceptions, general ownership/Drop, and
general reference lifetimes remain separate semantic work.

`valo build` now shares `Project` source discovery and import-scoped validation
with `check` and `run`. The frontend then constructs a deterministic combined
declaration view for native HIR/MIR lowering. LLVM receives only MIR. A
three-file test imports a Structure and overloaded function, then links and
executes successfully. Each body retains its source file's Option settings;
qualified namespace functions and `Sub Main()` also execute natively. See
[compilation.md](compilation.md) for the transitional compilation-unit model.
