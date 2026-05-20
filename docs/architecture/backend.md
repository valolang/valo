# Backend Architecture

The Backend is responsible for executing the validated code. Valo is designed to support multiple execution strategies.

## Tree-walking Interpreter (`core/src/interpreter/`)

The current default backend is an AST-based tree-walking interpreter.

*   **`interpreter.rs`:** The main engine.
*   **`eval_expr.rs`:** Recursive evaluation of expressions.
*   **`exec_stmt.rs`:** Execution of statements and control flow.
*   **`frame.rs`:** Call stack and local variable management.
*   **`calls.rs`:** Procedure, function, and method dispatch.

### Advantages
*   High fidelity to the source structure.
*   Easier to implement complex features like `On Error Resume Next`.
*   Direct access to AST metadata for debugging.

### Disadvantages
*   Higher overhead compared to bytecode or native code.
*   Recursive evaluation can hit stack limits on very deep expressions.

## Future: Bytecode Virtual Machine (VM)

The planned next step for Valo is a custom bytecode VM.

1.  **Compiler:** Translates AST into linear bytecode.
2.  **Bytecode IR:** A stable instruction set representing Valo operations.
3.  **VM Engine:** A high-performance loop that executes instructions using a stack or registers.

The migration to a VM will require moving more value-operation logic (like `eval_binary`) into the **Runtime** layer so it can be shared.

## Future: Compilation Targets

Valo's separation of Frontend and Runtime paves the way for alternative backends:
*   **WASM:** Compiling Valo to WebAssembly for execution in the browser.
*   **Native:** Using LLVM or a similar toolchain to produce standalone native binaries.
*   **FFI:** Integrating with native libraries will be a backend-specific feature that maps Valo objects to C-compatible resources.
