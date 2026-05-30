# Types

Valo has native scalar types, arrays, classes, enums, and value types.

## Native Structures

Use `Structure` for new value-type code:

```vb
Public Structure Point
    Public X As Integer
    Public Y As Integer

    Public Sub New(ByVal x As Integer, ByVal y As Integer)
        X = x
        Y = y
    End Sub

    Public Function Sum() As Integer
        Return X + Y
    End Function

    Public Property Get IsZero() As Boolean
        Return X = 0 And Y = 0
    End Property
End Structure

Dim p As New Point(10, 20)
Console.WriteLine(p.Sum())
```

Structures are value types. Assignment, `ByVal` parameter passing, and function returns copy the value. `ByRef` parameters and mutating methods called on a variable can update the original structure.

Structures support:

- Fields
- `Sub` and `Function` methods
- `Property Get` and `Property Let`
- `Sub New` constructors with at least one parameter
- Default `Property Get` indexers
- Module imports, including qualified construction

Structures do not support inheritance, interfaces, events, `WithEvents`, `Class_Initialize`, `Class_Terminate`, `Terminate`, reference identity with `Is`, `Set` assignment, or `Nothing`.

Calling a mutating structure method requires an assignable receiver such as a variable or `ByRef` parameter. Temporary values are treated as copies.

## VBA-Compatible Types

`Type ... End Type` remains supported for VBA compatibility and is fields-only:

```vb
Public Type Point
    X As Integer
    Y As Integer
End Type
```

Fields inside a `Type` use plain VBA UDT syntax. Do not prefix fields with `Public`, `Private`, or `Dim`.

Prefer `Structure` in new `.valo` code and keep `Type` for migrated VBA code.

## Class vs Structure vs Type

- `Class`: reference type with identity, lifecycle, events, default properties, and object references.
- `Structure`: native value type with fields, methods, properties, constructors, and copy semantics.
- `Type`: VBA-compatible fields-only record syntax.

## Nullable Types

Valo supports VB.NET-style nullable types for both value and reference types using the `?` suffix.

```vb
Dim age As Integer? = Nothing
If age Is Nothing Then
    Console.WriteLine("Age is unknown")
End If

age = 25
If age.HasValue Then
    Console.WriteLine("Age is: " & age.Value)
End If
```

### Synthetic Properties

Nullable types expose two read-only properties:

- `.HasValue`: Returns `True` if the variable contains a value, `False` if it is `Nothing`.
- `.Value`: Returns the underlying value. Accessing `.Value` when the variable is `Nothing` will result in a runtime error.

### Lifted Operators

Arithmetic and logical operators are "lifted" for nullable types. If either operand is `Nothing`, the result of the operation is `Nothing`.

```vb
Dim a As Integer? = 10
Dim b As Integer? = Nothing
Dim sum As Integer? = a + b ' Result is Nothing
```

### Reference Types

Reference types (like `String` or classes) can also use the nullable suffix for clarity, though they already support `Nothing`.

```vb
Dim s As String? = Nothing
```

## Byte Arrays

Use Basic-style array syntax for byte buffers:

```vb
Dim data() As Byte
ReDim data(0 To 15)
data(0) = CByte(255)
```

C-style square-bracket array spelling is not the official Valo syntax.
