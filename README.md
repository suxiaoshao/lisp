# Lisp Interpreter in Rust

A Lisp interpreter implemented in Rust with garbage collection, lexical scoping, and first-class functions.

## Features

- **Lisp dialect**: Supports common Lisp-like syntax with S-expressions
- **Garbage collection**: Uses `gc-arena` for automatic memory management
- **Lexical scoping**: Proper closure capture with lexical scope
- **First-class functions**: Lambda expressions and higher-order functions
- **REPL**: Interactive read-eval-print loop with history
- **Built-in functions**: Arithmetic, comparisons, logical operators, special forms

## Language Reference

### Syntax

This interpreter implements a Lisp dialect using **S-expressions** (symbolic expressions).

#### Core Syntax Elements

- **Atoms**: indivisible values
  - Numbers: `42`, `3.14`, `-5`, `0.0`
  - Strings: `"hello"`, `"multi\nline"`, `"quote\"test"`
  - Booleans: `#t` (true), `#f` (false)
  - Nil: `nil` (empty list, also false-ish)
  - Symbols: `x`, `+`, `if`, `lambda`, `my-var`

- **Lists**: `(element1 element2 ...)`
  - Function calls: `(+ 1 2)`, `(square x)`
  - Special forms: `(if cond then else)`, `(lambda (x) body)`

#### Comments
This interpreter does **not** support comments (no `;` or `#` comments).

### Data Types

| Type | Literal Syntax | Description |
|------|----------------|-------------|
| Number | `42`, `3.14` | IEEE 754 double-precision floating point |
| String | `"hello"` | UTF-8 string with escape sequences |
| Boolean | `#t`, `#f` | Boolean truth values |
| Nil | `()` or `nil` | Empty list, also false-ish |
| Lambda | `(lambda (params) body)` | Closure (first-class function) |
| Processor | Internal | Built-in function or special form |

### Special Forms

Special forms have **lazy evaluation** rules - they control which arguments are evaluated.

#### `if` - Conditional Branching

```lisp
(if condition then else)
```

Evaluates `condition`, then evaluates either `then` or `else`.

**Examples:**
```lisp
(if #t 1 2)    ; → 1
(if #f 1 2)    ; → 2
(if (> 3 2) "yes" "no")  ; → "yes"
```

#### `lambda` - Create Anonymous Function

```lisp
(lambda (param1 param2 ...) body1 body2 ...)
```

Creates a closure with lexical scoping. Free variables are captured from the defining environment.

**Examples:**
```lisp
(lambda (x) (* x x))           ; square function
(lambda (x y) (+ x y))         ; addition
((lambda (x) (* x x)) 5)       ; → 25 (call immediately)
```

#### `define` - Define Variables and Functions

Two forms:
- Variable: `(define name value)`
- Function: `(define (name params...) body...)`

**Examples:**
```lisp
(define x 42)                   ; define variable
(define (square x) (* x x))     ; define function
(define (factorial n)
  (if (<= n 1)
      1
      (* n (factorial (- n 1)))))
```

#### `let` - Local Bindings

Two forms:
- Simple: `(let ((var1 val1) (var2 val2) ...) body...)`
- Named (recursive): `(let name ((var1 val1) ...) body...)`

**Examples:**
```lisp
(let ((x 1) (y 2)) (+ x y))     ; → 3

(let loop ((n 5) (acc 1))
  (if (= n 0)
      acc
      (loop (- n 1) (* acc n))))  ; → 120 (factorial)
```

#### `cond` - Multi-way Branching

```lisp
(cond
  (test1 result1)
  (test2 result2)
  ...
  (else default_result))
```

Evaluates tests in order, returns result of first truthy test. `else` clause is optional but recommended as catch-all.

**Examples:**
```lisp
(cond ((> 10 20) "too high")
      ((< 10 5) "too low")
      (else "just right"))       ; → "just right"
```

#### `do` - Imperative Looping

```lisp
(do ((var1 init1 step1) ...) (test result) body...)
```

General iteration construct:
1. Initialize variables with `init` expressions
2. Loop:
   - Evaluate `body` expressions
   - Evaluate `test`: if truthy, evaluate and return `result`
   - Update variables with `step` expressions

**Examples:**
```lisp
; Countdown from 10
(do ((i 10 (- i 1)))
    ((= i 0) "done"))            ; → "done"

; Sum 1 through 10
(do ((i 10 (- i 1))
     (sum 0 (+ sum i)))
    ((= i 0) sum))               ; → 55

; Factorial using do
(do ((n 5 (- n 1))
     (acc 1 (* acc n)))
    ((= n 0) acc))               ; → 120
```

