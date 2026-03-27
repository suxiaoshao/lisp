//! Runtime value representation.
//!
//! This module defines the `Value` type, which is the result of evaluating
//! an `Expression`. All values are garbage-collected and stored in the GC arena.
//!
//! # Value Types
//! - String: String literals
//! - Number: Floating-point numbers (f64)
//! - Boolean: Boolean values (`#t` / `#f`)
//! - Nil: The empty value (`nil`)
//! - Lambda: User-defined closures
//! - Processor: Built-in functions and special forms
//!
//! The `Lambda` struct is defined in the `lambda` submodule.

mod lambda;

use gc_arena::Gc;
use gc_arena_derive::Collect;
use std::fmt::Display;

pub use lambda::Lambda;

/// Function pointer type for built-in operations.
///
/// All built-in functions and special forms use this signature.
/// It's a higher-ranked trait bound (HRTB) function that works with any lifetime `'gc`.
///
/// # Type Parameters
/// - `'gc`: GC arena lifetime
///
/// # Arguments
/// - `args`: Unevaluated argument expressions (AST nodes)
/// - `env`: Global environment (`LispRoot`)
/// - `variables`: Local variable bindings from surrounding scope
/// - `mc`: GC mutation context for allocation
///
/// # Returns
/// - `Ok(Value)` with the result
/// - `Err(LispComputerError)` on runtime errors
///
/// # Implementation Notes
/// Built-in functions receive unevaluated arguments and must evaluate them
/// as needed. Special forms may evaluate only some arguments (lazy evaluation).
/// The function must be `'static` (no captured non-static environment).
pub type ProcessorFunc = for<'gc> fn(
    args: &[Gc<'gc, crate::parse::Expression<'gc>>],
    env: &'gc crate::root::LispRoot<'gc>,
    variables: &std::collections::HashMap<String, Value<'gc>>,
    mc: &'gc gc_arena::Mutation<'gc>,
) -> Result<Value<'gc>, crate::errors::LispComputerError>;

/// Runtime values produced by evaluation.
///
/// All values are GC-allocated and participate in the arena's garbage collection.
/// The enum is `#[collect(no_drop)]` because the GC arena manages all lifetimes.
///
/// # Variants
///
/// - `String(Gc<String>)`: A string literal value
/// - `Number(f64)`: A numeric value (floating-point)
/// - `Boolean(bool)`: Boolean truth values (`#t` or `#f`)
/// - `Nil`: The empty/nil value (only falsy value besides `#f`)
/// - `Lambda(Gc<Lambda>)`: A user-defined closure
/// - `Processor(ProcessorFunc, &'static str)`: A built-in function or special form
#[derive(Debug, Clone, Collect)]
#[collect(no_drop)]
pub enum Value<'gc> {
    /// A string value, GC-allocated.
    String(Gc<'gc, String>),
    /// A numeric value (f64).
    Number(f64),
    /// A boolean value (`#t` or `#f`).
    Boolean(bool),
    /// The nil value (empty list, false-ish).
    Nil,
    /// A closure (lambda) capturing its environment.
    Lambda(Gc<'gc, Lambda<'gc>>),
    /// A built-in function or special form.
    ///
    /// The `&'static str` stores the function name for debugging/display.
    Processor(#[collect(require_static)] ProcessorFunc, &'static str),
}

impl<'gc> PartialEq for Value<'gc> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Lambda(a), Value::Lambda(b)) => a == b,
            (Value::Processor(_, name_a), Value::Processor(_, name_b)) => name_a == name_b,
            _ => false,
        }
    }
}

impl<'gc> Display for Value<'gc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::Lambda(l) => write!(f, "<lambda>:{}", l),
            Value::Processor(_, name) => write!(f, "<{}>", name),
        }
    }
}

impl<'gc> Value<'gc> {
    /// Convert to a boolean for truthiness testing.
    ///
    /// In Lisp, the following values are falsy:
    /// - `#f` (false)
    /// - `nil` (empty)
    ///
    /// All other values (including `#t`, numbers, strings, lambdas, processors) are truthy.
    ///
    /// # Returns
    /// - `false` for `Boolean(false)` or `Nil`
    /// - `true` for all other variants
    pub fn boolean(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Nil => false,
            _ => true,
        }
    }
}
