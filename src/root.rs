//! GC arena root and environment management for the Lisp interpreter.
//!
//! This module defines the root structure that holds the global environment,
//! along with the GC arena type and token for integrating with `gc-arena`.
//! It also defines the `LocalEnv` type for efficient lexical scoping.

use gc_arena::lock::RefLock;
use gc_arena::{Arena, Gc, Mutation, Rootable};
use gc_arena_derive::Collect;
use std::collections::{HashMap, HashSet};

use crate::process::{
    addition_call, and_call, cond_call, define_call, division_call, do_call, equal_call,
    greater_equal_call, greater_than_call, if_call, lambda_call, less_equal_call, less_than_call,
    let_call, multiplication_call, or_call, subtraction_call,
};
use crate::{errors::LispComputerError, parse::Expression, value::Value};

/// An environment frame containing variable bindings.
///
/// `EnvFrame` is an internal structure that represents a single lexical
/// frame in the environment chain. It has a parent frame (for outer scopes)
/// and a hash map of variable bindings.
#[derive(Collect, Debug, PartialEq)]
#[collect(no_drop)]
pub struct EnvFrame<'gc> {
    pub parent: Option<Gc<'gc, EnvFrame<'gc>>>,
    pub bindings: Gc<'gc, RefLock<HashMap<String, Value<'gc>>>>,
}

/// A handle to a local environment for lexical scoping.
///
/// `LocalEnv` is a lightweight handle that points to an environment frame
/// in the GC arena. It provides methods for environment lookup, extension,
/// and mutation. Unlike the old model that cloned entire hash maps, this
/// uses a chain of frames for efficient closure capture.
///
/// The environment lookup follows the chain: current frame -> parent frames
/// -> global environment (via LispRoot).
#[derive(Collect, Debug, Clone, PartialEq)]
#[collect(no_drop)]
pub struct LocalEnv<'gc> {
    pub frame: Option<Gc<'gc, EnvFrame<'gc>>>,
}

impl<'gc> LocalEnv<'gc> {
    /// Create an empty local environment with no bindings.
    pub fn empty() -> Self {
        LocalEnv { frame: None }
    }

    /// Extend the environment with a new frame containing the given bindings.
    ///
    /// Creates a new frame with the provided bindings and sets its parent
    /// to the current frame. Returns the new frame as a `LocalEnv`.
    pub fn extend_frame(
        &self,
        bindings: HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Self {
        let new_frame = Gc::new(
            mc,
            EnvFrame {
                parent: self.frame,
                bindings: Gc::new(mc, RefLock::new(bindings)),
            },
        );
        LocalEnv {
            frame: Some(new_frame),
        }
    }

    /// Extend the environment with a single binding.
    ///
    /// Creates a new frame with one binding and sets its parent to the
    /// current frame. This is a convenience method.
    pub fn extend_one(&self, name: String, value: Value<'gc>, mc: &'gc Mutation<'gc>) -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(name, value);
        self.extend_frame(bindings, mc)
    }

    /// Look up a variable in the local environment chain.
    ///
    /// Searches the current frame and all parent frames for the given
    /// variable name. Does not consult the global environment.
    pub fn lookup(&self, name: &str) -> Option<Value<'gc>> {
        let mut current = self.frame;
        while let Some(frame) = current {
            let bindings = frame.bindings.borrow();
            if let Some(value) = bindings.get(name) {
                return Some(value.clone());
            }
            current = frame.parent;
        }
        None
    }

    /// Insert a binding into the current frame only.
    ///
    /// This method only modifies the innermost frame (if it exists).
    /// It does not search parent frames. Used by `do` for mutable
    /// loop variables.
    pub fn insert_here(&self, name: String, value: Value<'gc>, mc: &'gc Mutation<'gc>) {
        if let Some(frame) = self.frame {
            frame.bindings.borrow_mut(mc).insert(name, value);
        }
    }
}

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
/// use lisp::GcArena;
/// let arena = GcArena::new(|mc| lisp::LispRoot::new(mc));
/// ```
pub type GcArena<'gc> = Arena<RootToken>;

impl<'gc> LispRoot<'gc> {
    /// Capture a set of free variables from the current local environment.
    ///
    /// Creates a new `LocalEnv` containing only the specified variable names,
    /// copying their current values from the given environment. This is used
    /// for closure capture, ensuring that captured values are immutable snapshots.
    ///
    /// # Arguments
    /// - `free_vars`: Set of variable names to capture
    /// - `locals`: Current local environment to capture from (borrowed)
    /// - `mc`: GC mutation context
    ///
    /// # Returns
    /// A new `LocalEnv` containing only the captured bindings.
    pub fn capture_env(
        &self,
        free_vars: &HashSet<String>,
        locals: &LocalEnv<'gc>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<LocalEnv<'gc>, LispComputerError> {
        let mut captured_bindings = HashMap::new();
        for name in free_vars {
            let Some(value) = self.get_variable(name, locals) else {
                return Err(LispComputerError::NotFoundVariable(name.clone()));
            };
            captured_bindings.insert(name.clone(), value);
        }
        Ok(LocalEnv {
            frame: Some(Gc::new(
                mc,
                EnvFrame {
                    parent: None,
                    bindings: Gc::new(mc, RefLock::new(captured_bindings)),
                },
            )),
        })
    }
    ///
    /// Looks up the symbol in the environment (local then global) and if found:
    /// - For `Value::Lambda`: calls the closure with the given arguments
    /// - For `Value::Processor`: calls the built-in function
    /// - For other values: returns `UnboundFunction` error
    ///
    /// # Arguments
    /// - `symbol`: The function name to look up
    /// - `args`: Array of argument expressions (unevaluated)
    /// - `locals`: Local variable environment chain
    /// - `mc`: GC mutation context
    ///
    /// # Returns
    /// - `Ok(Value)` on successful function call
    /// - `Err(LispComputerError::UnboundFunction)` if symbol not found or not callable
    pub fn process_variable(
        &'gc self,
        symbol: &str,
        args: &[Gc<'gc, Expression<'gc>>],
        locals: &LocalEnv<'gc>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if let Some(value) = self.get_variable(symbol, locals) {
            match value {
                Value::Lambda(lambda) => {
                    return lambda.call(args, self, locals, locals, mc);
                }
                Value::Processor(proc, _name) => return proc(args, self, locals, mc),
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
    /// First checks local `locals` chain, then falls back to global
    /// variables in `self.variables`.
    ///
    /// # Arguments
    /// - `name`: Variable name to look up
    /// - `locals`: Local variable environment chain (borrowed)
    ///
    /// # Returns
    /// - `Some(Value)` if found in either local or global scope
    /// - `None` if variable is unbound
    pub fn get_variable(&self, name: &str, locals: &LocalEnv<'gc>) -> Option<Value<'gc>> {
        if let Some(value) = locals.lookup(name) {
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
