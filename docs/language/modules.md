# Modules and Imports

Valo uses a structured module system to organize code and manage dependencies between files.

## Module Structure

In Valo, every `.valo`, `.bas`, or `.cls` file is treated as a module. The name of the module is derived from the file name (case-insensitively).

## Importing Modules

Use the `Import` keyword to make the public members of another module accessible.

```vb
Import Math

Sub Main()
    ' Use a member from the Math module
    Dim result As Double = Math.Sqrt(16)
End Sub
```

### Import Aliasing
You can provide an alias for an imported module using the `As` keyword. This is useful for resolving name conflicts or shortening long module names.

```vb
Import Models.State As S

Sub Main()
    Console.WriteLine(S.CurrentUser)
End Sub
```

## Visibility and Scoping

Module-level variables, constants, subs, and functions can be marked as `Public` or `Private`.

*   **Public (default):** Accessible from other modules that import this module.
*   **Private:** Accessible only within the module where they are declared.

```vb
' In Utils.valo
Private Const SECRET = "1234"
Public Sub Log(ByVal msg As String)
    ' This is accessible
End Sub
```

## Module Resolution

When you `Import ModuleName`, Valo searches for a file named `ModuleName.valo`, `ModuleName.bas`, or `ModuleName.cls` in the same directory as the importing module.

Modules can also be organized into subdirectories, and imported using dot-notation:

```vb
Import MyLib.Parser
```

This will look for `MyLib/Parser.valo` (or `.bas`/`.cls`) relative to the current file.
