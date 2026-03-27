//! GC arena root and environment management for the Lisp interpreter.
//!
//! This module defines the root structure that holds the global environment,
//! along with the GC arena type and token for integrating with `gc-arena`.

use gc_arena::lock::RefLock;
use gc_arena::{Arena, Gc, Mutation, Rootable};
use gc_arena_derive::Collect;
use std::collections::HashMap;

use crate::process::{
    addition_call, and_call, cond_call, define_call, division_call, do_call, equal_call,
    greater_equal_call, greater_than_call, if_call, lambda_call, less_equal_call, less_than_call,
    let_call, multiplication_call, or_call, subtraction_call,
};
use crate::{errors::LispComputerError, parse::Expression, value::Value};

/// Root struct for the GC arena, holding global variables.
///
/// The `LispRoot` is the root of the garbage collected arena and contains
/// the global variable environment. It implements `Rootable` to allow the
/// GC arena to manage lifetimes correctly.
///
/// # Fields
/// - `variables`: A thread-safe, GC-managed hash map storing global variables
///
/// # Environment Model
/// Variable lookup is two-tiered:
/// 1. Local variables (from `let` bindings or lambda parameters) are passed
///    as the `variables` parameter to evaluation functions
/// 2. Global variables are stored in `LispRoot::variables`
///
/// The built-in functions and special forms are initialized in `LispRoot::new()`.
#[derive(Collect)]
#[collect(no_drop)]
pub struct LispRoot<'gc> {
    /// Global variables stored in a GC-managed, reference-counted hash map.
    pub variables: Gc<'gc, RefLock<HashMap<String, Value<'gc>>>>,
}

/// A token type that implements `Rootable` for any lifetime, linking to `LispRoot`.
///
/// This zero-sized marker type is used by `gc-arena` to associate the arena
/// with the root type. It implements `Rootable<'gc>` with `Root = LispRoot<'gc>`.
pub struct RootToken;

impl<'gc> Rootable<'gc> for RootToken {
    type Root = LispRoot<'gc>;
}

/// Type alias for the GC arena.
///
/// This is the main arena type used for allocating all GC-managed objects
/// in the Lisp interpreter. It is parameterized by a lifetime `'gc` representing
/// the arena's lifetime.
///
/// # Example
/// ```
/// use lisp_core::GcArena;
/// let arena = GcArena::new(|mc| lisp_core::LispRoot::new(mc));
/// ```
pub type GcArena<'gc> = Arena<RootToken>;

impl<'gc> LispRoot<'gc> {
    /// Process a variable as a function (function application).
    ///
    /// Looks up the symbol in the environment (local then global) and if found:
    /// - For `Value::Lambda`: calls the closure with the given arguments
    /// - For `Value::Processor`: calls the built-in function
    /// - For other values: returns `UnboundFunction` error
    ///
    /// # Arguments
    /// - `symbol`: The function name to look up
    /// - `args`: Array of argument expressions (unevaluated)
    /// - `variables`: Local variable bindings from surrounding scope
    /// - `mc`: GC mutation context
    ///
    /// # Returns
    /// - `Ok(Value)` on successful function call
    /// - `Err(LispComputerError::UnboundFunction)` if symbol not found or not callable
    pub fn process_variable(
        &'gc self,
        symbol: &str,
        args: &[Gc<'gc, Expression<'gc>>],
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if let Some(value) = self.get_variable(symbol, variables) {
            match value {
                Value::Lambda(lambda) => return lambda.call(args, self, variables, mc),
                Value::Processor(proc, _name) => return proc(args, self, variables, mc),
                _ => {}
            }
        }
        Err(LispComputerError::UnboundFunction(symbol.to_string()))
    }

    /// Set a global variable to a value.
    ///
    /// Inserts or updates a global variable in the root environment.
    ///
    /// # Arguments
    /// - `name`: Variable name
    /// - `value`: Value to store (GC-managed)
    /// - `mc`: GC mutation context
    pub fn set_variable(&self, name: String, value: Value<'gc>, mc: &'gc Mutation<'gc>) {
        self.variables.borrow_mut(mc).insert(name, value);
    }

    /// Get a variable value from the environment.
    ///
    /// First checks local `variables` map (from closures/let bindings), then
    /// falls back to global variables in `self.variables`.
    ///
    /// # Arguments
    /// - `name`: Variable name to look up
    /// - `variables`: Local variable bindings
    ///
    /// # Returns
    /// - `Some(Value)` if found in either local or global scope
    /// - `None` if variable is unbound
    pub fn get_variable(
        &self,
        name: &str,
        variables: &HashMap<String, Value<'gc>>,
    ) -> Option<Value<'gc>> {
        if let Some(value) = variables.get(name).cloned() {
            Some(value)
        } else {
            self.variables.borrow().get(name).cloned()
        }
    }

    /// Create a new `LispRoot` with all built-in functions and special forms.
    ///
    /// Initializes the global environment with:
    /// - Boolean constants: `#t` (true) and `#f` (false)
    /// - Arithmetic operators: `+`, `-`, `*`, `/`
    /// - Comparison operators: `=`, `>`, `<`, `>=`, `<=`
    /// - Logical operators: `and`, `or`
    /// - Special forms: `if`, `cond`, `lambda`, `define`, `let`, `do`
    ///
    /// # Arguments
    /// - `mc`: GC mutation context for allocating global variable storage
    ///
    /// # Returns
    /// A new `LispRoot` with initialized global environment.
    pub fn new(mc: &'gc Mutation<'gc>) -> Self {
        let mut vars = HashMap::new();
        vars.insert("#f".to_string(), Value::Boolean(false));
        vars.insert("#t".to_string(), Value::Boolean(true));
        // Register all built-in functions and special forms
        vars.insert("+".to_string(), Value::Processor(addition_call, "+"));
        vars.insert("-".to_string(), Value::Processor(subtraction_call, "-"));
        vars.insert("*".to_string(), Value::Processor(multiplication_call, "*"));
        vars.insert("/".to_string(), Value::Processor(division_call, "/"));
        vars.insert("=".to_string(), Value::Processor(equal_call, "="));
        vars.insert(">".to_string(), Value::Processor(greater_than_call, ">"));
        vars.insert("<".to_string(), Value::Processor(less_than_call, "<"));
        vars.insert(">=".to_string(), Value::Processor(greater_equal_call, ">="));
        vars.insert("<=".to_string(), Value::Processor(less_equal_call, "<="));
        vars.insert("if".to_string(), Value::Processor(if_call, "if"));
        vars.insert("or".to_string(), Value::Processor(or_call, "or"));
        vars.insert("and".to_string(), Value::Processor(and_call, "and"));
        vars.insert("cond".to_string(), Value::Processor(cond_call, "cond"));
        vars.insert(
            "define".to_string(),
            Value::Processor(define_call, "define"),
        );
        vars.insert(
            "lambda".to_string(),
            Value::Processor(lambda_call, "lambda"),
        );
        vars.insert("let".to_string(), Value::Processor(let_call, "let"));
        vars.insert("do".to_string(), Value::Processor(do_call, "do"));

        LispRoot {
            variables: Gc::new(mc, RefLock::new(vars)),
        }
    }
}
