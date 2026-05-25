# Generics

Valo supports first-class, VB.NET-style generic type parameters on classes, structures, functions, and methods.

```vb
Class Box(Of T)
    Public Value As T
End Class

Structure Pair(Of A, B)
    Public Left As A
    Public Right As B
End Structure

Function Identity(Of T)(ByVal value As T) As T
    Identity = value
End Function
```

Generic instances are semantic types, not `Variant` aliases. `Box(Of String)` and `Box(Of Long)` have different type identities, and member types are substituted through fields, parameters, properties, return values, arrays, and nested generic arguments.

```vb
Dim x As Box(Of String)
Set x = New Box(Of String)()

x.Value = "hello"
' x.Value = 123  ' type mismatch
```

Nested generic type names are supported anywhere a type name is accepted:

```vb
Dim nested As Box(Of Box(Of String))
Dim pair As Pair(Of String, Long)
```

Generic functions use explicit type arguments:

```vb
Dim text As String
text = Identity(Of String)("hello")
```

The runtime keeps instantiated generic metadata and caches concrete class/structure layouts using the formatted type identity, such as `Box(Of String)`. This keeps the current interpreter VM-ready: future bytecode can reference canonical generic definitions plus concrete type arguments instead of relying on textual rewrites.

Generic classes can be used as base classes:

```vb
Class Repository(Of T)
    Public Current As T
End Class

Class UserRepository Inherits Repository(Of User)
End Class
```

Inherited generic members keep their instantiated type arguments in the runtime layout.

Valo accepts VB.NET-style variance markers and type parameter constraint syntax in generic
declarations. `Class`, `Structure`, `New`, and base-class constraints are checked when a
generic type or function is instantiated. Interface constraint matching is reserved for deeper
interface runtime work.

```vb
Interface IProducer(Of Out T)
End Interface

Interface IConsumer(Of In T)
End Interface

Class List(Of T As IDisposable)
End Class

Function Create(Of T)() As T Where T : Class, New
```

Generic method type inference, generic delegates, lambdas, overload resolution parity, and
LINQ-style collection APIs are still roadmap items.
