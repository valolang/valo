# Classes and Objects

Valo features a robust object-oriented system that combines the familiarity of Basic with modern language features.

## Class Definition

Classes are defined using the `Class` keyword and ended with `End Class`.

```vb
Public Class User
    Private mName As String

    Public Property Name() As String
        Get
        Return Me.mName
        End Get
        Set(ByVal value As String)
        Me.mName = value
        End Set
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

Exported `Attribute VB_*` declarations and `_NewEnum` metadata are rejected.
Use `Iterator`/`Yield` for enumeration and `Default` for default properties.

## Visibility

Valo supports `Public` and `Private` visibility for fields, methods, and properties.
*   **Public:** Accessible from anywhere.
*   **Private:** Accessible only within the declaring class.

## Classes vs Structures

Use `Class` for reference objects with identity, lifecycle hooks, events, `WithEvents`, and `Nothing`.

Use `Structure` for native value types. Structures can have fields, methods, properties, and constructors, but they are copied on assignment and do not support class lifecycle, events, inheritance, interfaces, `Set`, `Nothing`, or `Is` identity checks.

## Object Initializers

`New T With { .Member = value, ... }` constructs an object and then assigns the
listed members, so a constructor runs first and the initializer can build on
what it set.

```vb
Dim origin As Point = New Point With { .X = 0, .Y = 0 }
```

The `As New` form takes an initializer too:

```vb
Dim corner As New Point With { .X = 1920, .Y = 1080 }
```

Entries may span lines, and a trailing comma is allowed:

```vb
Dim item As Product = New Product With {
    .Name = "keyboard",
    .Price = 49.9,
    .Stock = 12,
}
```

Each entry is written through the same path as an ordinary member assignment, so
assigning to a property runs its `Set` accessor rather than bypassing it.

Validation rejects a member the class does not declare, a value that does not fit
the member's type, and the same member initialized twice.

## Related

- [Example: object initializers](../../examples/object_initializers.valo)
- [Lambdas](functions.md#lambdas)

## Null-Conditional Access

`?.` reads a member only when the receiver is not `Nothing`. When it is, the
whole expression yields `Nothing` instead of failing.

```vb
Console.WriteLine(customer.Home?.City)
```

It guards method calls as well:

```vb
Dim label As Variant = customer.Home?.Label()
```

The guard covers the rest of the chain, not just the member immediately after
it. In `customer?.Home.City`, a `Nothing` customer makes the whole expression
`Nothing`; `.Home` and `.City` are never evaluated.

Because a guarded access can always answer `Nothing`, its result is nullable.
That is what lets `Is Nothing` test it:

```vb
If customer.Home?.City Is Nothing Then
    Console.WriteLine("no address on file")
End If
```

A guarded access is a value, not a storage location, so it cannot be assigned
to. Check the receiver first and assign through `.`:

```vb
customer.Home?.City = "London"      ' rejected

If customer.Home IsNot Nothing Then
    customer.Home.City = "London"
End If
```
