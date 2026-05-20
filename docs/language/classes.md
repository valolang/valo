# Classes and Objects

Valo features a robust object-oriented system that combines the familiarity of Basic with modern language features.

## Class Definition

Classes are defined using the `Class` keyword and ended with `End Class`.

```vb
Public Class User
    Private mName As String

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value
    End Property
End Class
```

## Lifecycle

Valo supports both modern and legacy lifecycle methods.

### Native Lifecycle
The preferred way to handle object initialization and cleanup is using `Constructor` and `Terminate`.

```vb
Class Connection
    Public Constructor(ByVal host As String)
        ' Setup logic
    End Constructor

    Public Terminate()
        ' Cleanup logic
    End Terminate
End Class
```

### VBA Compatibility Aliases
For compatibility with existing VBA code, Valo also recognizes `Class_Initialize` and `Class_Terminate`.

```vb
Class Legacy
    Private Sub Class_Initialize()
        ' Runs when New Legacy is called
    End Sub

    Private Sub Class_Terminate()
        ' Runs when the object goes out of scope
    End Sub
End Class
```

## Default Members

Valo allows classes to have a single default member, enabling objects to be used as if they were their default property.

### Native Syntax
Use the `Default` keyword on a `Property Get`.

```vb
Class Box
    Private mValue As Integer

    Public Default Property Get Value() As Integer
        Return Me.mValue
    End Property
End Class

Sub Main()
    Dim b As New Box
    ' Implicitly calls b.Value
    Console.WriteLine(b)
End Sub
```

### Indexers
Default properties can take arguments, allowing for ergonomic "indexer" style access.

```vb
Class List
    Public Default Property Get Item(ByVal index As Integer) As String
        ' Return item at index
    End Property
End Class

Sub Main()
    Dim l As New List
    Console.WriteLine(l(0)) ' Calls l.Item(0)
End Sub
```

## Events

Classes can declare events that other objects or modules can handle.

```vb
Class Timer
    Public Event Tick(ByVal seconds As Integer)

    Public Sub Run()
        RaiseEvent Tick(1)
    End Sub
End Class
```

## Visibility

Valo supports `Public` and `Private` visibility for fields, methods, and properties.
*   **Public:** Accessible from anywhere.
*   **Private:** Accessible only within the declaring class.
