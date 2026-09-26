# Experimental native managed values

The interpreter is the current semantic reference. Its `Collection` is an
ordered, optionally keyed, heterogeneous container. Numeric positions are
1-based. Assignment shares the container; `For Each` takes an entry snapshot
of the elements. Class assignment shares object identity, `Nothing` is a null
reference, and `Is`/`IsNot` compare identity. The interpreter implements these
with Rust `Rc`/`RefCell`; those types are not part of the native ABI.

In the games, Raycaster stores map-row Strings, Vec2 Structures, and Class
references (including objects later viewed through `IThing`) in Collections.
Breakout stores Paddle, Ball, Brick and other Class references and iterates
them as Variant. Both use Add, Count and For Each; Breakout also removes by
numeric position and Raycaster inserts before a numeric position. Neither
game currently needs Collection keys. Interface dispatch, Class inheritance,
constructors and instance methods are still outside the supported native
subset.

## Managed copy and destruction

`TypeProperties::copy_kind` distinguishes trivial bitwise Copy from managed
Copy and non-Copy/unknown types. String, Variant, Collection and supported
Class references are semantically Copy but require a retain on copy and a
release on Drop. A Structure or tuple containing one of these values inherits
managed Copy and Drop recursively. LLVM lowering of a verified `CloneManaged`
walks fields in declaration order to acquire shares; explicit `Drop` walks
them in reverse declaration order to release shares. Assignment evaluates and
acquires the RHS before releasing the destination, so self-assignment is safe.
MIR Drop elaboration remains path-sensitive and rejects uncertain ownership
without drop flags. Fixed arrays containing managed elements remain gated.

The restricted Class ABI is one pointer to an allocation beginning with a
64-bit non-atomic reference count and a field-destructor callback pointer,
followed by fields in resolved declaration order. LLVM's target data layout
determines padding and alignment; the native runtime allocates and
zero-initializes the object. Null is `Nothing`. A generated destructor releases
managed fields when the last reference is released. ByVal copies retain;
ByRef and ByRef ReadOnly pass the variable address; returns transfer a share.
Collection references use the same passing modes. The current native
creation subset excludes constructors, user `Terminate` finalizers, field
initializers, inheritance and abstract Classes. ARC cycles can leak. The runtime is single-threaded and
allocation failure aborts; no exception unwinding occurs.

The restricted native Variant is an immutable, reference-counted box with a
unique per-type tag, a generated drop callback and an aligned payload. Boxing
transfers an owned value; unboxing checks its exact tag and clones the payload
according to its type. This is an erased native value, never an interpreter
`Value` or a bare `void**` element. Dynamic coercions beyond exact-tag `CType`,
`DirectCast`, `TryCast` and runtime reflection remain unsupported. Distinct
type tags are writable private LLVM globals so optimization cannot merge
equal-address constants. The current payload ABI supports types requiring at
most eight-byte alignment; wider alignments need an explicit runtime change.

The native Collection is an ARC handle to `{refcount, length, capacity,
tagged-element pointers}`. Add transfers a Variant share, Item returns a
retained share, Remove releases one element, and final release destroys items
in reverse order. Numeric indexing is 1-based; a missing Before position
appends. Iteration clones the element sequence once on entry, so changes to
the original Collection do not change which elements are visited. Element
Class references still share object identity. MIR owns the snapshot local and
emits its Drop on normal completion, Exit, Return and outer-loop transfers;
Continue keeps the snapshot. Key lookup/insertion, After positioning and
general Variant coercions receive controlled unsupported diagnostics.

Invalid numeric positions abort through the current runtime trap path; native
exception dispatch is not implemented. A null Collection handle also aborts
when used. These are runtime error paths, not unchecked memory accesses.

`core/native_runtime/object.c`, `dynamic.c`, and `collection.c` implement the
private C ABI and are compiled automatically with the existing String runtime
when building an executable. `VALO_RUNTIME_ASSERT_CLEAN=1` makes normal
process exit abort if dynamically allocated objects, boxes, Collections or
Strings remain live. This checks acyclic test programs, not ARC cycles.
The ABI, symbol names and object layout are experimental and cannot be used
as a general C FFI contract.
