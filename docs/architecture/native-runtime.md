# Experimental native String runtime

Valo's source interpreter stores immutable strings behind `Rc<String>` and
copies handles on ordinary assignment. The native representation is independent
of that Rust implementation. A `String` is semantically copyable: assigning or
passing ByVal leaves the source usable. Native copies retain a shared immutable
handle, while scope exit releases each owned handle. This makes `String`
`Copy = Yes` and `requires_drop = Yes`; `Copy` does **not** mean bitwise copy.

The internal ABI represents a String as one pointer. Null is the valid empty
string. A non-null pointer addresses `{ u64 references, u64 byte_len,
u64 scalar_len, u8 data[byte_len] }` with target C alignment. LLVM literal
globals use the same header and `references = UINT64_MAX`; their UTF-8 bytes
are immortal. Dynamically concatenated/formatted strings have a reference
count starting at one, allocated with `malloc`, and are freed with `free` when
the last reference is released. This is a private, unstable Valo runtime ABI,
not a C string or a Rust `String`. There is no terminator requirement or
automatic FFI conversion. The runtime is currently single-threaded; the
reference count is not atomic. Allocation failure aborts. An internal
`VALO_RUNTIME_ASSERT_CLEAN=1` test mode aborts on leaked dynamic strings at
normal process exit.

UTF-8 is the storage encoding. For the supported native subset, `Len` counts
Unicode scalar values, matching the current interpreter's `.chars().count()`;
`LenB` and indexing are not yet native. This differs from .NET's UTF-16 code
unit model and is an existing Valo interpreter behavior that needs an explicit
future language-level compatibility decision. The default `Dim S As String`
initializes the empty value, represented by null. The current frontend rejects
`Dim S As String = Nothing` because `Nothing` requires a Class object type;
the native null handle is an internal encoding of empty String, not source
`Nothing`.

The C ABI helpers in `core/native_runtime/string.c` accept/return the pointer
handle: `__valo_string_clone` retains, `__valo_string_release` releases,
`__valo_string_concat_consume` consumes two handles and returns one,
`__valo_string_compare_consume` and `__valo_string_len_consume` consume their
arguments, and numeric interpolation helpers produce new handles. LLVM IR
declares these centrally. `valo build` compiles the runtime source with the
discovered Clang and links its object automatically for executable output.
`--emit=obj` preserves the Valo object with unresolved runtime symbols for a
later link. The compiler never passes interpreter `Value` objects to LLVM.

MIR `CloneString` turns an addressable String into an owned temporary.
`StringConcat`, `StringCompare`, and `StringLen` consume String temporaries.
Calls transfer ByVal String handles to callee parameters; returns transfer
them to callers. `Store` initializes, while `Replace` evaluates its RHS
first, releases the old whole-local value, then stores the new handle. Thus
`S = S` clones before release. HIR supplies scope-exit candidates and cleanup
ordering; `mir::drop_elaboration` converts known String obligations to explicit
`Drop`. CFG ownership analysis checks availability and rejects an uncertain
drop. No conditional drop flags exist yet; such cases are rejected. Normal
scope exit, Return, loop iteration, and normal Finally paths are covered.
`Using` Dispose remains a distinct operation. Trap/abort and future exception
unwind paths do not run String cleanup.

Binary String comparisons are content-based, with UTF-8 ordinal ordering.
`Option Compare Text` remains semantically distinct in HIR and is rejected
for native comparison; it is not silently treated as Binary. Native `&`
concatenation supports String and currently supported primitive-to-String
rendering. Interpolation supports String holes, integer and Boolean default
rendering, and `0.0` through `0.000000` fixed decimal formatting for floating
holes. Other formats, alignment, default floating formatting, String indexing,
String arrays, and managed aggregates remain unsupported natively. A Structure
or tuple containing String has a managed drop obligation, but aggregate
clone/drop elaboration is not implemented; native compilation rejects it.
Public `Move()` remains gated.
