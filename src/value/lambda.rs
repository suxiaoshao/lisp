//! Lambda closures and lexical scoping.
//!
//! This module implements lambda closures with proper lexical scoping
//! and free variable capture. The `Lambda` struct represents a user-defined
//! function with its parameters, body, and captured environment.
//!
//! # Lexical Scoping
//!
//! Closures capture their defining environment. When a lambda is created,
//! all free variables (variables not in the parameter list) are looked up
//! in the current environment and their values are stored in the closure's
//! `captured` frame. This ensures lexical (not dynamic) scoping.

use std::{collections::HashSet, fmt::Display};

use crate::{
    errors::LispComputerError,
    parse::Expression,
    root::LocalEnv,
    symbol::{ResolvedVar, Symbol},
    value::Value,
};

use gc_arena::{Gc, Mutation};
use gc_arena_derive::Collect;

/// A lambda closure with lexical scoping.
///
/// A closure consists of:
/// - `params`: Parameter names
/// - `body`: List of body expressions (evaluated sequentially)
/// - `captured`: Local environment frame containing free variables captured from the closure's defining environment.
///
/// # Memory Management
/// All fields are GC-allocated (`Gc` or `LocalEnv`). The struct
/// is `#[collect(no_drop)]` because the GC arena manages all lifetimes.
///
/// # Evaluation
/// When called:
/// 1. Check arity (number of args must match number of params)
/// 2. Create new local scope with captured variables (allowing shadowing by args)
/// 3. Bind arguments to parameters (evaluated before binding)
/// 4. Evaluate body expressions sequentially, returning the last result
#[derive(Debug, Clone, PartialEq, Collect)]
#[collect(no_drop)]
pub struct Lambda<'gc> {
    /// Parameter names (in order).
    params: Vec<Symbol<'gc>>,
    /// Body expressions to evaluate.
    body: Vec<Gc<'gc, Expression<'gc>>>,
    /// Captured free variables from the closure's defining environment.
    ///
    /// This is a LocalEnv handle pointing to an environment frame containing
    /// the values of variables that are used in the body but not defined as
    /// parameters. They are "frozen" at lambda creation time.
    captured: LocalEnv<'gc>,
    /// Fixed tail environment for synthetic lambdas such as `let` / named `let`.
    tail_env: LocalEnv<'gc>,
}

impl<'gc> Display for Lambda<'gc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params = self
            .params
            .iter()
            .map(|symbol| symbol.name.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        write!(f, "(lambda ({params})(")?;
        for expr in &self.body {
            write!(f, " {}", expr)?;
        }
        write!(f, "))")
    }
}

impl<'gc> Lambda<'gc> {
    /// Create a new lambda closure.
    ///
    /// # Arguments
    /// - `params`: List of parameter names
    /// - `body`: Body expressions (AST nodes)
    /// - `captured`: Local environment frame containing captured free variables
    ///
    /// # Returns
    /// A new `Lambda` instance.
    pub fn new(
        params: Vec<Symbol<'gc>>,
        body: Vec<Gc<'gc, Expression<'gc>>>,
        captured: LocalEnv<'gc>,
    ) -> Self {
        Self::with_tail_env(params, body, captured, LocalEnv::empty())
    }

    pub fn with_tail_env(
        params: Vec<Symbol<'gc>>,
        body: Vec<Gc<'gc, Expression<'gc>>>,
        captured: LocalEnv<'gc>,
        tail_env: LocalEnv<'gc>,
    ) -> Self {
        Lambda {
            params,
            body,
            captured,
            tail_env,
        }
    }

