# Frontend Architecture

The Valo Frontend is a multi-stage pipeline that ensures source code is syntactically correct and semantically valid before it reaches the execution layer.

## Pipeline Stages

1.  **Preprocessor (`core/src/frontend/preprocessor.rs`):**
    *   Handles `#If...Then...#Else` directives.
    *   Manages conditional compilation constants.
    *   Operates at the token level (conceptually) or line level before the lexer.

2.  **Lexer (`core/src/frontend/lexer/`):**
    *   Converts source text into a stream of `Token`s.
    *   Handles Basic-specific syntax like type characters (`$`, `%`, `&`, `!`, `#`, `@`).
    *   Identifies keywords, literals, and operators.

3.  **Parser (`core/src/frontend/parser/`):**
    *   A hand-written recursive descent parser.
    *   Builds a high-fidelity Abstract Syntax Tree (AST) in `core/src/frontend/ast/`.
    *   Preserves modern VB.NET syntax; exported VBA metadata and standalone legacy property declarations are rejected.

4.  **Semantic Validator (`core/src/frontend/semantics/`):**
    *   Performs symbol resolution (identifying variables, subs, functions).
    *   Enforces type safety.
    *   Validates control flow (e.g., `Exit For` must be inside a `For` loop).
    *   Validates source programs and exposes typed scalar function bodies through `lower_function_body`; the module loader owns the Project structure.

5.  **Package and Module Loader (`core/src/frontend/package.rs`, `core/src/frontend/modules.rs`):**
    *   Discovers `valo.toml` when present and resolves the package entrypoint.
    *   Finds `.valo` files on disk.
    *   Decodes UTF-8 and BOM-marked UTF-16 LE/BE.
    *   Resolves `Import` statements with case-insensitive lookup and `.valo` and `index.valo` candidates.
    *   Ensures unique module names and handles circular dependencies.
    *   Manages the `SourceMap`, assigning a unique `FileId` to each loaded module for accurate diagnostics.

## Key Design Principles

*   **Independence:** The Frontend should not know about the Interpreter or VM. It only knows how to build a valid representation of the code.
*   **Diagnostic-First:** Every stage is designed to produce high-quality diagnostics with accurate source mapping.
*   **Case Insensitivity:** The Frontend handles Valo's case-insensitive nature by normalizing keys (usually via a `key()` helper) for symbol lookups.
*   **Typed bodies:** `typed_hir.rs` records resolved locals, parameter storage, arithmetic signatures, conversions and selected function calls. Lowering currently supports straight-line scalar functions and rejects unsupported constructs explicitly. The source interpreter retains the broader language surface.
*   **Shared arithmetic:** `semantics/arithmetic.rs` selects primitive operation types for validation, typed HIR and interpreter execution.
*   **Native interoperability:** Modern `Declare` remains part of the frontend; exported Office project metadata does not.
