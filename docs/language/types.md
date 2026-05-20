# Types

Valo has native scalar types, arrays, classes, enums, and value types.

## Native Structures

Use `Structure` for new value-type code:

```vb
Public Structure Point
    Public X As Integer
    Public Y As Integer

    Public Sub Constructor(ByVal x As Integer, ByVal y As Integer)
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
- `Sub Constructor`
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

Prefer `Structure` in new `.valo` code and keep `Type` for migrated VBA code.

## Class vs Structure vs Type

- `Class`: reference type with identity, lifecycle, events, default properties, and object references.
- `Structure`: native value type with fields, methods, properties, constructors, and copy semantics.
- `Type`: VBA-compatible fields-only record syntax.

## Byte Arrays

Use Basic-style array syntax for byte buffers:

```vb
Dim data() As Byte
ReDim data(0 To 15)
data(0) = CByte(255)
```

C-style square-bracket array spelling is not the official Valo syntax.
