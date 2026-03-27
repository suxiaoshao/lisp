//! A Lisp interpreter implemented in Rust.
//!
//! This crate provides the Lisp interpreter core with:
//! - Garbage collection via `gc-arena`
//! - Lexical scoping with proper closure capture
//! - First-class functions and built-in special forms
//!
//! # Quick Start
//!
//! ```
//! use lisp_core::{GcArena, LispRoot, parse_expression, Value};
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
//! use lisp_core::{parse_expression, GcArena, LispRoot};
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
//! use lisp_core::{Value, GcArena, Gc, LispRoot};
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
//! use lisp_core::{parse_expression, GcArena, LispRoot, LispComputerError};
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
//! - [`LispComputerError`] for runtime error handling
//! - [`parse_string`] for string parsing
//!
//! # Internal Modules
//!
//! This crate uses internal modules for organization. The public API is
//! primarily re-exported at the crate root:
//!
//! - `parse`: Internal parser implementation (use `parse_expression` from root)
//! - `value`: Internal value types (use `Value`, `Lambda` from root)
//! - `root`: Internal GC arena root (use `GcArena`, `LispRoot` from root)
//! - `process`: Internal built-in function implementations
//! - `errors`: Internal error types (use `LispComputerError` from root)
//! - [`parse_string`]: String parsing module (re-exported at root as `parse_string`)
//!
//! See the [Re-exports](#reexports) section above for the complete public API.

// Re-export core types and functions for convenient access
pub use crate::errors::LispComputerError;
pub use crate::parse::{Expression, parse_expression, parse_program};
pub use crate::root::{GcArena, LispRoot};
pub use crate::value::{Lambda, ProcessorFunc, Value};
pub use gc_arena::Gc;
use std::collections::HashMap;

mod errors;
mod parse;
mod process;
mod root;
#[allow(dead_code)]
mod test_utils;
mod value;

pub mod parse_string {
    //! String parsing with escape sequence support.
    //!
    //! Re-exports the `parse_string` function for parsing Lisp string literals
    //! with support for escape sequences like `\n`, `\t`, `\u{XXXX}`, etc.
    //!
    //! # Example
    //! ```
    //! use lisp_core::parse_string::parse_string;
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
    //! use lisp_core::parse_string::parse_string;
    //! use nom::error::Error;
    //! let result = parse_string::<Error<&str>>("\"\\u{2764} love\"");
    //! assert!(result.is_ok());
    //! let (_, s) = result.unwrap();
    //! assert_eq!(s, "❤ love");
    //! ```

    // Re-export string parsing functionality
    pub use crate::parse::string::parse_string;
}

/// Evaluate a full Lisp program consisting of zero or more top-level forms.
///
/// Forms are evaluated sequentially in a fresh arena and environment. The
/// string representation of the final value is returned.
pub fn eval_program(input: &str) -> Result<String, LispComputerError> {
    let arena = GcArena::new(|mc| LispRoot::new(mc));
    arena.mutate(|mc, root| -> Result<String, LispComputerError> {
        let (remaining, expressions) = parse_program(mc, input)
            .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;

        if !remaining.trim().is_empty() {
            return Err(LispComputerError::InvalidExpression(format!(
                "unparsed input: {}",
                remaining.trim()
            )));
        }

        let vars = HashMap::new();
        let mut value = Value::Nil;
        for expression in expressions {
            value = expression.eval(root, &vars, mc)?;
        }

        Ok(value.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_program_runs_multiple_top_level_forms() {
        assert_eq!(eval_program("(define x 2)\n(+ x 3)"), Ok("5".to_string()));
    }

    #[test]
    fn eval_program_runs_single_list_expression() {
        assert_eq!(
            eval_program("((lambda (x y) (+ x y)) 2 3)"),
            Ok("5".to_string())
        );
    }

    #[test]
    fn eval_program_rejects_unparsed_tail() {
        assert_eq!(
            eval_program("(+ 1 2))"),
            Err(LispComputerError::InvalidExpression(
                "unparsed input: )".to_string()
            ))
        );
    }
}
