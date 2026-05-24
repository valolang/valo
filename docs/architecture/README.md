# Architecture Overview

Valo's architecture is divided into three primary layers to ensure modularity, portability, and future-readiness.

## 1. Frontend (`core/src/frontend/`)
The Frontend is responsible for transforming raw source code into a validated, semantically-rich Abstract Syntax Tree (AST). It is entirely independent of how the code is eventually executed.

*   **Preprocessor:** Handles conditional compilation (`#If`) and source transformations.
*   **Lexer:** Tokenizes source code into discrete elements.
*   **Parser:** Performs recursive descent to build the AST.
*   **AST:** The structural representation of Valo code.
*   **Semantics:** Validates symbols, types, and control flow.
*   **Module System:** Discovers and resolves module dependencies.

Learn more in **[Frontend Architecture](frontend.md)**.

## 2. Runtime (`core/src/runtime/`)
The Runtime defines the core data model and behavior of the Valo language. It is shared by both the Frontend (for type-checking) and all Backends (for execution).

*   **Value System:** Defines `Value`, `ObjectValue`, and type coercion rules.
*   **Diagnostics:** Provides source-aware error reporting.
*   **Operations:** Centralized logic for arithmetic, comparison, and coercion.
*   **Resource Model:** Manages deterministic cleanup via `Using` and `Dispose`.

Learn more in **[Runtime Architecture](runtime.md)**.

## 3. Backend (`core/src/backend/`)
The Backend is the execution engine that consumes the validated AST (or future intermediate representations) and performs the actual work.

*   **Interpreter:** The current reference execution engine (tree-walking).
*   **Future VM:** A planned bytecode virtual machine for higher performance.
*   **Future Backends:** Potential WASM and Native compilation targets.

Learn more in **[Backend Architecture](backend.md)**.

## 4. Platform
The Platform layer is the emerging project/package, namespace, tooling, standard-library, and interop architecture that turns Valo from a single-file language runtime into an ecosystem.

*   **Package Identity:** `valo.toml` project roots and entrypoint resolution.
*   **Semantic IDs/HIR:** Stable project-wide identities for tooling and future VM lowering.
*   **Namespaces:** Logical API identity decoupled from filenames.
*   **Runtime Services:** Standard-library boundaries shared by interpreter, VM, and embedders.
*   **Interop:** COM/type-library architecture layered beside native FFI.

Learn more in **[Platform Architecture](platform.md)**.
