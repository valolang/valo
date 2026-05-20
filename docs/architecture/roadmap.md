# Valo Roadmap

Valo is an evolving project aimed at providing the best standalone Basic experience. This document outlines our future direction and planned features.

## Phase 1: Core Foundation (Current)
*   [x] Lexer & Recursive Descent Parser
*   [x] Semantic Validation & Type Checking
*   [x] Tree-walking Interpreter
*   [x] Basic Object Model (Classes, Properties, Events)
*   [x] VBA Compatibility Layer (.bas/.cls, Attributes)
*   [x] Professional Diagnostics System

## Phase 2: Performance & Stability
*   [ ] **Bytecode VM:** Transition from tree-walking to a stack-based or register-based virtual machine for significant performance gains.
*   [ ] **Comprehensive Collections:** Native support for high-performance `List`, `Dictionary`, and `Queue` types.
*   [ ] **Robust Standard Library:** File I/O, networking, JSON parsing, and string manipulation utilities.

## Phase 3: Developer Experience
*   [ ] **LSP Support:** A Language Server for editor features like autocomplete, go-to-definition, and real-time error reporting.
*   [ ] **Formatting & Linting:** Automated tools to keep Valo codebases clean and consistent.
*   [ ] **Package Manager:** A system for managing and distributing Valo libraries.

## Phase 4: Integration
*   [ ] **FFI (Foreign Function Interface):** Support for calling into C-compatible libraries and native OS APIs.
*   [ ] **Embedding API:** Allow Valo to be easily embedded as a scripting engine in other Rust applications.

## Vision

Our long-term goal is to make Valo the go-to language for developers who value the productivity of Basic but need the performance, safety, and ecosystem of modern systems programming languages.