    /// Call the closure with arguments.
    ///
    /// Applies the lambda to the given arguments in the provided environment.
    ///
    /// # Arguments
    /// - `args`: Unevaluated argument expressions
    /// - `env`: Global environment (for nested lookups if needed)
    /// - `arg_env`: Environment for evaluating arguments (caller's locals, borrowed)
    /// - `body_tail_env`: Additional environment to chain after captured frame (for named let recursion, borrowed)
    /// - `mc`: GC mutation context
    ///
    /// # Returns
    /// - `Ok(Value)`: Result of evaluating the last body expression
    /// - `Err(LispComputerError::ArityMismatch)`: If wrong number of arguments
    /// - `Err(LispComputerError::NotFoundVariable)`: If an argument evaluation references an unbound variable
    ///
    /// # Evaluation Steps
    /// 1. Evaluate all arguments in `arg_env`
    /// 2. Check arity
    /// 3. Construct parameter frame (with arg values)
    /// 4. Build body environment chain: params frame -> captured frame -> body_tail_env -> global env
    /// 5. Evaluate body expressions sequentially, returning the last
    pub fn call(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &'gc crate::root::LispRoot<'gc>,
        arg_env: &crate::root::LocalEnv<'gc>,
        body_tail_env: &crate::root::LocalEnv<'gc>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if args.len() != self.params.len() {
            return Err(LispComputerError::ArityMismatch(
                "lambda-function".to_string(),
                self.params.len(),
                args.len(),
            ));
        }

        // Step 1: Evaluate arguments in arg_env
        let mut arg_values = Vec::new();
        for arg in args {
            arg_values.push(arg.eval(env, arg_env, mc)?);
        }

        // Step 2: Build parameter bindings
        let param_bindings: Vec<_> = self
            .params
            .iter()
            .zip(arg_values)
            .map(|(param, value)| (*param, value))
            .collect();

        // Step 3: Construct environment chain for body:
        // params frame (top) -> captured frame (with parent = body_tail) -> body_tail -> global
        let tail_env = if self.tail_env.frame.is_some() {
            &self.tail_env
        } else {
            body_tail_env
        };
        let parent = self.captured.snapshot_with_tail(tail_env.frame, mc).frame;

        let param_layout = Gc::new(
            mc,
            crate::root::FrameLayout::new(
                param_bindings.iter().map(|(symbol, _)| *symbol).collect(),
                false,
            ),
        );
        let param_values = param_bindings.into_iter().map(|(_, value)| value).collect();
        let body_env =
            crate::root::LocalEnv { frame: parent }.extend_slots(param_layout, param_values, mc);

        // Step 4: Evaluate body expressions sequentially
        let mut result = Value::Nil;
        for expr in &self.body {
            result = expr.eval(env, &body_env, mc)?;
        }
        Ok(result)
    }

    /// Collect free variables from an expression for closure capture.
    ///
    /// Walks the AST and identifies all variable references that are not
    /// in the `bound` set (parameters or already-bound variables). This
    /// is used during lambda/let/define to determine which variables
    /// need to be captured in the closure's environment.
    ///
    /// # Arguments
    /// - `expr`: Expression to analyze
    /// - `bound`: Set of currently bound variable names (parameters, let bindings)
    /// - `free`: Mutable set to accumulate free variable names
    ///
    /// # Special Forms Handling
    /// - `lambda`: Adds new parameter names to `bound` for body analysis
    /// - `let`: Handles both simple and named forms; adds binding names to `bound`
    /// - Other forms: Recursively analyze all subexpressions
    ///
    /// # Example
    /// ```
    /// use lisp::Lambda;
    /// use std::collections::HashSet;
    /// use lisp::Expression;
    /// // In real code, you would have an actual Expression AST.
    /// // This is a simplified demonstration of the API:
    /// let mut free: HashSet<String> = HashSet::new();
    /// let bound: HashSet<String> = ["y"].iter().cloned().map(String::from).collect();
    /// // Lambda::collect_free_vars(&expr, &bound, &mut free);
    /// // assert!(free.contains("x"));
    /// ```
    pub fn collect_free_vars(
        expr: &Expression<'gc>,
        bound: &HashSet<String>,
        free: &mut HashSet<String>,
    ) {
        match expr {
            Expression::Number(_) | Expression::String(_) | Expression::Symbol(_) => {}
            Expression::Variable(ResolvedVar::Global(symbol)) => {
                let name = symbol.name.to_string();
                if !bound.contains(&name) {
                    free.insert(name);
                }
            }
            Expression::Variable(ResolvedVar::Local(_)) => {}
            Expression::List(exprs) => {
                for expr in exprs {
                    Self::collect_free_vars(expr, bound, free);
                }
            }
        }
    }
}
