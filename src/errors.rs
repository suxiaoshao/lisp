//! Error types for the Lisp interpreter.
//!
//! This module defines two main error enums:
//! - `LispError`: Top-level errors that can occur during REPL operation
//! - `LispComputerError`: Runtime errors during evaluation
//!
//! All errors implement `thiserror::Error` for easy error handling and display.

use rustyline::error::ReadlineError;

/// Top-level errors that can occur in the Lisp interpreter.
///
/// This enum wraps errors from the REPL, readline library, and runtime evaluation.
#[derive(thiserror::Error, Debug)]
pub enum LispError {
    /// The input could not be parsed as a valid Lisp expression.
    #[error("Invalid input")]
    InvalidInput,
    /// Error from the readline library (I/O, Ctrl-C, Ctrl-D, etc.)
    #[error("readline error")]
    ReadlineError(#[from] ReadlineError),
    /// Runtime error during evaluation (unbound variable, type mismatch, etc.)
    #[error("computer error: {0}")]
    ComputerError(#[from] LispComputerError),
}

/// Runtime errors that occur during Lisp expression evaluation.
///
/// These errors are raised by the evaluator when encountering invalid
/// operations, unbound variables, arity mismatches, etc.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum LispComputerError {
    /// A function or variable name is not defined in the environment.
    #[error("Unbound function:{}", .0)]
    UnboundFunction(String),
    /// Binary operation type mismatch with both operands present.
    ///
    /// E.g., `(+ "hello" 1)` would produce this error.
    #[error("Operation {} mismatch: left {}, right {}", .operation, .left_str, .right_str)]
    TypeMismatch2 {
        /// The operation name (e.g., "+", "-", etc.)
        operation: String,
        /// String representation of left operand
        left_str: String,
        /// String representation of right operand
        right_str: String,
    },
    /// Unary operation type mismatch.
    ///
    /// E.g., `(- "hello")` would produce this error.
    #[error("Operation {} mismatch: get {}", .operation, .left_str)]
    TypeMismatch1 {
        /// The operation name
        operation: String,
        /// String representation of the operand
        left_str: String,
    },
    /// Invalid arguments provided to a function or special form.
    ///
    /// Includes the function name and a list of the invalid arguments.
    #[error("Invalid arguments for function {}: {}", .0, .1.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(" "))]
    InvalidArguments(String, Vec<String>),
    /// A variable reference was not found in any scope.
    #[error("Variable not found: {}", .0)]
    NotFoundVariable(String),
    /// Wrong number of arguments to a function or special form.
    ///
    /// # Fields
    /// - `0`: Function/special form name
    /// - `1`: Expected number of arguments
    /// - `2`: Actual number of arguments received
    #[error("Arity mismatch {}: expected {}, got {}", .0, .1, .2)]
    ArityMismatch(String, usize, usize),
    /// `let` special form with naming syntax did not return a value.
    ///
    /// This is an internal error that shouldn't occur with valid code.
    #[error("Let naming not return")]
    LetNamingNotReturn,
    /// An expression is not valid in its context.
    ///
    /// E.g., trying to call a non-callable value like a number.
    #[error("Invalid expression: {0}")]
    InvalidExpression(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_error_display() {
        let err = LispComputerError::UnboundFunction("foo".to_string());
        assert_eq!(format!("{}", err), "Unbound function:foo");

        let err = LispComputerError::TypeMismatch1 {
            operation: "+".to_string(),
            left_str: Value::Number(1.0).to_string(),
        };
        assert!(format!("{}", err).contains("Operation + mismatch"));

        let err = LispComputerError::InvalidExpression("test expr".to_string());
        assert_eq!(format!("{}", err), "Invalid expression: test expr");

        let err = LispComputerError::ArityMismatch("func".to_string(), 2, 3);
        assert_eq!(format!("{}", err), "Arity mismatch func: expected 2, got 3");

        let err = LispComputerError::NotFoundVariable("x".to_string());
        assert_eq!(format!("{}", err), "Variable not found: x");
    }
}
