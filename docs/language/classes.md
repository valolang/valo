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

## Inheritance

Classes can inherit one base class with `Inherits`. Inherited fields, methods, properties, and Shared members participate in normal member lookup.

```vb
Class Animal
    Public Overridable Sub Speak()
        Console.WriteLine("...")
    End Sub
End Class

Class Dog Inherits Animal
    Public Overrides Sub Speak()
        Console.WriteLine("Woof")
    End Sub
End Class
```

`MustInherit` classes cannot be constructed directly. `MustOverride` members declare required behavior for concrete derived classes. `NotInheritable` prevents further inheritance.

Use `Protected` for members intended for derived classes, and `Protected Friend` for members visible to derived classes and the current project/module boundary. `Shadows` declares intentional hiding; `Overrides` requires a matching overridable base member.

`MyBase.Member()` calls the base implementation directly. `MyClass.Member()` is parsed as a current-class dispatch form for source compatibility.

## Lifecycle And Cleanup

Valo supports both modern and legacy lifecycle methods.

### Construction
The native constructor form is `Sub New`.

```vb
Class Connection
    Public Sub New(ByVal host As String)
        ' Setup logic
    End Sub

    Public Sub Terminate()
        ' Cleanup logic
    End Sub
End Class
```

### Deterministic Cleanup
Use `Sub Dispose` for explicit resource cleanup. A `Using` block calls `Dispose` automatically when the block exits, including exits caused by `Return`, `Exit Sub`, or a runtime error.

```vb
Class Resource
    Public Sub Dispose()
        Console.WriteLine("disposed")
    End Sub
End Class

Sub Main()
    Using res As New Resource()
        Console.WriteLine("inside")
    End Using
End Sub
```

`Dispose` must be parameterless when used by `Using`. Manual calls such as `res.Dispose()` are normal method calls.

### Lifecycle Hooks
`Sub Terminate` remains a lifecycle hook that runs when an object is released by the runtime. Prefer `Dispose` and `Using` for resources, especially code that will later interact with FFI or external handles.

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

VBA-exported classes may mark the default property group with `Attribute Item.VB_UserMemId = 0`. Valo groups same-name `Property Get`, `Property Let`, and `Property Set` accessors as one property, so indexed reads, value assignments, and object `Set` assignments all target the same default member.

Class-scope constants are supported:

```vb
Class MathBox
    Private Const Scale As Double = 2
End Class
```

## Properties

Valo supports full property blocks with `Get`/`Let`/`Set` accessors as well as modern auto-implemented properties.

### Auto-implemented Properties
Auto-properties provide a concise way to declare properties that wrap a private field. The compiler automatically generates a hidden backing field.

```vb
Public Class Product
    Public Property Name As String = "Untitled"
    Public Property Price As Double
    Public Shared Property InstanceCount As Integer
End Class
```

Shared auto-properties are shared across all instances of the class.

### Full Property Blocks
For more complex logic, use a full property block:

```vb
Public Class User
    Private mAge As Integer

    Public Property Age As Integer
        Get
            Return mAge
        End Get
        Set(ByVal value As Integer)
            If value >= 0 Then mAge = value
        End Set
    End Property
End Class
```

Valo also supports legacy `Property Get`, `Property Let` (for value types), and `Property Set` (for object types) as standalone members for compatibility.

## Events

Classes can declare events that other objects or modules can handle.

```vb
Class Source
    Public Event Click(ByVal value As Integer)

    Public Sub DoClick(ByVal v As Integer)
        RaiseEvent Click(v)
    End Sub
End Class
```

### Static Event Handling (`WithEvents`)
Use the `WithEvents` modifier on a field to automatically handle events from an object. Event handlers must follow the `FieldName_EventName` naming convention.

```vb
Class Form
    Private WithEvents mSource As Source

    Public Sub New(ByVal src As Source)
        mSource = src
    End Sub

    Private Sub mSource_Click(ByVal v As Integer)
        Console.WriteLine("Clicked: " & v)
    End Sub
End Class
```

### Dynamic Event Handling (`AddHandler`)
Use `AddHandler` and `RemoveHandler` to dynamically bind event handlers at runtime.

```vb
Sub Main()
    Dim src As New Source()
    AddHandler src.Click, AddressOf MyGlobalHandler
    src.DoClick(42)
End Sub

Sub MyGlobalHandler(ByVal v As Integer)
    Console.WriteLine("Dynamic handler: " & v)
End Sub
```

## Collection Initializers

Collections can be populated inline during construction using the `From { ... }` syntax.

```vb
Dim fruits As New Collection() From { "Apple", "Banana", "Orange" }
```

This syntax is equivalent to calling the `Add` method for each item in the list.

## Iterators

Valo supports native iterators inspired by VB.NET. Iterators use the `Iterator` modifier on `Function` or `Property Get` and emit values using the `Yield` statement.

### Iterator Functions

```vb
Class Range
    Public Iterator Function Items(ByVal count As Integer) As Variant
        Dim i As Integer
        For i = 1 To count
            Yield i
        Next i
    End Function
End Class

Sub Main()
    Dim r As New Range
    Dim n As Variant
    For Each n In r.Items(5)
        Console.WriteLine(n)
    Next n
End Sub
```

### Default Object Iteration
If a class has exactly one parameterless `Public Iterator Function` or `Public Iterator Property Get`, it is used as the default enumerator for `For Each` over an instance of that class.

```vb
Class Words
    Public Iterator Function Items() As Variant
        Yield "Valo"
        Yield "is"
        Yield "modern"
    End Function
End Class

Sub Main()
    Dim words As New Words
    Dim item As Variant
    For Each item In words
        Console.WriteLine(item)
    Next item
End Sub
```

Rules for Iterators:
- Must contain at least one `Yield` statement.
- Cannot have `ByRef` parameters.
- `Return` is not allowed; use `Yield` or `Exit Function`.
- In the current implementation, iterators materialize yielded values into an internal array; lazy generators are planned for the future.

### VBA Compatibility
Valo preserves compatibility with VBA-style `_NewEnum` members using `Attribute _NewEnum.VB_UserMemId = -4`.

```vb
Class LegacyList
    Public Property Get _NewEnum() As Variant
    Attribute _NewEnum.VB_UserMemId = -4
        _NewEnum = someArray
    End Property
End Class
```

## Visibility

Valo supports `Public` and `Private` visibility for fields, methods, and properties.
*   **Public:** Accessible from anywhere.
*   **Private:** Accessible only within the declaring class.

## Classes vs Structures

Use `Class` for reference objects with identity, lifecycle hooks, events, `WithEvents`, and `Nothing`.

Use `Structure` for native value types. Structures can have fields, methods, properties, and constructors, but they are copied on assignment and do not support class lifecycle, events, inheritance, interfaces, `Set`, `Nothing`, or `Is` identity checks.
