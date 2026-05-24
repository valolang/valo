# Inheritance

Valo supports modern Basic-style class inheritance while preserving existing VBA-compatible class behavior.

```vb
MustInherit Class Animal
    Public MustOverride Sub Speak()
    End Sub
End Class

Class Dog Inherits Animal
    Public Overrides Sub Speak()
        Console.WriteLine("Woof")
    End Sub
End Class
```

Implemented semantics include inherited instance and Shared members, virtual dispatch through overridden members, `MustInherit`, `MustOverride`, `NotInheritable`, `Protected`, `Protected Friend`, `Shadows`, `MyBase`, `TypeOf ... Is` base-class checks, and generic base class layouts.

Override declarations are validated against the base hierarchy. A concrete class must implement inherited `MustOverride` members, and a class cannot inherit a `NotInheritable` base class.