### Built-in Functions

All built-in functions use **eager evaluation** - all arguments are evaluated before the function is called.

#### Arithmetic Operators

| Function | Description | Arity | Types |
|----------|-------------|-------|-------|
| `+` | Addition / string concatenation | variadic | numbers OR strings |
| `-` | Subtraction | 1+ | numbers |
| `*` | Multiplication | variadic | numbers |
| `/` | Division | 1+ | numbers |

**Examples:**
```lisp
(+ 1 2 3)          ; → 6
(+ "hello" " " "world")  ; → "hello world"
(- 10 3)           ; → 7
(- 5)              ; → -5 (negation)
(* 2 3 4)          ; → 24
(/ 20 4)           ; → 5
(/ 2)              ; → 0.5 (reciprocal)
```

**Notes:**
- `+` can concatenate strings, but mixing numbers and strings causes `TypeMismatch2` error
- Empty `+` returns 0, empty `*` returns 1
- Division by zero yields infinity (IEEE 754)

#### Comparison Operators

| Function | Description | Arity | Types |
|----------|-------------|-------|-------|
| `=` | Equal | 2+ | any |
| `>` | Greater than | 2+ | numbers |
| `<` | Less than | 2+ | numbers |
| `>=` | Greater or equal | 2+ | numbers |
| `<=` | Less or equal | 2+ | numbers |

**Examples:**
```lisp
(= 5 5)            ; → #t
(= 5 5 5)          ; → #t (all equal)
(= 1 2)            ; → #f
(> 10 5)           ; → #t
(> 10 5 3)         ; → #t (10 > 5 > 3)
(< 1 2 3)          ; → #t (1 < 2 < 3)
(>= 5 5)           ; → #t
(<= 3 3)           ; → #t
```

**Notes:**
- Numbers use IEEE 754 equality (so `(= 0.0 -0.0)` is true, but `NaN` is not equal to itself)
- Strings, booleans, nil, lambdas use structural/reference equality

#### Logical Operators

| Function | Description | Evaluation | Returns |
|----------|-------------|------------|---------|
| `and` | Logical AND | short-circuit (stops at first falsy) | First falsy or last truthy |
| `or` | Logical OR | short-circuit (stops at first truthy) | First truthy or last falsy |

**Truthiness:** Only `#f` and `nil` are falsy. Everything else (including `0`, `""`, `#t`, lambdas) is truthy.

**Examples:**
```lisp
(and #t #t)        ; → #t
(and #t #f)        ; → #f
(and 1 2 3)        ; → 3 (all truthy, returns last)
(and)              ; → #t (empty and = true)

(or #f #f #t)      ; → #t
(or #f 0)          ; → 0 (0 is truthy)
(or)               ; → #f (empty or = false)

; Short-circuit demonstration:
(and #f (/ 1 0))   ; → #f (division never evaluated, no error)
(or #t (/ 1 0))    ; → #t (division never evaluated, no error)
```

### Truthiness and Nil

In conditionals and logical operators:
- **Falsy**: `#f`, `nil`
- **Truthy**: everything else (`#t`, numbers (including `0`), strings (including `""`), lambdas, processors)

`()` (empty list) is equivalent to `nil`.

### Symbol Naming Rules

- Can contain letters, digits (but not as first char), hyphens, underscores
- Cannot start with digit: `123` is invalid, `x1` is valid
- Cannot contain whitespace or parentheses: `( )`
- Case-sensitive: `X` ≠ `x`

**Valid symbols:** `x`, `+`, `if`, `lambda`, `my-var`, `factorial`, `_private`

**Invalid symbols:** `123`, `"hello"`, `(test)`, `my var`

## Quick Example


```rust
use lisp::{parse_expression, GcArena, LispRoot, Value, LispComputerError};
use std::collections::HashMap;

fn main() -> Result<(), LispComputerError> {
    let mut arena = GcArena::new(|mc| LispRoot::new(mc));

    arena.mutate(|mc, root| {
        let (_, expr) = parse_expression(mc, "(+ 1 2 3)").unwrap();
        let vars = HashMap::new();
        let result = expr.eval(root, &vars, mc)?;
        println!("Result: {}", result); // Prints: 6
        Ok(())
    })?;

    Ok(())
}
```

## Usage

### Basic Evaluation

