# Error Handling

Valo implements the classic Basic error handling model while integrating it with modern runtime diagnostics.

## Try/Catch/Finally

For modern `.valo` code, Valo supports a structured `Try/Catch/Finally` mechanism. This is the recommended approach for new development.

```vb
Try
    ' Code that might fail
    DangerousOperation()
Catch ex As Error
    ' Handle the error
    Console.WriteLine("Error " & ex.Number & ": " & ex.Message)
Finally
    ' Cleanup code (always runs)
    CloseResources()
End Try
```

### Rules
- `Try` must be followed by either `Catch`, `Finally`, or both.
- `Catch` variables are optional and must be of type `Error`.
- `Finally` blocks are guaranteed to execute even if the `Try` block returns or exits.
- `Try/Catch` handles runtime errors only; semantic errors are caught before execution.

## The Error Object

In a `Catch` block, the error object exposes several properties:
- `Number`: Error number.
- `Message`: Error message.
- `Description`: Full description (compatibility alias for Message).
- `Source`: Error source module/class.
- `HelpFile`: Path to help file.
- `HelpContext`: Help context ID.

## On Error Statement (VBA Compatibility)

### On Error GoTo <label>
Redirects control to a specific label when an error occurs.

```vb
Sub Main()
    On Error GoTo Handler
    Dim x As Integer = 1 / 0
    Exit Sub

Handler:
    Console.WriteLine("An error occurred: " & Err.Description)
    Resume Next
End Sub
```

### On Error Resume Next
Instructs the runtime to ignore any errors and continue execution with the next statement.

```vb
On Error Resume Next
Dim value As Integer = GetValue() ' If this fails, execution continues
```

## The Err Object

The `Err` object provides information about the most recent error.

| Property/Method | Description |
|-----------------|-------------|
| `Number` | The unique numeric identifier for the error. |
| `Description` | A human-readable description of the error. |
| `Source` | The name of the module or class where the error originated. |
| `HelpFile` | (Optional) Path to a help file. |
| `HelpContext` | (Optional) Help context ID. |
| `Clear()` | Resets all properties of the `Err` object to zero or empty strings. |
| `Raise(num, ...)` | Programmatically triggers a runtime error. |

## Resuming Execution

The `Resume` statement is used inside an error handler to specify where control should return.

*   **`Resume`:** Retries the statement that caused the error.
*   **`Resume Next`:** Continues with the statement immediately following the one that caused the error.
*   **`Resume <label>`:** Jumps to a specific label.

## Semantic Validation

Unlike traditional VBA, Valo performs extensive semantic validation *before* execution. This means that many "errors" (like syntax errors, undeclared variables, or type mismatches) are caught at compilation time and will prevent the program from running, rather than triggering the `On Error` handler at runtime.
