# Architecture Overview

Valo's architecture is divided into three primary layers to ensure modularity, portability, and future-readiness.

## 1. Frontend
The Frontend is responsible for transforming raw source code into a validated, semantically-rich Abstract Syntax Tree (AST). It is entirely independent of how the code is eventually executed.

*   **[Preprocessor](parser.md):** Handles conditional compilation (`#If`) and source transformations.
*   **Lexer:** Tokenizes source code into discrete elements.
*   **Parser:** Performs recursive descent to build the AST.
*   **AST:** The structural representation of Valo code.
*   **Semantics:** Validates symbols, types, and control flow.
*   **[Module System](modules.md):** Discovers and resolves module dependencies.

Learn more in **[Frontend Architecture](frontend.md)**.

## 2. Runtime
The Runtime defines the core data model and behavior of the Valo language. It is shared by both the Frontend (for type-checking) and all Backends (for execution).

*   **Value System:** Defines `Value`, `ObjectValue`, and type coercion rules.
*   **Diagnostics:** Provides source-aware error reporting.
*   **Builtins:** Standard library functions (Console, Math, Strings).
*   **Resource Model:** Manages deterministic cleanup via `Using` and `Dispose`.

Learn more in **[Runtime Architecture](runtime.md)**.

## 3. Backend
The Backend is the execution engine that consumes the validated AST (or future intermediate representations) and performs the actual work.

*   **Tree-walking Interpreter:** The current reference execution engine.
*   **Future VM:** A planned bytecode virtual machine for higher performance.
*   **Future Backends:** Potential WASM and Native compilation targets.

Learn more in **[Backend Architecture](backend.md)**.
