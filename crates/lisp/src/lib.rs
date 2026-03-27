//! A Lisp interpreter implemented in Rust.
//!
//! This crate is the compatibility entrypoint for the workspace layout. The
//! interpreter implementation lives in `lisp_core`, while this crate keeps the
//! existing `lisp::...` imports stable and provides the REPL binary.
//!
//! # Quick Start
//!
//! ```
//! use lisp::{GcArena, LispRoot, parse_expression};
//! use std::collections::HashMap;
//!
//! let arena = GcArena::new(|mc| LispRoot::new(mc));
//! arena.mutate(|mc, root| {
//!     let (_, expr) = parse_expression(mc, "(+ 1 2 3)").unwrap();
//!     let result = expr.eval(root, &HashMap::new(), mc).unwrap();
//!     assert_eq!(result.to_string(), "6");
//!     Ok::<(), ()>(())
//! }).unwrap();
//! ```
//!
//! # Common Usage
//!
//! Parse and evaluate a single expression:
//!
//! ```
//! use lisp::{GcArena, LispRoot, parse_expression};
//! use std::collections::HashMap;
//!
//! let arena = GcArena::new(|mc| LispRoot::new(mc));
//! arena.mutate(|mc, root| {
//!     let (_, expr) = parse_expression(mc, "((lambda (x) (* x x)) 5)").unwrap();
//!     let value = expr.eval(root, &HashMap::new(), mc).unwrap();
//!     assert_eq!(value.to_string(), "25");
//!     Ok::<(), ()>(())
//! }).unwrap();
//! ```
//!
//! Evaluate a full program through the workspace-compatible helper:
//!
//! ```
//! use lisp::eval_program;
//!
//! let output = eval_program("(define x 10)\n(+ x 5)").unwrap();
//! assert_eq!(output, "15");
//! ```
//!
//! Parse a string literal with escapes:
//!
//! ```
//! use lisp::parse_string::parse_string;
//! use nom::error::Error;
//!
//! let (remaining, value) = parse_string::<Error<&str>>("\"hello\\nworld\"").unwrap();
//! assert!(remaining.is_empty());
//! assert_eq!(value, "hello\nworld");
//! ```
//!
//! # REPL
//!
//! Run the interactive interpreter with:
//!
//! ```text
//! cargo run -p lisp
//! ```
//!
//! The REPL supports:
//! - line editing and history via `rustyline`
//! - evaluating expressions in a persistent global environment
//! - `exit`, `Ctrl-C`, and `Ctrl-D` to leave the session
//!
//! Example session:
//!
//! ```text
//! >> (define x 42)
//! (define x 42)
//! Result: nil
//! >> (+ x 8)
//! (+ x 8)
//! Result: 50
//! ```
//!
//! # Public API
//!
//! This crate re-exports the full interpreter API from `lisp_core`, including:
//! - [`GcArena`] and [`LispRoot`] for arena-backed execution
//! - [`Expression`] and [`parse_expression`] for parsing and AST access
//! - [`Value`], [`Lambda`], and [`ProcessorFunc`] for runtime values
//! - [`LispComputerError`] for evaluation failures
//! - [`eval_program`] and [`parse_program`] for multi-form program execution

mod errors;

pub use errors::LispError;
pub use lisp_core::*;
