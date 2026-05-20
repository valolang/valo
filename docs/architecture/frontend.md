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
    *   Supports both modern Valo syntax and VBA-style declarations (Attributes, `Type`, etc.).

4.  **Semantic Validator (`core/src/frontend/semantics/`):**
    *   Performs symbol resolution (identifying variables, subs, functions).
    *   Enforces type safety.
    *   Validates control flow (e.g., `Exit For` must be inside a `For` loop).
    *   Produces a "Project" structure that is ready for execution.

5.  **Module Loader (`core/src/frontend/modules.rs`):**
    *   Finds `.valo`, `.bas`, and `.cls` files on disk.
    *   Resolves `Import` statements.
    *   Ensures unique module names and handles circular dependencies.

## Key Design Principles

*   **Independence:** The Frontend should not know about the Interpreter or VM. It only knows how to build a valid representation of the code.
*   **Diagnostic-First:** Every stage is designed to produce high-quality diagnostics with accurate source mapping.
*   **Case Insensitivity:** The Frontend handles Valo's case-insensitive nature by normalizing keys (usually via a `key()` helper) for symbol lookups.
