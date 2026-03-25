# Lisp Interpreter in Rust

A simple Lisp/Scheme-like interpreter implemented in Rust using `gc-arena` for garbage collection.

## Features

- Basic arithmetic: `+`, `-`, `*`, `/`
- Comparison: `=`, `<`, `>`, `<=`, `>=`
- Logical operations: `and`, `or`, `if`, `cond`
- Lambda expressions with lexical scoping and closure capture
- `define` for global definitions
- `let` for local bindings (including named let for recursion)
- `do` for sequential evaluation
- String concatenation with `+`

## Build and Run

```bash
cargo build
cargo run
```

## Testing

```bash
cargo test
```

## Examples

```lisp
;; Basic arithmetic
(+ 1 2 3)  ; => 6

;; Lambda expressions
((lambda (x) (* x x)) 5)  ; => 25

;; Closures with lexical scoping
(let ((x 10))
  (lambda (y) (+ x y)))  ; captures x=10

;; Define and let
(define square (lambda (x) (* x x)))
(let ((a 1) (b 2)) (+ a b))

;; Recursion with named let
(let loop ((n 5) (acc 1))
  (if (= n 0)
      acc
      (loop (- n 1) (* acc n))))  ; factorial
```

## Architecture

- **gc-arena**: Garbage-collected memory arena for managing Lisp values
- **parse.rs**: Parser using `nom` library
- **process.rs**: Built-in functions and special forms
- **root.rs**: `LispRoot` struct holds global environment
- **value.rs**: Core value types (Number, String, Boolean, Lambda, Processor)
- **value/lambda.rs**: Lambda implementation with free variable capture

## License

MIT
