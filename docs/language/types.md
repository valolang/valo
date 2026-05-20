# Types

Valo has native scalar types, arrays, classes, enums, and simple value records.

## Native Structures

Use `Structure` for new value-record code:

```vb
Public Structure Point
    Public X As Integer
    Public Y As Integer
End Structure

Dim p As Point
p.X = 10
p.Y = 20
```

Structures currently share the existing user-defined record behavior: fields only, default field values, assignment by value, function return support, parameter support, and module import support. They do not add methods, constructors, inheritance, interfaces, or generics.

## VBA-Compatible Types

`Type ... End Type` remains supported for VBA compatibility and maps to the same simple record representation:

```vb
Public Type Point
    X As Integer
    Y As Integer
End Type
```

Prefer `Structure` in new `.valo` code and keep `Type` for migrated VBA code.

## Byte Arrays

Use Basic-style array syntax for byte buffers:

```vb
Dim data() As Byte
ReDim data(0 To 15)
data(0) = CByte(255)
```

C-style square-bracket array spelling is not the official Valo syntax.
