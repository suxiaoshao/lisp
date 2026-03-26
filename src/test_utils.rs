//! Test utilities for the Lisp interpreter.
//!
//! This module provides helper functions for testing, primarily `eval_str`
//! which allows evaluating expressions in a fresh or provided arena.

use crate::{GcArena, LispComputerError, parse_expression, root::LocalEnv};

/// Evaluate a Lisp expression from a string in the given arena.
///
/// This is a convenience function for tests that:
/// 1. Parses the input string into an expression
/// 2. Evaluates it in a fresh environment (empty local variables)
/// 3. Returns the result formatted as a display string
///
/// # Arguments
/// - `input`: Lisp expression as a string
/// - `arena`: GC arena to use for evaluation (must be mutable)
///
/// # Returns
/// - `Ok(String)`: The result value converted to string (e.g., "42", "\"hello\"", "true")
/// - `Err(LispComputerError)`: If parsing or evaluation fails
///
/// # Example
/// ```
/// use lisp::{parse_expression, GcArena, Value, LocalEnv};
/// let mut arena = GcArena::new(|mc| lisp::LispRoot::new(mc));
/// arena.mutate(|mc, root| {
///     let (_, expr) = parse_expression(mc, root, "(+ 1 2)").unwrap();
///     let locals = LocalEnv::empty();
///     let value = expr.eval(root, &locals, mc).unwrap();
///     assert_eq!(format!("{}", value), "3");
///     Ok::<(), ()>(())
/// }).unwrap();
/// ```
///
/// # Notes
/// - Uses empty local `LocalEnv` (no closure environment)
/// - All errors are converted to `LispComputerError` (parse errors become `InvalidExpression`)
/// - The arena is mutated via `arena.mutate()`
pub fn eval_str<'gc>(
    input: &str,
    arena: &'gc mut GcArena<'gc>,
) -> Result<String, LispComputerError> {
    arena.mutate(|mc, root| -> Result<String, LispComputerError> {
        let (_, expr) = parse_expression(mc, root, input)
            .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;
        let locals = LocalEnv::empty();
        let value = expr.eval(root, &locals, mc)?;
        Ok(format!("{}", value))
    })
}
