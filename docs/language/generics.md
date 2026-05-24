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

Constraint syntax is reserved for a future pass:

```vb
Class List(Of T As IDisposable)
Function Create(Of T As {Class, New})() As T
```
