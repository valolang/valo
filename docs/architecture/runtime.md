# Runtime and Interpreter

Valo currently uses a high-fidelity tree-walking interpreter to execute the validated AST. The runtime is designed to be safe, portable, and eventually transition to a bytecode virtual machine.

## Interpreter Structure (`core/src/interpreter/`)

The interpreter consists of several modules handling different aspects of execution:

1.  **`interpreter.rs`:** The central engine that coordinates execution, manages module state, and maintains the call stack.
2.  **`eval_expr.rs`:** Evaluates AST expressions into runtime `Value`s.
3.  **`exec_stmt.rs`:** Executes AST statements and manages control flow (loops, branches, returns).
4.  **`frame.rs`:** Defines the execution `Frame`, which holds local variables and parameter bindings for a single procedure call.
5.  **`values.rs`:** Defines the runtime `Value` enum, representing all possible data types in Valo (Integer, String, Object, etc.).
6.  **`objects.rs`:** Implements the native object model, handling class instances, field storage, and method dispatch.
7.  **`control_flow.rs`:** Defines signals for structured control flow, such as `Exit Sub`, `GoTo`, and `Return`.

## Memory Management

Valo uses reference counting (via Rust's `Rc` and `RefCell`) to manage object lifetimes. This provides predictable cleanup and supports the `Terminate` / `Class_Terminate` events when an object's reference count drops to zero.

## Semantic Validation (`core/src/semantics/`)

Before execution, every project undergoes a semantic validation pass. This pass ensures that:
*   All referenced symbols (variables, subs, modules) exist and are accessible.
*   Types are consistent across assignments and calls.
*   Control flow is valid (e.g., no `Exit For` outside of a loop).
*   Module dependencies are correctly resolved.

## The `Value` System

The `Value` enum is the core of the runtime:
*   `Integer(i64)`
*   `Double(f64)`
*   `String(String)`
*   `Boolean(bool)`
*   `Object(Rc<RefCell<RuntimeClass>>)`
*   `Array { elements: Vec<Cell<Value>>, ... }`
*   `Variant` (can wrap any of the above)
*   `Nothing`, `Empty`, `Null`