The core API consists of:
- `GcArena`: Garbage collected arena for allocation
- `parse_expression()`: Parse Lisp code to AST
- `Expression::eval()`: Evaluate AST to a `Value`

```rust
use lisp::{parse_expression, GcArena, LispRoot};
use std::collections::HashMap;

let mut arena = GcArena::new(|mc| LispRoot::new(mc));
arena.mutate(|mc, root| {
    // Parse expression
    let (remaining, expr) = parse_expression(mc, "(* (+ 1 2) 3)").unwrap();
    assert!(remaining.is_empty());

    // Evaluate
    let vars = HashMap::new(); // empty local scope
    let result = expr.eval(root, &vars, mc).unwrap();

    // Result is a Value enum
    println!("{}", result); // "9"
    Ok::<(), ()>(())
}).unwrap();
```

### Value Types

The `Value` enum represents all possible Lisp values:

```rust
use lisp::Value;

// Numbers (f64)
let num = Value::Number(42.0);

// Strings (GC-allocated)
let s = Value::String(Gc::new(mc, "hello".to_string()));

// Booleans
let t = Value::Boolean(true);
let f = Value::Boolean(false);

// Nil (empty list)
let nil = Value::Nil;

// Lambdas (closures)
let lambda = Value::Lambda(Gc::new(mc, my_lambda));

// Built-in functions (processor)
let proc = Value::Processor(addition_call, "+");
```

### Defining Variables

Use the `define` special form to create global variables:

```rust
use lisp::{parse_expression, GcArena, LispRoot};
use std::collections::HashMap;

let mut arena = GcArena::new(|mc| LispRoot::new(mc));
arena.mutate(|mc, root| {
    // Define x = 42
    let (_, expr) = parse_expression(mc, "(define x 42)").unwrap();
    expr.eval(root, &HashMap::new(), mc).unwrap();

    // Now x is available in global scope
    let (_, expr) = parse_expression(mc, "x").unwrap();
    let value = expr.eval(root, &HashMap::new(), mc).unwrap();
    assert_eq!(format!("{}", value), "42");

    Ok::<(), ()>(())
}).unwrap();
```

### Lambda Expressions

Create anonymous functions with `lambda`:

```rust
use lisp::{parse_expression, GcArena, LispRoot};
use std::collections::HashMap;

let mut arena = GcArena::new(|mc| LispRoot::new(mc));
arena.mutate(|mc, root| {
    // Create a lambda: (lambda (x) (* x x))
    let (_, expr) = parse_expression(mc, "(lambda (x) (* x x))").unwrap();
    let _lambda = expr.eval(root, &HashMap::new(), mc).unwrap();

    // Call it immediately: ((lambda (x) (* x x)) 5)
    let (_, call) = parse_expression(mc, "((lambda (x) (* x x)) 5)").unwrap();
    let result = call.eval(root, &HashMap::new(), mc).unwrap();
    assert_eq!(format!("{}", result), "25");

    Ok::<(), ()>(())
}).unwrap();
```

### Lexical Scoping (Closures)

Closures capture variables from their defining environment:

```rust
use lisp::{parse_expression, GcArena, LispRoot};
use std::collections::HashMap;

let mut arena = GcArena::new(|mc| LispRoot::new(mc));
arena.mutate(|mc, root| {
    // Create a closure that captures x = 10
    let code = r#"
        (let ((x 10))
            (lambda (y) (+ x y)))
    "#;
    let (_, expr) = parse_expression(mc, code).unwrap();
    let _closure = expr.eval(root, &HashMap::new(), mc).unwrap();

    // The closure remembers x=10 even though x is no longer in scope
    let call = parse_expression(mc, "((lambda (y) (+ x y)) 5)").unwrap();
    let (_, call_expr) = call;
    let result = call_expr.eval(root, &HashMap::new(), mc).unwrap();
    assert_eq!(format!("{}", result), "15");

    Ok::<(), ()>(())
}).unwrap();
```

### Special Forms

#### `if` - Conditional

```rust
use lisp::{parse_expression, GcArena, LispRoot};
use std::collections::HashMap;

let mut arena = GcArena::new(|mc| LispRoot::new(mc));
arena.mutate(|mc, root| {
    let (_, expr) = parse_expression(mc, "(if (> 10 5) \"yes\" \"no\")").unwrap();
    let result = expr.eval(root, &HashMap::new(), mc).unwrap();
    assert_eq!(format!("{}", result), "\"yes\"");
    Ok::<(), ()>(())
}).unwrap();
```

