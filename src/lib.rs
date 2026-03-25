//! A Lisp interpreter implemented in Rust.
//!
//! This crate provides a complete Lisp interpreter with:
//! - Garbage collection via `gc-arena`
//! - Lexical scoping with proper closure capture
//! - First-class functions and built-in special forms
//! - A REPL with readline support
//!
//! # Quick Start
//!
//! ```
//! use lisp::{GcArena, LispRoot, parse_expression, Value};
//! use std::collections::HashMap;
//!
//! let mut arena = GcArena::new(|mc| LispRoot::new(mc));
//! arena.mutate(|mc, root| {
//!     // Parse and evaluate a simple expression
//!     let (_, expr) = parse_expression(mc, "(+ 1 2 3)").unwrap();
//!     let vars = HashMap::new();
//!     let result = expr.eval(root, &vars, mc).unwrap();
//!     assert_eq!(format!("{}", result), "6");
//!     Ok::<(), ()>(())
//! }).unwrap();
//! ```
//!
//! # Core API
//!
//! ## Parsing
//! ```
//! use lisp::{parse_expression, GcArena, LispRoot};
//!
//! let mut arena = GcArena::new(|mc| LispRoot::new(mc));
//! arena.mutate(|mc, _root| {
//!     let (rem, expr) = parse_expression(mc, "(* 2 3)").unwrap();
//!     assert!(rem.is_empty());
//!     Ok::<(), ()>(())
//! }).unwrap();
//! ```
//!
//! ## Values
//! ```
//! use lisp::{Value, GcArena, Gc, LispRoot};
//!
//! let mut arena = GcArena::new(|mc| LispRoot::new(mc));
//! arena.mutate(|mc, _root| {
//!     let num = Value::Number(42.0);
//!     let s = Value::String(Gc::new(mc, "hello".to_string()));
//!     let t = Value::Boolean(true);
//!     let nil = Value::Nil;
//!     Ok::<(), ()>(())
//! }).unwrap();
//! ```
//!
//! ## Built-in Functions
//! All built-ins are registered in `LispRoot::new()`:
//! - Arithmetic: `+`, `-`, `*`, `/`
//! - Comparison: `=`, `>`, `<`, `>=`, `<=`
//! - Logical: `and`, `or`
//! - Special forms: `if`, `cond`, `lambda`, `define`, `let`, `do`
//!
//! ## Error Handling
//! ```
//! use lisp::{parse_expression, GcArena, LispRoot, LispComputerError};
//!
//! let mut arena = GcArena::new(|mc| LispRoot::new(mc));
//! arena.mutate(|mc, root| {
//!     let (_, expr) = parse_expression(mc, "(+ 1 \"error\")").unwrap();
//!     let vars = std::collections::HashMap::new();
//!     match expr.eval(root, &vars, mc) {
//!         Err(LispComputerError::TypeMismatch2 { .. }) => println!("Type error!"),
//!         _ => {}
//!     }
//!     Ok::<(), ()>(())
//! }).unwrap();
//! ```
//!
//! # Re-exports
//!
//! The crate root re-exports the most commonly used types and functions:
//! - [`GcArena`] and [`LispRoot`] for GC management
//! - [`parse_expression`] for parsing
//! - [`Expression`] for AST
//! - [`Value`] and [`Lambda`] for runtime values
//! - [`LispComputerError`] for error handling
//! - [`parse_string`] for string parsing
//!
//! # Modules
//!
//! - [`parse`]: Parser and AST implementation
//! - [`value`]: Values and closures
//! - [`root`]: GC arena root
//! - `process`: Built-in function implementations (internal)
//! - [`errors`]: Error types
//! - [`parse_string`]: String parser
//!
//! For comprehensive usage examples, see the [README](README.md).

// Re-export core types and functions for convenient access
pub use crate::errors::{LispComputerError, LispError};
pub use crate::parse::{Expression, parse_expression};
pub use crate::root::{GcArena, LispRoot};
pub use crate::value::{Lambda, ProcessorFunc, Value};
pub use gc_arena::Gc;

pub mod errors;
pub mod parse;
mod process;
pub mod root;
#[allow(dead_code)]
mod test_utils;
pub mod value;

pub mod parse_string {
    //! String parsing with escape sequence support.
    //!
    //! Re-exports the `parse_string` function for parsing Lisp string literals
    //! with support for escape sequences like `\n`, `\t`, `\u{XXXX}`, etc.
    //!
    //! # Example
    //! ```
    //! use lisp::parse_string::parse_string;
    //! use nom::error::Error;
    //! let result = parse_string::<Error<&str>>("\"hello\\nworld\"");
    //! assert!(result.is_ok());
    //! let (rem, s) = result.unwrap();
    //! assert!(rem.is_empty());
    //! assert_eq!(s, "hello\nworld");
    //! ```
    //!
    //! # Unicode Example
    //! ```
    //! use lisp::parse_string::parse_string;
    //! use nom::error::Error;
    //! let result = parse_string::<Error<&str>>("\"\\u{2764} love\"");
    //! assert!(result.is_ok());
    //! let (_, s) = result.unwrap();
    //! assert_eq!(s, "❤ love");
    //! ```

    // Re-export string parsing functionality
    pub use crate::parse::string::parse_string;
}
