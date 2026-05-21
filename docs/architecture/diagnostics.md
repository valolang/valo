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

Diagnostics are rendered in a professional, Rust-inspired format. Features include:

-   **Source-Aware Spans:** Diagnostics correctly point to the original source file, even across imported modules.
-   **Colorized Output:** Color is automatic only when stderr is a capable TTY. `NO_COLOR` disables ANSI, redirected output is plain text, and the CLI supports `--color auto|always|never`.
-   **Contextual Labels:** Primary and secondary labels provide pinpoint accuracy for errors.
-   **Import Chains:** Errors in imported modules include notes explaining the import chain.

```txt
error[V0100]: expected statement after `Then` or newline for block If
  --> List.cls:39:24
   |
39 |     If newCap < 0 Then Err.Raise 5, "List", "Capacity must be >= 0"
   |                        ^^^^^^^^^ expected statement
   |
   = note: while parsing imported module `List`
   = note: imported from main.valo:1:1
```

## Implementation Details

The diagnostics system uses a central `SourceMap` (`core/src/runtime/diagnostic.rs`) to manage file names and contents. Every `Span` contains a `FileId` that resolves back to the `SourceMap`, allowing the renderer to show the correct source line regardless of which file triggered the diagnostic.

On Windows, Valo avoids emitting raw ANSI escapes in unsupported consoles. Modern terminals can use color automatically; legacy or redirected sessions receive plain diagnostics.