#### `cond` - Multi-way Branch

```rust
use lisp::{parse_expression, GcArena, LispRoot};
use std::collections::HashMap;

let mut arena = GcArena::new(|mc| LispRoot::new(mc));
arena.mutate(|mc, root| {
    let code = r#"
        (cond
            ((> 10 20) "too high")
            ((< 10 5) "too low")
            (else "just right"))
    "#;
    let (_, expr) = parse_expression(mc, code).unwrap();
    let result = expr.eval(root, &HashMap::new(), mc).unwrap();
    assert_eq!(format!("{}", result), "\"just right\"");
    Ok::<(), ()>(())
}).unwrap();
```

#### `let` - Local Bindings

```rust
use lisp::{parse_expression, GcArena, LispRoot};
use std::collections::HashMap;

let mut arena = GcArena::new(|mc| LispRoot::new(mc));
arena.mutate(|mc, root| {
    // Simple let
    let (_, expr) = parse_expression(mc, "(let ((x 1) (y 2)) (+ x y))").unwrap();
    let result = expr.eval(root, &HashMap::new(), mc).unwrap();
    assert_eq!(format!("{}", result), "3");

    // Named let (recursive)
    let code = r#"
        (let loop ((n 5) (acc 1))
            (if (= n 0)
                acc
                (loop (- n 1) (* acc n))))
    "#;
    let (_, expr) = parse_expression(mc, code).unwrap();
    let result = expr.eval(root, &HashMap::new(), mc).unwrap();
    assert_eq!(format!("{}", result), "120"); // 5!

    Ok::<(), ()>(())
}).unwrap();
```

#### `do` - Imperative Looping

```rust
use lisp::{parse_expression, GcArena, LispRoot};
use std::collections::HashMap;

let mut arena = GcArena::new(|mc| LispRoot::new(mc));
arena.mutate(|mc, root| {
    let code = r#"
        (do ((i 10 (- i 1))
             (sum 0 (+ sum i)))
            ((= i 0) sum))
    "#;
    let (_, expr) = parse_expression(mc, code).unwrap();
    let result = expr.eval(root, &HashMap::new(), mc).unwrap();
    assert_eq!(format!("{}", result), "55"); // sum from 1 to 10
    Ok::<(), ()>(())
    }).unwrap();
    ```
    
    ## Built-in Functions Summary

All built-in functions and special forms are available after creating a `LispRoot`:

### Special Forms

| Form | Arity | Description | Lazy? |
|------|-------|-------------|-------|
| `if` | 3 | Conditional branching | Yes (only one branch evaluated) |
| `lambda` | 2+ | Create anonymous function | No (params and body) |
| `define` | 2 | Define variable/function | No (but value expression evaluated) |
| `let` | 2+ | Local bindings | Yes (bindings evaluated before body) |
| `cond` | 2+ | Multi-way branching | Yes (only matching branch evaluated) |
| `do` | 3+ | Imperative looping | No (all parts evaluated) |

### Arithmetic Functions

| Function | Min Arity | Description | Types |
|----------|-----------|-------------|-------|
| `+` | 0 | Addition / string concatenation | numbers or strings |
| `-` | 1 | Subtraction (or negation) | numbers |
| `*` | 0 | Multiplication | numbers |
| `/` | 1 | Division (or reciprocal) | numbers |

### Comparison Functions

| Function | Min Arity | Description | Types |
|----------|-----------|-------------|-------|
| `=` | 2 | Equality | any |
| `>` | 2 | Greater than | numbers |
| `<` | 2 | Less than | numbers |
| `>=` | 2 | Greater or equal | numbers |
| `<=` | 2 | Less or equal | numbers |

### Logical Functions

| Function | Arity | Description | Short-circuit |
|----------|-------|-------------|---------------|
| `and` | 0+ | Logical AND | Yes (stops at falsy) |
| `or` | 0+ | Logical OR | Yes (stops at truthy) |

### Constants

- `#t` - true
- `#f` - false
- `nil` - empty list / falsy

### Usage Notes

- **Arity**: Special forms and functions have fixed minimum arity; calling with wrong number of arguments returns `ArityMismatch` error
- **Type checking**: Type errors return `TypeMismatch1` (unary) or `TypeMismatch2` (binary)
- **Unbound symbols**: Referencing undefined variable returns `NotFoundVariable`
- **Short-circuit**: `and` and `or` evaluate arguments left-to-right and stop early; `if`, `cond` only evaluate selected branch

    ### Arithmetic and Comparison

