# Properties

Properties use VB.NET-style declarations, auto properties, or Get/Set blocks.
Standalone `Property Get`, `Property Let` and `Property Set` declarations are
rejected. One setter representation serves both value and reference types.

```vb
Public Property Health As Integer = 100

Public Property Value As Integer
    Get
        Return Stored
    End Get
    Set(NewValue As Integer)
        Stored = NewValue
    End Set
End Property
```

ReadOnly and WriteOnly restrict accessors. Access modifiers, Shared, Default,
overrides and overloaded indexed properties remain supported. Generic property
types use the enclosing type's generic parameters.

## Indexed properties

```vb
Class Store
    Private Slots(9) As String

    Public Default Property Item(Index As Integer) As String
        Get
            Return Slots(Index)
        End Get
        Set(Value As String)
            Slots(Index) = Value
        End Set
    End Property
End Class
```

`Store.Item(2)` reads through Get; `Store.Item(2) = "two"` invokes Set. A setter's
value follows the index parameters internally. Overloads are selected from the
index and value types; duplicate signatures are rejected.

Interfaces declare the property name and type without bodies:

```vb
Interface ICounter
    Property Value As Integer
End Interface

Class Counter
    Implements ICounter
    Public Property Value As Integer Implements ICounter.Value
End Class
```

Named modules can contain properties, accessed through their module name.
Exported `Attribute VB_UserMemId` metadata is not supported; use `Default` to
select a default property. Modern angle-bracket attributes remain available.
