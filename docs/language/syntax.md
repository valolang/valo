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

Valo is statically typed with an optional `Variant` type for flexibility.

*   `Integer`: 64-bit signed integer.
*   `Double`: 64-bit floating point number.
*   `String`: UTF-8 encoded string.
*   `Boolean`: `True` or `False`.
*   `Variant`: A flexible type that can hold any value.
*   User-defined `Type` and `Enum`.

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
```

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
