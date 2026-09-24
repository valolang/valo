# Experimental LLVM native backend

The backend consumes verified, dataflow-checked MIR. It never reads the AST,
re-resolves a name, or embeds interpreter `Value` objects. This is a restricted
native compilation path, not a replacement for Valo's interpreter.

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
`valo run` remains interpreter-based. The CLI currently compiles one source
file; imported multi-file projects need a later native project pipeline.

## Translation contract

The backend first collects native signatures for every MIR function, then
lowers their bodies. Symbols use `valo_f<resolved function ID>` within one
compilation unit, so source overloads and forward/recursive calls do not
collide. The entry wrapper `main` calls exactly one parameterless
`Function Main() As Integer` and returns its Int32 result as the process exit
code. Module initialization and a stable external Valo ABI are deferred.

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
backend does not claim source support for them. Source `Sub`/Void function
bodies are not yet represented by typed HIR; `void` currently appears only in
the LLVM trap intrinsic. FFI/extern ABI, including Boolean layout, is not
supported by this backend.

MIR value locals become entry-block LLVM allocas. Primitive ByVal parameters
are stored into those slots; primitive ByRef and ByRef ReadOnly parameters
are pointers to the caller's verified local Place. MIR temporaries become
LLVM SSA values or inline constants. Only local-root Places lower; resolved
field, tuple and index Places are rejected until aggregate layout and bounds
rules are available. MIR `Goto`, `Branch`, `Return` and `Unreachable` become
LLVM terminators. `Trap` calls `llvm.trap` and does not masquerade as a Valo
exception. The backend sees Try/Finally only as MIR CFG blocks. A primitive
Finally body can therefore run natively on normal and return paths; Dispose
still requires unresolved native Class semantics and is rejected.

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
| Try/Finally without exception dispatch | Yes | Yes / Yes | Yes for primitive bodies |
| Using with Class Dispose | Yes | Partial / Yes | No |
| Catch and exception unwind | Yes | Partial / No dispatch | No |
| Structure/tuple/array Places, For Each | Yes | Partial / Yes for supported forms | No |
| Class, Variant/dynamic, strings, async | Yes | Partial | No |
| Public Move, native Drop, unique resources | Not public | Internal only | No |

The first eligibility policy is **whole single-file module**: every function
must lower to MIR and be native-eligible, even if unreachable from Main.
Unsupported features return stage-specific diagnostics before LLVM object
emission. Native tests are unconditional Rust tests but skip toolchain-dependent
execution when `clang`, `opt`, or `llc` cannot be discovered; frontend and MIR
tests always run. The next backend milestone should add checked narrowing and
float-to-integer conversions, plain Structure layout/field Places, and a
native project/call-graph pipeline. Native exceptions, ownership/Drop, and
general reference lifetimes remain separate semantic work.
