# Contributing to Valo

Thank you for your interest in contributing to Valo! As an experimental project, we welcome help in many forms, from bug reports to new language features.

## How to Contribute

### 1. Report Bugs
If you find a bug, please open an issue on GitHub. Include:
- A minimal reproduction script (`.valo` file).
- The expected output vs. actual output.
- Your environment details (OS, Rust version).

### 2. Suggest Features
We are actively refining the Valo language. If you have ideas for syntax improvements or missing features, please open a Discussion or Issue.

### 3. Submit Pull Requests
1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/amazing-feature`).
3. Commit your changes (`git commit -m 'Add some amazing feature'`).
4. Push to the branch (`git push origin feature/amazing-feature`).
5. Open a Pull Request.

## Development Workflow

### Building from Source
Valo is written in Rust. You will need the latest stable Rust toolchain.

```bash
cargo build
```

### Running Tests
We maintain high standards for language correctness. Always run the test suite before submitting a PR.

```bash
cargo test
```

The test suite includes:
- Unit tests for lexer, parser, and semantic validator.
- Integration tests that run all files in the `examples/` directory.

### Code Style
- Follow standard Rust idioms and `rustfmt` defaults.
- Ensure all public functions and types in `core` are documented.
- Keep the Basic-style syntax of Valo consistent with its VBA-inspired philosophy.

## Roadmap & Priorities
Check the [README.md](README.md) for the current roadmap. We are currently prioritizing the Bytecode VM and standard library foundations.
