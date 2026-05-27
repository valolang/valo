# COM Automation

Valo supports COM (Component Object Model) automation on Windows, allowing you to control external applications like Microsoft Office, Scripting.FileSystemObject, and more, just like you would in VBA.

## CreateObject

Use `CreateObject(progId)` to create a new instance of a COM object.

```vb
Dim fso As Object
Set fso = CreateObject("Scripting.FileSystemObject")
```

## Late Binding

COM objects in Valo are late-bound. This means you can call methods and access properties that are not known at compile time.

```vb
Dim pptApp As Object
Set pptApp = CreateObject("PowerPoint.Application")
pptApp.Visible = True
```

## Chained Member Access

Valo supports chained member access and method calls, which is common in COM automation.

```vb
slide.Shapes.Title.TextFrame.TextRange.Text = "Hello from Valo"
```

## Collection Enumeration

You can use `For Each` to iterate over COM collections.

```vb
Dim folder As Object
Set folder = fso.GetFolder("C:\Windows")

For Each subfolder In folder.SubFolders
    Debug.Print subfolder.Path
Next
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

## Requirements

- **Operating System**: Windows (COM is not supported on Linux or macOS).
- **Libraries**: Automation objects must be registered on the system.
