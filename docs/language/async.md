# Async and Await

Valo accepts VB.NET-style `Async Sub`, `Async Function`, and `Await` syntax.

`Await` is valid only inside an `Async Sub` or `Async Function`, including class methods. The current interpreter evaluates awaited expressions immediately and returns the value, so this is a compatibility syntax and control-flow validation feature rather than a concurrent task scheduler.

```vb
Async Function FetchAsync(ByVal id As Integer) As String
    Return "item-" & id
End Function

Async Sub Main()
    Dim value As String
    value = Await FetchAsync(42)
    Console.WriteLine(value)
End Sub
```

Using `Await` outside an async procedure is rejected during semantic validation. Awaiting a `Sub` as an expression is also rejected; use an async function when a value-producing awaitable expression is needed.
