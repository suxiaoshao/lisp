# Lisp Interpreter in Rust

A Lisp interpreter implemented in Rust with garbage collection, lexical scoping, and first-class functions.

## Features

- **Lisp dialect**: Supports common Lisp-like syntax with S-expressions
- **Garbage collection**: Uses `gc-arena` for automatic memory management
- **Lexical scoping**: Proper closure capture with lexical scope
- **First-class functions**: Lambda expressions and higher-order functions
- **REPL**: Interactive read-eval-print loop with history
- **Built-in functions**: Arithmetic, comparisons, list operations, and more

## Project Structure

```
src/
├── main.rs           # REPL entry point
├── parse.rs          # Parser using nom
├── process.rs        # Expression evaluation and built-in functions
├── value.rs          # Value types (Number, String, Boolean, Nil, Lambda)
├── value/
│   └── lambda.rs     # Lambda implementation with closure capture
├── environment.rs    # Environment management
├── errors.rs         # Error types
├── root.rs           # GC arena root and lifecycle management
└── test_utils.rs     # Test utilities
```

## Requirements

- Rust 2024 edition
- Stable toolchain

## Building

```bash
cargo build
```

## Running

```bash
cargo run
```

This starts the REPL. Type Lisp expressions followed by Enter:

```
>> (+ 1 2 3)
6
>> (define factorial (lambda (n) (if (<= n 1) 1 (* n (factorial (- n 1))))))
>> (factorial 5)
120
>> exit
```

## Testing

```bash
cargo test
```

## CI/CD

The project uses GitHub Actions with the following checks:
- `cargo fmt --check` (Ubuntu)
- `cargo test --locked`
- `cargo clippy --all-targets --all-features --locked -- -D warnings` (Ubuntu)

Runs on Ubuntu, macOS, and Windows.

## Branch Protection

The `main` branch is protected:
- Direct pushes are disallowed
- Pull requests require at least 1 review approval
- CI workflow must pass before merging
- Linear history is not enforced

## Dependencies

- `nom = "8.0.0"` - Parser combinator library
- `rustyline = "15.0.0"` - Readline implementation for REPL
- `thiserror = "2.0.11"` - Error handling
- `gc-arena = "0.5"` - Garbage collected arena allocation
- `gc-arena-derive = "0.5"` - Derive macros for gc-arena

## Development Guidelines

See [AGENTS.md](AGENTS.md) for detailed contribution guidelines including:
- Code modification principles
- Rust and interpreter rules
- GitHub workflow
- Validation and CI requirements

## License

MIT License - see [LICENSE](LICENSE) file for details.
