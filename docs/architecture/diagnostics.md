# Diagnostics System

Valo prioritizes developer experience through a professional diagnostics system that provides clear, actionable feedback for syntax errors, semantic violations, and runtime failures.

## Diagnostic Structure

Every error or warning in Valo is encapsulated in a `Diagnostic` struct (`core/src/runtime/diagnostic.rs`), which includes:

1.  **Unique Code:** A searchable identifier (e.g., `V1100`) for specific error types.
2.  **Primary Message:** A concise summary of the issue.
3.  **Span:** The exact location (line, column) in the source code where the issue was detected.
4.  **Labels:** Secondary annotations that point to related code segments or provide more context.
5.  **Notes and Help:** Additional information or suggestions for how to fix the issue.

## Rendering

Diagnostics are rendered in a format inspired by Rust and Zig, optimized for readability in terminal environments.

```txt
error[V1100]: Cannot assign String value to Integer variable
  --> script.valo:3:3
   |
3 |   x = "string"
   |   ^^^^^^^^^^^^ expected Integer, found String
   |
help: change the variable type or assign a value with the expected type
```

## Phases

Valo emits diagnostics during three distinct phases:

1.  **Parse Phase:** Reports syntax errors like missing keywords, unbalanced parentheses, or malformed constructs.
2.  **Semantic Phase:** Reports logical errors like type mismatches, unknown variables, or invalid module imports.
3.  **Runtime Phase:** Reports execution-time failures like division by zero or out-of-bounds array access.

## Implementation Details

The `Diagnostic` system is designed to be cumulative during the parse and semantic phases. The `Parser` and `Validator` can continue processing after an error is found, collecting multiple diagnostics to report to the user at once.
