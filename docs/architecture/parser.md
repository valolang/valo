# Parser Architecture

The Valo parser is a hand-written recursive descent parser designed to handle the nuances of the Basic language while enforcing strict semantic rules.

## Design Goals

1.  **High Fidelity:** Accurately represent the structure of Basic code, including legacy quirks like line-oriented statements and case-insensitivity.
2.  **Explicit Diagnostics:** Provide clear, localized error messages when parsing fails, using a custom `Diagnostic` system.
3.  **Modern Extensions:** Support modern features like `Constructor`, `Default Property`, and structured imports alongside traditional Basic syntax.

## Parsing Pipeline

1.  **Lexer (`core/src/lexer/`):** Scans the source string into a sequence of `Token`s. It handles Basic-specific rules like single-quote comments and case-insensitive keywords.
2.  **Preprocessor (`core/src/preprocessor.rs`):** Handles conditional compilation directives (`#If`, `#Else`, `#End If`, `#Const`). It operates on the token stream to produce a filtered stream for the parser.
3.  **Parser (`core/src/parser/`):**
    *   **`mod.rs`:** Defines the central `Parser` struct and common utilities.
    *   **`declarations.rs`:** Parses top-level declarations (subs, functions, classes, enums, types, imports).
    *   **`statements.rs`:** Parses statements inside procedure and property bodies (assignments, loops, if-blocks, error handling).
    *   **`expressions.rs`:** Parses expressions using operator precedence climbing.
    *   **`program.rs`:** The entry point for parsing a complete source file into a `Program` AST.

## Error Recovery

When a parse error occurs, the parser emits a `Diagnostic` and attempts to synchronize by skipping tokens until the next statement boundary (e.g., a newline or specific keyword). This allows it to report multiple errors in a single pass.

## AST Structure (`core/src/ast/`)

The Abstract Syntax Tree (AST) is defined as a series of Rust `enum`s and `struct`s:
*   `Program`: The top-level container for a module.
*   `Decl`: Represents declarations like `Function`, `Sub`, `Class`, etc.
*   `Stmt`: Represents executable statements.
*   `Expr`: Represents computable expressions.
