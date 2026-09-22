# Backend Architecture

The Backend is responsible for executing the validated code. Valo is designed to support multiple execution strategies.

## Tree-walking Interpreter (`core/src/backend/interpreter/`)

The current default backend is an AST-based tree-walking interpreter.

*   **`interpreter.rs`:** The main engine.
*   **`eval_expr.rs`:** Recursive evaluation of expressions.
*   **`exec_stmt.rs`:** Execution of statements and control flow.
*   **`frame.rs`:** Call stack and local variable management.
*   **`calls.rs`:** Procedure, function, and method dispatch.
*   **`builtins/`:** Basic runtime intrinsics (legacy-only helpers remain migration debt) for strings, arrays, math, console/debug output, error state, and type helpers.
*   **`file_io.rs`:** Transitional file-number I/O and local filesystem helpers.
*   **`ffi.rs`:** Native `Declare` dispatch, pointer handling, callback support, and diagnostics for unsupported native boundaries.

### Advantages
*   High fidelity to the source structure.
*   Easier to implement complex features like `On Error Resume Next`.
*   Direct access to AST metadata for debugging.

### Disadvantages
*   Higher overhead compared to bytecode or native code.
*   Recursive evaluation can hit stack limits on very deep expressions.

## Native backend preparation

The next compiler milestones are resolved typed HIR, ownership analysis and typed control-flow IR. A native backend will consume that verified representation. A VM or JIT may be added as a separate consumer; bytecode is not a mandatory prerequisite. See the [migration plan](native-migration.md).
