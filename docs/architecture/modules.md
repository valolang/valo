# Module System Implementation

The Valo module system provides a structured way to handle multi-file projects, with explicit imports and case-insensitive symbol resolution.

## Module Loading Pipeline

1.  **Discovery:** When an `Import ModuleName` is encountered, the module loader (`core/src/frontend/modules.rs`) searches for corresponding `.valo` or `index.valo` files.
2.  **Preprocessing & Parsing:** Each discovered file is preprocessed and parsed into a `Program` AST.
3.  **Recursive Loading:** The loader recursively parses all modules imported by the new module, building a dependency graph.
4.  **Cycle Detection:** The loader detects and reports circular imports to prevent infinite recursion and clarify project structure.

## Semantic Resolution

After all modules are parsed, the semantic validator processes the entire project:

1.  **Global Symbol Table:** Collects all public members from all modules.
2.  **Import Binding:** For each module, it maps imported names to their corresponding symbols in the global table.
3.  **Visibility Enforcement:** Ensures that `Private` members are not accessed outside their declaring module.


No compatibility mode or implicit sibling import group exists. The entrypoint is a Main sub in a .valo module.
