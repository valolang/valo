# Module System Implementation

The Valo module system provides a structured way to handle multi-file projects, with support for both modern explicit imports and legacy VBA-style shared namespaces.

## Module Loading Pipeline

1.  **Discovery:** When an `Import ModuleName` is encountered, the module loader (`core/src/modules.rs`) searches for corresponding `.valo`, `.bas`, or `.cls` files.
2.  **Preprocessing & Parsing:** Each discovered file is preprocessed and parsed into a `Program` AST.
3.  **Recursive Loading:** The loader recursively parses all modules imported by the new module, building a dependency graph.
4.  **Cycle Detection:** The loader detects and reports circular imports to prevent infinite recursion and clarify project structure.

## Semantic Resolution

After all modules are parsed, the semantic validator processes the entire project:

1.  **Global Symbol Table:** Collects all public members from all modules.
2.  **Import Binding:** For each module, it maps imported names to their corresponding symbols in the global table.
3.  **Visibility Enforcement:** Ensures that `Private` members are not accessed outside their declaring module.

## VBA Compatibility Mode

In compatibility mode (handling `.bas` and `.cls` files), the module loader supports legacy behaviors:
*   **Case-Insensitivity:** Module and symbol resolution are case-insensitive, matching VBA expectations.
*   **Attribute Processing:** The loader preserves and interprets `Attribute VB_*` metadata, ensuring that module names and default members are correctly identified from the source files.

## Project Structure

A Valo "Project" is essentially a collection of modules organized in a directory tree. The entry point is typically a `Main` sub in a module designated by the user.
