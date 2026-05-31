# COM Automation

Valo supports COM (Component Object Model) automation on Windows. This allows Valo code to control registered automation servers such as `Scripting.Dictionary`, `Scripting.FileSystemObject`, and Microsoft Office applications in the same style as VBA.

COM is Windows-only. On non-Windows hosts, `CreateObject` and `GetObject` report clear runtime diagnostics instead of pretending COM is available.

## CreateObject

Use `CreateObject(progId)` to create a new instance of a COM object.

```vb
Sub Main()
    Dim fso As Object
    Set fso = CreateObject("Scripting.FileSystemObject")

    Console.WriteLine(fso.GetAbsolutePathName("."))
End Sub
```

`GetObject(pathname, class)` is also available for attaching to existing automation objects where the host supports it.

## Late Binding

COM objects in Valo are late-bound. This means you can call methods and access properties that are not known at compile time.

```vb
Sub Main()
    Dim pptApp As Object
    Set pptApp = CreateObject("PowerPoint.Application")
    pptApp.Visible = True
End Sub
```

Default properties are supported, so common VBA idioms such as `dict("Name") = "Valo"` work for automation objects that expose a default member.

## Chained Member Access

Valo supports chained member access and method calls, which is common in COM automation.

```vb
Sub Main()
    Dim app As Object
    Set app = CreateObject("PowerPoint.Application")

    app.Visible = True
    app.Presentations.Add
    app.ActivePresentation.Slides.Add(1, 1)
    app.ActiveWindow.View.Slide.Shapes.Title.TextFrame.TextRange.Text = "Hello from Valo"
End Sub
```

## Collection Enumeration

You can use `For Each` to iterate over COM collections.

```vb
Sub Main()
    Dim fso As Object
    Set fso = CreateObject("Scripting.FileSystemObject")

    Dim folder As Object
    Set folder = fso.GetFolder("C:\Windows")

    For Each subfolder In folder.SubFolders
        Debug.Print subfolder.Path
    Next
End Sub
```

## Returned Objects

When a COM method or property returns an object, Valo automatically wraps it as a COM object, preserving its automation capabilities.

## Type Compatibility

Valo marshals types between its runtime and COM VARIANTs:

| Valo Type | COM VARIANT Type |
|-----------|------------------|
| String    | VT_BSTR          |
| Integer   | VT_I2            |
| Long      | VT_I4            |
| Double    | VT_R8            |
| Boolean   | VT_BOOL          |
| Date      | VT_DATE          |
| Object    | VT_DISPATCH      |
| Nothing   | VT_DISPATCH (null)|
| Null      | VT_NULL          |

The COM bridge is late-bound and pragmatic. Type-library import, early binding, and complete Office object-model metadata are future tooling directions, not requirements for using late-bound automation today.

## Requirements

- **Operating System**: Windows (COM is not supported on Linux or macOS).
- **Libraries**: Automation objects must be registered on the system.
