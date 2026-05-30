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
*   [x] **VBA Collections Foundation:** Runtime support for practical `Collection` behavior, including keyed lookup, positional access, removal, and enumeration.
*   [x] **VBA File I/O Foundation:** File-number based `Open`, `Close`, `Input`, `Print`, `Write`, `Get`, `Put`, `EOF`, `LOF`, `Seek`, and local filesystem helpers.
*   [ ] **Broader Standard Library:** Networking, JSON parsing, richer collections, and production-grade utility APIs.

## Phase 3: Developer Experience
*   [ ] **LSP Support:** A Language Server for editor features like autocomplete, go-to-definition, and real-time error reporting.
*   [ ] **Formatting & Linting:** Automated tools to keep Valo codebases clean and consistent.
*   [ ] **Package Manager:** A system for managing and distributing Valo libraries.

## Phase 4: Integration
*   [x] **FFI Foundation:** Support for VBA-style `Declare`, `PtrSafe`, `LongPtr`, `AddressOf`, callbacks, platform-aware library loading, and diagnostics for unsupported native shapes.
*   [ ] **FFI Expansion:** Broader marshaling support, type-library tooling, and more complete platform integration.
*   [ ] **Embedding API:** Allow Valo to be easily embedded as a scripting engine in other Rust applications.

## Vision

Our long-term goal is to make Valo the go-to language for developers who value the productivity of Basic but need the performance, safety, and ecosystem of modern systems programming languages.
