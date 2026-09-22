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
  --> List.valo:39:24
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

## Codes

A diagnostic's code is its identity. Codes are stable: once released, a code
keeps its meaning, and a retired one is never reused. That is what makes them
safe to search for, to match on in tooling, and to cite in a bug report.

They are declared once, by the `diagnostic_codes!` macro in
[`runtime::diagnostic`](../../core/src/runtime/diagnostic.rs). The macro produces
both the constants the compiler refers to and a table carrying each code's name
and summary, so a code cannot exist without a description.

Three tests hold that arrangement together:

- no two diagnostics share a code, which would otherwise be invisible since
  codes are hand-assigned strings
- every code is well formed and has a summary that reads as a sentence
- [the published reference](../reference/diagnostics.md) lists every code and its
  summary, so the documentation cannot silently fall behind the compiler

`V0001` is the one code with no specific meaning: it marks a diagnostic that has
not been given a better one yet. Its remaining uses are a work list, not a
destination.

### Enforcing a rule in both layers

Some rules are checked twice: once by the analyzer, so a mistake is caught before
the program runs, and again by the interpreter, because a value's shape is not
always known statically. Argument count is the clearest case.

Both now go through `Builtin::check_arity` on the registry entry, so a program is
rejected for the same reason and with the same wording either way. Before, the
analyzer and each of ninety-seven builtin implementations phrased the same
rejection separately, and the interpreter reported it as `V0001`.