```rust
use lisp::{parse_expression, GcArena, LispRoot};
use std::collections::HashMap;

let mut arena = GcArena::new(|mc| LispRoot::new(mc));
arena.mutate(|mc, root| {
    // Arithmetic
    assert_eq!(eval_str(&mut arena, "(+ 1 2 3)"), Ok("6"));
    assert_eq!(eval_str(&mut arena, "(- 10 3)"), Ok("7"));
    assert_eq!(eval_str(&mut arena, "(* 2 3 4)"), Ok("24"));
    assert_eq!(eval_str(&mut arena, "(/ 20 4)"), Ok("5"));

    // Comparison
    assert_eq!(eval_str(&mut arena, "(= 5 5)"), Ok("true"));
    assert_eq!(eval_str(&mut arena, "(> 10 5)"), Ok("true"));
    assert_eq!(eval_str(&mut arena, "(< 3 7)"), Ok("true"));
    assert_eq!(eval_str(&mut arena, "(>= 5 5)"), Ok("true"));
    assert_eq!(eval_str(&mut arena, "(<= 3 3)"), Ok("true"));

    Ok::<(), ()>(())
}).unwrap();

// Helper for examples
fn eval_str(arena: &mut GcArena, code: &str) -> Result<String, lisp::LispComputerError> {
    arena.mutate(|mc, root| {
        let (_, expr) = lisp::parse_expression(mc, code).unwrap();
        let vars = std::collections::HashMap::new();
        Ok(format!("{}", expr.eval(root, &vars, mc)?))
    })
}
```

### Logical Operators

```rust
use lisp::{parse_expression, GcArena, LispRoot};
use std::collections::HashMap;

let mut arena = GcArena::new(|mc| LispRoot::new(mc));
arena.mutate(|mc, root| {
    // and - returns first false value, or last true value
    assert_eq!(eval_str(&mut arena, "(and #t #t #t)"), Ok("true"));
    assert_eq!(eval_str(&mut arena, "(and #t #f #t)"), Ok("false"));
    assert_eq!(eval_str(&mut arena, "(and 1 2 3)"), Ok("3")); // all truthy
    assert_eq!(eval_str(&mut arena, "(and)"), Ok("true")); // empty and = true

    // or - returns first true value, or last false value
    assert_eq!(eval_str(&mut arena, "(or #f #f #t)"), Ok("true"));
    assert_eq!(eval_str(&mut arena, "(or #f 0 \"hello\")"), Ok("0"));
    assert_eq!(eval_str(&mut arena, "(or)"), Ok("false")); // empty or = false

    Ok::<(), ()>(())
}).unwrap();
```

### Error Handling

All evaluation functions return `Result<Value, LispComputerError>`:

```rust
use lisp::{parse_expression, GcArena, LispRoot, LispComputerError};
use std::collections::HashMap;

let mut arena = GcArena::new(|mc| LispRoot::new(mc));
arena.mutate(|mc, root| {
    // Unbound variable
    let (_, expr) = parse_expression(mc, "undefined_var").unwrap();
    match expr.eval(root, &HashMap::new(), mc) {
        Err(LispComputerError::NotFoundVariable(name)) => {
            println!("Variable '{}' not found", name);
        }
        _ => panic!("Unexpected result"),
    }

    // Type mismatch
    let (_, expr) = parse_expression(mc, "(+ 1 \"hello\")").unwrap();
    match expr.eval(root, &HashMap::new(), mc) {
        Err(LispComputerError::TypeMismatch2 { operation, left_str, right_str }) => {
            println!("Cannot {} {} and {}", operation, left_str, right_str);
        }
        _ => panic!("Unexpected result"),
    }

    // Arity mismatch
    let (_, expr) = parse_expression(mc, "((lambda (x) x) 1 2)").unwrap();
    match expr.eval(root, &HashMap::new(), mc) {
        Err(LispComputerError::ArityMismatch(func, expected, actual)) => {
            println!("{} expects {} args, got {}", func, expected, actual);
        }
        _ => panic!("Unexpected result"),
    }

    Ok::<(), ()>(())
}).unwrap();
```

### Complete Example: Factorial

