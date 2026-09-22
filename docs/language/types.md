# Types

Valo preserves familiar VB.NET names and adds explicit native aliases.

| VB.NET name | Native name | Width |
| --- | --- | --- |
| Byte | UInt8 | 8 unsigned bits |
| Short | Int16 | 16 signed bits |
| Integer | Int32 | 32 signed bits |
| Long | Int64 | 64 signed bits |
| UInteger | UInt32 | 32 unsigned bits |
| ULong | UInt64 | 64 unsigned bits |
| Single | Float32 | 32-bit floating point |
| Double | Float64 | 64-bit floating point |
| Boolean | Bool | Logical True/False; native ABI still under review |

These aliases are implemented. SByte/Int8, UShort/UInt16, ISize/USize, Char and
Float16 are not yet implemented. The future Char encoding and target-dependent
pointer layout must be specified before claiming support.

Unsuffixed integer literals infer Integer when they fit Int32, otherwise Long.
`CInt` converts to Int32 and `CLng` to Int64. Identifier suffix `%` denotes Integer,
and `&` denotes Long. Other literal/coercion/overflow rules still require work;
see the [migration inventory](../architecture/native-migration.md).

## Structures

```vb
Public Structure Point
    Public X As Float32
    Public Y As Float32

    Public Sub New(ByVal XValue As Float32, ByVal YValue As Float32)
        X = XValue
        Y = YValue
    End Sub

    Public Function Sum() As Float32
        Return X + Y
    End Function
End Structure
```

Structures support fields, methods, properties, constructors, generic parameters,
interfaces and operators within the implemented subset. Assignment and ByVal
preserve value semantics; ByRef can mutate the original. The interpreter uses
copy-on-write storage, not a guarantee of stack placement or zero-cost native code.

Classes currently have shared reference identity in the interpreter. Native class
ownership and deterministic destruction still need a compiler-level model. Do not
infer CLR lifetime guarantees from class syntax.

## Nullable values and strings

`T?`, `Nothing`, `.HasValue` and `.Value` remain supported. Reading an absent value
fails. The native Option/Result representation is planned.

String currently uses Rust-backed UTF-8 storage. String views, C string ownership
and zero-copy slices remain design work. Native ByVal String calls allocate a
temporary NUL-terminated byte buffer.

## Transitional behavior

The interpreter still has Variant/Empty/Null and some dynamic defaults, legacy Type
records, Currency, and old lower-bound behavior. These are active migration debt,
not the intended static systems type model. Use typed declarations or inference
from initializers, modern Structure syntax and zero-based arrays in new code.


Local, static and module declarations without `As` require an initializer for
inference. Bare `Dim X` is rejected with suggestions to add a type or initializer.
`Dim A, B As Integer` gives both names the same type. Explicitly typed variables
retain default initialization.


Omitted array lower bounds are zero. `Option Base` is rejected; there is no
module-wide setting that changes array indexing. `ReDim Preserve` retains values
when resizing within its supported constraints.
