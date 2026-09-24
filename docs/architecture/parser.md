# Parser Architecture

The Valo parser is a hand-written recursive descent parser designed to handle the nuances of the Basic language while enforcing strict semantic rules.

## Design Goals

1.  **High Fidelity:** Accurately represent the structure of Basic code, including legacy quirks like line-oriented statements and case-insensitivity.
2.  **Explicit Diagnostics:** Provide clear, localized error messages when parsing fails, using a custom `Diagnostic` system.
3.  **Modern Extensions:** Support modern features like class `Sub New`, `Structure`, `Default Property`, and structured imports alongside traditional Basic syntax.

## Parsing Pipeline

1.  **Preprocessor (`core/src/frontend/preprocessor.rs`):** Handles conditional directives and explicit `_` continuation.
2.  **Lexer (`core/src/frontend/lexer/`):** Scans into tokens, including newlines and source spans.
3.  **Parser (`core/src/frontend/parser/`):** Normalizes implicit continuation tokens, then parses declarations, statements and expressions.
    *   **`mod.rs`:** Defines the central `Parser` struct and common utilities.
    *   **`declarations.rs`:** Parses top-level declarations (subs, functions, classes, enums, types, imports).
    *   **`statements.rs`:** Parses statements inside procedure and property bodies (assignments, loops, if-blocks, error handling).
    *   **`expressions.rs`:** Parses expressions using operator precedence climbing.
    *   **`program.rs`:** The entry point for parsing a complete source file into a `Program` AST.

## Error Recovery

When a parse error occurs, the parser emits a `Diagnostic` and attempts to synchronize by skipping tokens until the next statement boundary (e.g., a newline or specific keyword). This allows it to report multiple errors in a single pass.

## AST Structure (`core/src/ast/`)

The Abstract Syntax Tree (AST) is defined as a series of Rust `enum`s and `struct`s:
*   `Program`: The top-level container for one parsed source file.
*   `Decl`: Represents declarations like `Function`, `Sub`, `Class`, etc.
*   `Stmt`: Represents executable statements.
*   `Expr`: Represents computable expressions.

## Implicit line continuation

The parser centrally suppresses a newline when the preceding token requires
more syntax, such as `(`, `{`, `,` inside a delimiter, `.`, `=`, or a binary
operator. A newline before a closing `)` or `}` inside a delimiter also
continues the list. Comments and blank lines are allowed after commas in an
open list. Thus calls, parameter lists, native `Declare` signatures, tuple
literals, generic lists, collection/object initializers, and parenthesized
expressions can span lines without `_`. Explicit `_` remains supported.
An `Implements` type list also continues after its comma.

Newlines between two complete expressions remain significant: `A = 1` and
`B = 2` are separate statements, and `Foo(A` followed by `B)` still needs a
comma. `Return` followed by a bare newline does not implicitly consume the
next line. A leading dot after a complete line is not treated as continuation,
because it is also valid inside a `With` block. Call argument trailing commas
retain the existing omitted-argument meaning; they are not silently ignored.
