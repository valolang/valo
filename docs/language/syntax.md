# Syntax Overview

Valo is a modern Basic-inspired language. It retains the productive simplicity of Basic while introducing modern ergonomics.

## Basic Syntax

Valo is line-oriented. Statements are typically ended by a newline, although multiple statements can be placed on a single line using a colon (`:`).

```vb
Dim x As Integer : x = 10
Console.WriteLine(x)
```

### Comments
Use a single quote (`'`) for comments.

```vb
' This is a comment
Dim name As String ' Inline comment
```

## Data Types

Local, static and module variables need a declared type or an initializer.
`Dim Count = 10` infers Integer; `Dim Count` is an error. A shared `As` clause
applies to the preceding names: `Dim A, B As Integer` declares two Integers.

Valo is moving to strict static typing. Other dynamic paths and explicit Variant
behavior remain migration debt in the [audit](../architecture/native-migration.md).

*   `Byte`: 8-bit unsigned integer.
*   `Integer` / `Int32`: 32-bit signed integer.
*   `Long` / `Int64`: 64-bit signed integer.
*   `Int64`: 64-bit signed integer.
*   `Double`: 64-bit floating point number.
*   `String`: UTF-8 encoded string.
*   `Boolean`: `True` or `False`.
*   `Variant`: A flexible type that can hold any value.
*   User-defined `Structure`, transitional legacy `Type`, and `Enum`.

Native Valo code should use `Structure` for simple value records:

```vb
Public Structure Point
    Public X As Integer
    Public Y As Integer
End Structure
```

Structures may also declare methods, properties, and `Sub New` constructors, while keeping value-copy semantics.

Byte arrays use Basic-style array declarations:

```vb
Dim data() As Byte
ReDim data(0 To 3)
data(0) = CByte(255)
```

## Variables and Constants

### Declarations
Variables are declared using `Dim`, `Public`, or `Private`.

```vb
Dim x As Integer
Public y As String = "Hello"
```

### Constants
Constants are declared using `Const`.

```vb
Const PI = 3.14159
```

## Control Flow

### If Statement
```vb
If x > 10 Then
    ' Logic
ElseIf x < 5 Then
    ' Logic
Else
    ' Logic
End If
```

### Select Case
```vb
Select Case x
    Case 1
        ' One
    Case 2 To 5
        ' Range
    Case Is > 10
        ' Comparison
    Case Else
        ' Fallback
End Select
```

### Loops
```vb
' For Loop
For i = 0 To 10 Step 2
    ' Logic
Next i

' For Each Loop
For Each item In collection
    ' Logic
Next item

' While Loop
While condition
    ' Logic
Wend

' Do Loop
Do While condition
    ' Logic
Loop

Do
    ' Logic
Loop While condition

Do
    ' Logic
Loop Until condition

' Deterministic cleanup
Using resource As New Resource()
    resource.Use()
End Using
```

### Exiting and continuing a loop

`Exit For`, `Exit While`, and `Exit Do` leave a loop. `Continue For`,
`Continue While`, and `Continue Do` skip to its next iteration.

```vb
For i = 1 To 10
    If i Mod 2 = 0 Then Continue For
    Console.WriteLine(i)
Next i
```

Both name the kind of loop they act on, so they apply to the nearest enclosing
loop of that kind rather than to the innermost loop. That is what lets an inner
loop hand control back to an outer one:

```vb
For i = 1 To 4
    Do
        If i Mod 2 = 0 Then Continue For   ' advances the For, not the Do
        Console.WriteLine(i)
        Exit Do
    Loop
Next i
```

Using `Continue For` outside a `For` loop is rejected during validation.

## Procedures

### Subs
Procedures that do not return a value.

```vb
Sub Greet(ByVal name As String)
    Console.WriteLine("Hello " & name)
End Sub
```

### Functions
Procedures that return a value.

```vb
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b ' Modern native return
    ' Add = a + b ' VBA-style return is also supported
End Function
```

## Arrays

Valo supports multidimensional arrays with custom bounds.

### Declarations
```vb
Dim matrix(1 To 3, 0 To 5) As Integer
Dim dynamic() As String
```

### Resizing
```vb
ReDim dynamic(0 To 10)
ReDim Preserve dynamic(0 To 20) ' Preserves existing values
```

### Array Built-ins
- `Array(1, 2, 3)`: Create a Variant array.
- `Split("a,b,c", ",")`: Split a string into an array.
- `Join(arr, "-")`: Join an array into a string.
- `Filter(arr, "match")`: Filter an array based on a string.
- `LBound(arr, [dim])`: Get the lower bound.
- `UBound(arr, [dim])`: Get the upper bound.

## Option Strict

`Option Strict On` rejects two things a program usually did not mean:

```vb
Option Strict On

Dim rate As Double = 3          ' fine: nothing is lost widening
Dim rounded As Integer = 3.9    ' rejected: use CInt(3.9)
Dim parsed As Long = "7"        ' rejected: use CLng("7")
```

- **Conversions that can lose something**: a narrower number, a string
  becoming a number or the reverse, or anything out of a `Variant`. Widening is
  untouched, since nothing is gained by spelling out a conversion that cannot
  fail. `CInt`, `CLng`, `CStr`, `CType`, and the rest say it explicitly.
- **Late binding**: reaching a member of a `Variant` or an `Object`, where
  nothing at that point can say whether the member exists. `CType` says what
  the value is and brings the member back.

It is off by default, which is what VBA source expects. `Option Strict Off`
says so explicitly. `Option Explicit` accepts `On` and `Off` the same way, and
means on when neither is written.

`Option Strict` is per module: a file that turns it on is not affected by one
that does not.
