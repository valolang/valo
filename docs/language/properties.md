# Properties

Properties use VBA-compatible `Property Get`, `Property Let`, and `Property Set` procedures.

```vb
Property Let Name(value As String)
    m_Name = value
End Property
```

The final value parameter for `Property Let` and `Property Set` may omit `ByVal`, matching common VBA and exported `.cls` code. Explicit `ByVal` and `ByRef` remain accepted.