```rust
use lisp::{parse_expression, GcArena, LispRoot};
use std::collections::HashMap;

fn factorial(n: i64) -> Result<i64, lisp::errors::LispComputerError> {
    let mut arena = GcArena::new(|mc| LispRoot::new(mc));
    arena.mutate(|mc, root| {
        // Define factorial function in Lisp
        let define_code = r#"
            (define (factorial n)
                (if (<= n 1)
                    1
                    (* n (factorial (- n 1)))))
        "#;
        let (_, expr) = parse_expression(mc, define_code).unwrap();
        expr.eval(root, &HashMap::new(), mc)?;

        // Call it
        let call_code = format!("(factorial {})", n);
        let (_, call_expr) = parse_expression(mc, &call_code).unwrap();
        let result = call_expr.eval(root, &HashMap::new(), mc)?;

        if let lisp::value::Value::Number(val) = result {
            Ok(val as i64)
        } else {
            panic!("Expected number");
        }
    })
}

fn main() -> Result<(), lisp::errors::LispError> {
    assert_eq!(factorial(5)?, 120);
    assert_eq!(factorial(6)?, 720);
    assert_eq!(factorial(10)?, 3628800);
    Ok(())
}
```

## String Parsing

The `lisp::parse_string` module provides standalone string parsing:

```rust
use lisp::parse_string;
use nom::error::Error;

let result = parse_string::<Error<&str>>("\"hello\\nworld\"");
assert!(result.is_ok());
let (remaining, s) = result.unwrap();
assert!(remaining.is_empty());
assert_eq!(s, "hello\nworld");

// Unicode escapes
let result = parse_string::<Error<&str>>("\"\\u{2764} love\"");
assert!(result.is_ok());
let (_, s) = result.unwrap();
assert_eq!(s, "❤ love");
```

## Project Structure

```
src/
├── main.rs           # REPL entry point
├── lib.rs            # Library root, exports all public modules
├── parse.rs          # Parser using nom combinators
├── parse/
│   └── string.rs     # String parsing with escape sequences
├── process.rs        # Built-in functions and special forms
├── value.rs          # Value enum and ProcessorFunc type
├── value/
│   └── lambda.rs     # Lambda struct with closure capture
├── root.rs           # GC arena root (LispRoot, GcArena)
├── errors.rs         # Error types (LispError, LispComputerError)
└── test_utils.rs     # Test helper functions
```

## Public API

Everything you need is in the root module:

```rust
// Core types
use lisp::{Expression, Value, Lambda, LispRoot, GcArena};

// Parsing
use lisp::{parse_expression, parse_string};

// Errors
use lisp::{LispError, LispComputerError};
```

### Modules

- `lisp::parse`: Parser and AST (`Expression`)
- `lisp::value`: Runtime values (`Value`, `Lambda`, `ProcessorFunc`)
- `lisp::root`: GC arena (`GcArena`, `LispRoot`, `RootToken`)
- `lisp::process`: Built-in functions (all `pub` but typically used via `LispRoot::new()`)
- `lisp::errors`: Error types
- `lisp::parse_string`: Re-export of string parser

## Running the REPL

```bash
cargo run
```

The REPL supports:
- Reading expressions with line editing (via rustyline)
- History navigation (up/down arrows)
- Ctrl-C to cancel current input
- Ctrl-D or type `exit` to quit

Example session:

```
>> (+ 1 2 3)
(+ 1 2 3)
Result: 6

>> (define x 42)
(define x 42)
Result: nil

>> (if (> x 40) "big" "small")
(if (> x 40) "big" "small")
Result: "big"

>> ((lambda (a b) (* a b)) 6 7)
((lambda (a b) (* a b)) 6 7)
Result: 42

>> exit
```

## Testing

Run the test suite:

```bash
cargo test
```

The interpreter has 42+ unit tests covering:
- Parser correctness
- All arithmetic and comparison operators
- Logical operators (`and`, `or`)
- Special forms (`if`, `cond`, `lambda`, `define`, `let`, `do`)
- Lambda closures and lexical scoping
- Variable capture in nested closures
- GC preservation across mutations
- Error handling

## Dependencies

- **nom 8.0**: Parser combinators
- **gc-arena 0.5**: Garbage collected arena
- **gc-arena-derive 0.5**: Derive macros for gc-arena
- **rustyline 15.0**: Readline support
- **thiserror 2.0**: Error handling
- **anyhow 1.0** (dev): Test utilities

## Requirements

- Rust 2024 edition
- Stable toolchain (1.80+ recommended)

## Building

```bash
cargo build --release
```

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

See [AGENTS.md](AGENTS.md) for detailed contribution guidelines.
