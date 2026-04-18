//! Built-in functions and special forms implementation.
//!
//! This module contains all the primitive operations and special forms
//! for the Lisp interpreter. It includes:
//!
//! - Arithmetic operators: `+`, `-`, `*`, `/`
//! - Comparison operators: `=`, `>`, `<`, `>=`, `<=`
//! - Logical operators: `and`, `or`
//! - Special forms: `if`, `cond`, `lambda`, `define`, `let`, `do`
//!
//! All functions follow the `ProcessorFunc` signature and are registered
//! as `Value::Processor` in `LispRoot::new()`.

use std::collections::{HashMap, HashSet};

use crate::{errors::LispComputerError, parse::Expression, root::LispRoot, value::Value};
use gc_arena::{Gc, Mutation};

enum AdditionState {
    Empty,
    Number(f64),
    String(String),
}

fn invalid_arguments<'gc>(operation: &str, args: &[Gc<'gc, Expression<'gc>>]) -> LispComputerError {
    LispComputerError::InvalidArguments(
        operation.to_string(),
        args.iter().map(|e| format!("{}", e)).collect(),
    )
}

fn type_mismatch1(operation: &str, value: impl std::fmt::Display) -> LispComputerError {
    LispComputerError::TypeMismatch1 {
        operation: operation.to_string(),
        left_str: value.to_string(),
    }
}

fn eval_number<'gc>(
    operation: &str,
    expr: &Gc<'gc, Expression<'gc>>,
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<f64, LispComputerError> {
    match expr.eval(env, variables, mc)? {
        Value::Number(n) => Ok(n),
        other => Err(type_mismatch1(operation, other)),
    }
}

fn eval_numbers<'gc>(
    operation: &str,
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Vec<f64>, LispComputerError> {
    args.iter()
        .map(|arg| eval_number(operation, arg, env, variables, mc))
        .collect()
}

fn compare_numbers<'gc>(
    operation: &str,
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
    arity_error: LispComputerError,
    ordered: impl Fn(f64, f64) -> bool,
) -> Result<Value<'gc>, LispComputerError> {
    if args.len() < 2 {
        return Err(arity_error);
    }

    let numbers = eval_numbers(operation, args, env, variables, mc)?;
    Ok(Value::Boolean(
        numbers.windows(2).all(|pair| ordered(pair[0], pair[1])),
    ))
}

fn parse_param_names<'gc>(
    params: &[Gc<'gc, Expression<'gc>>],
) -> Result<Vec<String>, LispComputerError> {
    params
        .iter()
        .map(|param| match &**param {
            Expression::Variable(name) => Ok(name.clone()),
            _ => Err(invalid_arguments("lambda-params", params)),
        })
        .collect()
}

fn capture_free_vars<'gc>(
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    body: &[Gc<'gc, Expression<'gc>>],
    bound: &HashSet<String>,
) -> Result<HashMap<String, Value<'gc>>, LispComputerError> {
    let mut free = HashSet::new();
    for expr in body {
        crate::value::Lambda::collect_free_vars(expr, bound, &mut free);
    }

    let mut captured = HashMap::new();
    for name in &free {
        if let Some(value) = env.get_variable(name, variables) {
            captured.insert(name.clone(), value.clone());
        } else {
            return Err(LispComputerError::NotFoundVariable(name.clone()));
        }
    }
    Ok(captured)
}

fn new_lambda<'gc>(
    params: Vec<String>,
    body: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
    recursive_name: Option<&str>,
) -> Result<Gc<'gc, crate::value::Lambda<'gc>>, LispComputerError> {
    let mut bound: HashSet<String> = params.iter().cloned().collect();
    if let Some(name) = recursive_name {
        bound.insert(name.to_string());
    }
    let captured = capture_free_vars(env, variables, body, &bound)?;
    Ok(Gc::new(
        mc,
        crate::value::Lambda::new(params, body.to_vec(), captured),
    ))
}

fn parse_let_binding<'a, 'gc>(
    binding: &'a Gc<'gc, Expression<'gc>>,
    bindings: &[Gc<'gc, Expression<'gc>>],
) -> Result<(&'a str, Gc<'gc, Expression<'gc>>), LispComputerError> {
    let Expression::List(list) = &**binding else {
        return Err(invalid_arguments("let-bindings", bindings));
    };
    let [var, value] = list.as_slice() else {
        return Err(invalid_arguments("let-bindings", bindings));
    };
    let Expression::Variable(name) = &**var else {
        return Err(invalid_arguments("let-bindings", bindings));
    };
    Ok((name, *value))
}

fn parse_do_binding<'a, 'gc>(
    binding: &'a Gc<'gc, Expression<'gc>>,
    args: &[Gc<'gc, Expression<'gc>>],
) -> Result<(&'a str, Gc<'gc, Expression<'gc>>, Gc<'gc, Expression<'gc>>), LispComputerError> {
    let Expression::List(list) = &**binding else {
        return Err(invalid_arguments("do", args));
    };
    let [var, initial, step] = list.as_slice() else {
        return Err(invalid_arguments("do", args));
    };
    let Expression::Variable(name) = &**var else {
        return Err(invalid_arguments("do", args));
    };
    Ok((name, *initial, *step))
}

/// Addition operator (`+`).
///
/// Adds numbers or concatenates strings. Type-mixed operations are not allowed.
///
/// # Arguments
/// - Zero or more numbers: returns their sum (empty sum = 0)
/// - Zero or more strings: returns their concatenation
///
/// # Errors
/// - `TypeMismatch1`: If any argument is neither number nor string
/// - `TypeMismatch2`: If mixing numbers and strings
///
/// # Examples
/// - `(+ 1 2 3)` → `6`
/// - `(+ "hello" " " "world")` → `"hello world"`
/// - `(+ 1 "hello")` → Type error
pub fn addition_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    let mut state = AdditionState::Empty;

    for arg in args {
        match arg.eval(env, variables, mc)? {
            Value::Number(n) => match &mut state {
                AdditionState::Empty => state = AdditionState::Number(n),
                AdditionState::Number(sum) => *sum += n,
                AdditionState::String(result_string) => {
                    return Err(LispComputerError::TypeMismatch2 {
                        operation: "+".to_string(),
                        left_str: result_string.clone(),
                        right_str: n.to_string(),
                    });
                }
            },
            Value::String(s) => match &mut state {
                AdditionState::Empty => state = AdditionState::String(s.to_string()),
                AdditionState::String(result_string) => result_string.push_str(&s),
                AdditionState::Number(sum) => {
                    return Err(LispComputerError::TypeMismatch2 {
                        operation: "+".to_string(),
                        left_str: sum.to_string(),
                        right_str: s.to_string(),
                    });
                }
            },
            other => {
                return Err(type_mismatch1("+", other));
            }
        }
    }

    match state {
        AdditionState::Empty => Ok(Value::Number(0.0)),
        AdditionState::Number(sum) => Ok(Value::Number(sum)),
        AdditionState::String(s) if s.is_empty() => Ok(Value::Number(0.0)),
        AdditionState::String(s) => Ok(Value::String(Gc::new(mc, s))),
    }
}

/// Subtraction operator (`-`).
///
/// Subtracts numbers. With one argument, computes the negation.
///
/// # Arguments
/// - One number `x`: returns `-x`
/// - Two or more numbers `x y z...`: returns `x - y - z - ...`
///
/// # Errors
/// - `TypeMismatch1`: If any argument is not a number
/// - `TypeMismatch1`: If called with no arguments
///
/// # Examples
/// - `(- 5 2)` → `3`
/// - `(- 10 1 2)` → `7`
/// - `(- 5)` → `-5`
pub fn subtraction_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    let Some((first, rest)) = args.split_first() else {
        return Err(LispComputerError::TypeMismatch1 {
            operation: "-".to_string(),
            left_str: "nil".to_string(),
        });
    };

    let initial_value = eval_number("-", first, env, variables, mc)?;
    let value = if rest.is_empty() {
        -initial_value
    } else {
        rest.iter().try_fold(initial_value, |acc, expr| {
            Ok(acc - eval_number("-", expr, env, variables, mc)?)
        })?
    };
    Ok(Value::Number(value))
}

/// Multiplication operator (`*`).
///
/// Multiplies numbers. Empty product returns 1.
///
/// # Arguments
/// - Zero or more numbers: returns their product (empty product = 1)
///
/// # Errors
/// - `TypeMismatch1`: If any argument is not a number
///
/// # Examples
/// - `(* 2 3)` → `6`
/// - `(* 2 3 4)` → `24`
/// - `(*)` → `1`
pub fn multiplication_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    let mut result = 1.0;
    for arg in args {
        result *= eval_number("*", arg, env, variables, mc)?;
    }
    Ok(Value::Number(result))
}

/// Division operator (`/`).
///
/// Divides numbers. With one argument, computes the reciprocal (1/x).
///
/// # Arguments
/// - One number `x`: returns `1/x`
/// - Two or more numbers `x y z...`: returns `x / y / z / ...`
///
/// # Errors
/// - `TypeMismatch1`: If first argument is not a number
/// - `TypeMismatch2`: If any subsequent argument is not a number
/// - `TypeMismatch1`: If called with no arguments
///
/// # Notes
/// Division by zero returns infinity (per IEEE 754), not an error.
///
/// # Examples
/// - `(/ 10 2)` → `5`
/// - `(/ 10 2 2)` → `2.5`
/// - `(/ 2)` → `0.5`
pub fn division_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    let Some((first, rest)) = args.split_first() else {
        return Err(LispComputerError::TypeMismatch1 {
            operation: "/".to_string(),
            left_str: "nil".to_string(),
        });
    };

    let initial_value = eval_number("/", first, env, variables, mc)?;
    let value = rest.iter().try_fold(initial_value, |acc, expr| {
        let value = expr.eval(env, variables, mc)?;
        match value {
            Value::Number(n) => Ok(acc / n),
            value => Err(LispComputerError::TypeMismatch2 {
                operation: "/".to_string(),
                left_str: acc.to_string(),
                right_str: format!("{}", value),
            }),
        }
    })?;
    Ok(Value::Number(value))
}

/// Equality operator (`=`).
///
/// Compares two or more values for equality.
///
/// # Arguments
/// - Two or more values of any type
///
/// # Returns
/// - `#t` (true) if all arguments are equal
/// - `#f` (false) if any pair differs
///
/// # Errors
/// - `ArityMismatch`: If fewer than 2 arguments
///
/// # Equality Rules
/// - Numbers: IEEE 754 equality (including `NaN != NaN`)
/// - Strings: character-by-character equality
/// - Booleans: `#t` equals `#t`, `#f` equals `#f`
/// - Nil: `nil` equals `nil`
/// - Lambdas: reference equality (same closure object)
/// - Processors: equality by name (e.g., `+` equals `+`)
///
/// # Examples
/// - `(= 1 1)` → `#t`
/// - `(= 1 2)` → `#f`
/// - `(= "hello" "hello")` → `#t`
pub fn equal_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    if args.len() < 2 {
        return Err(LispComputerError::ArityMismatch(
            "=".to_string(),
            2,
            args.len(),
        ));
    }

    let mut evaluated_args = Vec::new();
    for arg in args {
        evaluated_args.push(arg.eval(env, variables, mc)?);
    }

    for pair in evaluated_args.windows(2) {
        if pair[0] != pair[1] {
            return Ok(Value::Boolean(false));
        }
    }

    Ok(Value::Boolean(true))
}

/// Greater-than operator (`>`).
///
/// Compares numbers. Returns true if each argument is strictly greater than the next.
///
/// # Arguments
/// - Two or more numbers
///
/// # Returns
/// - `#t` if `arg1 > arg2 > arg3 > ...`
/// - `#f` otherwise
///
/// # Errors
/// - `ArityMismatch`: If fewer than 2 arguments
/// - `TypeMismatch1`: If any argument is not a number
///
/// # Examples
/// - `(> 5 3)` → `#t`
/// - `(> 5 3 2)` → `#t` (5 > 3 and 3 > 2)
/// - `(> 5 5)` → `#f` (not strictly greater)
pub fn greater_than_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    compare_numbers(
        ">",
        args,
        env,
        variables,
        mc,
        LispComputerError::ArityMismatch(">".to_string(), 2, args.len()),
        |left, right| left > right,
    )
}

/// Less-than operator (`<`).
///
/// Compares numbers. Returns true if each argument is strictly less than the next.
///
/// # Arguments
/// - Two or more numbers
///
/// # Returns
/// - `#t` if `arg1 < arg2 < arg3 < ...`
/// - `#f` otherwise
///
/// # Errors
/// - `InvalidArguments`: If fewer than 2 arguments
/// - `TypeMismatch1`: If any argument is not a number
///
/// # Examples
/// - `(< 3 5)` → `#t`
/// - `(< 2 3 5)` → `#t` (2 < 3 and 3 < 5)
/// - `(< 3 3)` → `#f` (not strictly less)
pub fn less_than_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    compare_numbers(
        "<",
        args,
        env,
        variables,
        mc,
        invalid_arguments("<", args),
        |left, right| left < right,
    )
}

/// Greater-than-or-equal operator (`>=`).
///
/// Compares numbers. Returns true if each argument is greater than or equal to the next.
///
/// # Arguments
/// - Two or more numbers
///
/// # Returns
/// - `#t` if `arg1 >= arg2 >= arg3 >= ...`
/// - `#f` otherwise
///
/// # Errors
/// - `InvalidArguments`: If fewer than 2 arguments
/// - `TypeMismatch1`: If any argument is not a number
///
/// # Examples
/// - `(>= 5 3)` → `#t`
/// - `(>= 5 5 2)` → `#t` (5 >= 5 and 5 >= 2)
/// - `(>= 3 5)` → `#f`
pub fn greater_equal_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    compare_numbers(
        ">=",
        args,
        env,
        variables,
        mc,
        invalid_arguments(">=", args),
        |left, right| left >= right,
    )
}

/// Less-than-or-equal operator (`<=`).
///
/// Compares numbers. Returns true if each argument is less than or equal to the next.
///
/// # Arguments
/// - Two or more numbers
///
/// # Returns
/// - `#t` if `arg1 <= arg2 <= arg3 <= ...`
/// - `#f` otherwise
///
/// # Errors
/// - `InvalidArguments`: If fewer than 2 arguments
/// - `TypeMismatch1`: If any argument is not a number
///
/// # Examples
/// - `(<= 3 5)` → `#t`
/// - `(<= 3 3 5)` → `#t` (3 <= 3 and 3 <= 5)
/// - `(<= 5 3)` → `#f`
pub fn less_equal_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    compare_numbers(
        "<=",
        args,
        env,
        variables,
        mc,
        invalid_arguments("<=", args),
        |left, right| left <= right,
    )
}

/// Conditional special form (`if`).
///
/// Evaluates one of two branches based on a condition. Uses lazy evaluation:
/// only the selected branch is evaluated.
///
/// # Arguments
/// 1. `condition` - expression evaluated as boolean
/// 2. `then_branch` - expression evaluated if condition is true
/// 3. `else_branch` - expression evaluated if condition is false
///
/// # Returns
/// Result of evaluating the selected branch.
///
/// # Errors
/// - `ArityMismatch`: If not exactly 3 arguments
///
/// # Examples
/// - `(if #t 1 2)` → `1`
/// - `(if #f 1 2)` → `2`
/// - `(if (> 3 2) "yes" "no")` → `"yes"`
pub fn if_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    match args {
        [condition, then_branch, else_branch] => {
            let condition = condition.eval(env, variables, mc)?.boolean();
            match condition {
                true => then_branch.eval(env, variables, mc),
                false => else_branch.eval(env, variables, mc),
            }
        }
        _ => Err(LispComputerError::ArityMismatch(
            "if".to_string(),
            3,
            args.len(),
        )),
    }
}

/// Logical OR special form (`or`).
///
/// Evaluates arguments left-to-right until one returns a truthy value.
/// Uses lazy evaluation - stops at first truthy result.
///
/// # Arguments
/// - Zero or more expressions
///
/// # Returns
/// - First truthy value, or the last (falsy) value if none are truthy
///
/// # Truthiness
/// - `#f` and `nil` are falsy
/// - All other values (including `#t`, numbers, strings, lambdas) are truthy
///
/// # Examples
/// - `(or #f #f #t)` → `#t`
/// - `(or #f 0 "hello")` → `0` (first truthy)
/// - `(or)` → `#f` (empty or returns false)
pub fn or_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    let mut last_value = Value::Boolean(false);

    for arg in args {
        let value = arg.eval(env, variables, mc)?;
        if value.boolean() {
            return Ok(value);
        }
        last_value = value;
    }

    Ok(last_value)
}

/// Logical AND special form (`and`).
///
/// Evaluates arguments left-to-right until one returns a falsy value.
/// Uses lazy evaluation - stops at first falsy result.
///
/// # Arguments
/// - Zero or more expressions
///
/// # Returns
/// - First falsy value, or the last (truthy) value if all are truthy
///
/// # Truthiness
/// - `#f` and `nil` are falsy
/// - All other values are truthy
///
/// # Examples
/// - `(and #t #t)` → `#t`
/// - `(and #t #f #t)` → `#f`
/// - `(and 1 2 3)` → `3` (all truthy, returns last)
/// - `(and)` → `#t` (empty and returns true)
pub fn and_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    let mut last_value = Value::Boolean(true);

    for arg in args {
        let value = arg.eval(env, variables, mc)?;
        if !value.boolean() {
            return Ok(value);
        }
        last_value = value;
    }

    Ok(last_value)
}

/// Conditional dispatch special form (`cond`).
///
/// Evaluates a sequence of test/result pairs. Returns the result of the
/// first test that evaluates to a truthy value. The last clause can be
/// `(else result)` as a catch-all.
///
/// # Syntax
/// ```lisp
/// (cond
///   (test1 result1)
///   (test2 result2)
///   ...
///   (else default_result))
/// ```
///
/// # Arguments
/// - Two or more lists, each with a test expression and result expression
///
/// # Returns
/// Result from the first matching test clause, or the `else` clause if present.
///
/// # Errors
/// - `InvalidArguments`: If structure is malformed or no `else` clause and no tests match
///
/// # Examples
/// - `(cond (#t 1) (else 2))` → `1`
/// - `(cond (#f 1) (#f 2) (else 3))` → `3`
/// - `(cond (else 42))` → `42`
pub fn cond_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    let Some((last, clauses)) = args.split_last() else {
        return Err(invalid_arguments("cond", args));
    };

    for arg in clauses {
        if let Expression::List(inner_args) = &**arg
            && let [condition, result] = inner_args.as_slice()
        {
            let condition_value = condition.eval(env, variables, mc)?;
            if condition_value.boolean() {
                return result.eval(env, variables, mc);
            }
        }
    }
    if let Expression::List(inner_args) = &**last
        && let [var_gc, result_gc] = inner_args.as_slice()
        && let Expression::Variable(name) = &**var_gc
        && name == "else"
    {
        return result_gc.eval(env, variables, mc);
    }

    Err(invalid_arguments("cond", args))
}

/// Variable definition special form (`define`).
///
/// Defines a global variable or a function. Creates bindings in the global environment.
///
/// # Syntax
/// - Simple variable: `(define name value)`
/// - Function: `(define (name params...) body...)`
///
/// # Arguments
/// 1. Variable name or function signature `(name params...)`
/// 2. Value expression or function body
///
/// # Returns
/// `nil` on success
///
/// # Errors
/// - `InvalidArguments`: If form is malformed
/// - `NotFoundVariable`: If a free variable in function body is unbound
///
/// # Examples
/// - `(define x 42)` → defines global variable `x`
/// - `(define (square x) (* x x))` → defines function `square`
/// - `(define (factorial n) (if (= n 0) 1 (* n (factorial (- n 1)))))`
pub fn define_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    let [first, second] = args else {
        return Err(invalid_arguments("define", args));
    };

    if let Expression::Variable(name) = &**first {
        // Simple variable definition: (define x value)
        let value = second.eval(env, variables, mc)?;
        env.set_variable(name.to_string(), value, mc);
        return Ok(Value::Nil);
    }

    let Expression::List(params) = &**first else {
        return Err(invalid_arguments("define", args));
    };
    let Expression::List(body) = &**second else {
        return Err(invalid_arguments("define", args));
    };
    let [var, tail @ ..] = params.as_slice() else {
        return Err(invalid_arguments("define", args));
    };
    let Expression::Variable(name) = &**var else {
        return Err(invalid_arguments("lambda-params", params));
    };

    // Function definition: (define (name params...) body...)
    let params_vec = parse_param_names(tail)?;
    let lambda = new_lambda(params_vec, body, env, variables, mc, None)?;
    env.set_variable(name.to_string(), Value::Lambda(lambda), mc);
    Ok(Value::Nil)
}

/// Lambda (anonymous function) special form (`lambda`).
///
/// Creates a closure with lexical scoping. Captures free variables from
/// the defining environment.
///
/// # Syntax
/// `(lambda (param1 param2 ...) body1 body2 ...)`
///
/// # Arguments
/// 1. Parameter list (a list of variable names)
/// 2. One or more body expressions
///
/// # Returns
/// A closure (`Value::Lambda`) that can be called with arguments.
///
/// # Errors
/// - `InvalidArguments`: If parameter list is not a list of symbols, or no body provided
/// - `NotFoundVariable`: If a free variable in the body is not bound in the environment
///
/// # Closure Semantics
/// - Parameters are bound to evaluated arguments when called
/// - Free variables are captured from the environment at definition time (lexical scoping)
/// - Captured values are immutable snapshots of the environment at lambda creation
///
/// # Examples
/// - `(lambda (x) (+ x 1))` → closure adding 1 to its argument
/// - `(lambda (x y) (* x y))` → multiplication closure
/// - `(lambda () 42)` → zero-argument closure returning 42
pub fn lambda_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    // args should be: [params_gc, body1, body2, ...]
    let [params_gc, rest @ ..] = args else {
        return Err(invalid_arguments("lambda", args));
    };
    let Expression::List(params) = &**params_gc else {
        return Err(invalid_arguments("lambda", args));
    };

    // Extract parameter names
    let param_names = parse_param_names(params)?;
    let lambda = new_lambda(param_names, rest, env, variables, mc, None)?;

    // Create closure
    Ok(Value::Lambda(lambda))
}

/// Local binding special form (`let`).
///
/// Creates a new scope with local variable bindings. Supports two forms:
///
/// # Simple let
/// ```lisp
/// (let ((var1 val1) (var2 val2) ...) body...)
/// ```
/// Binds variables to evaluated values and evaluates body in that scope.
///
/// # Named let (recursive)
/// ```lisp
/// (let name ((var1 val1) ...) body...)
/// ```
/// Creates a named recursive lambda and immediately calls it. The `name` is
/// bound within the body for recursion (like a named `let` in Scheme).
///
/// # Arguments
/// - Bindings: list of `(var value)` pairs
/// - Body: one or more expressions evaluated sequentially
///
/// # Returns
/// Result of the last body expression.
///
/// # Errors
/// - `InvalidArguments`: If bindings or body structure is malformed
/// - `NotFoundVariable`: If a free variable is unbound
///
/// # Examples
/// - `(let ((x 1) (y 2)) (+ x y))` → `3`
/// - `(let ((x 1)) (let ((y 2)) (+ x y)))` (nested let)
/// - `(let loop ((n 10) (acc 1)) (if (= n 0) acc (loop (- n 1) (* acc n))))` (factorial)
pub fn let_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    /// Helper: construct a lambda from let bindings and body.
    fn get_lambda_from<'gc>(
        env: &'gc LispRoot<'gc>,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
        recursive_name: Option<&str>,
        bindings: &[Gc<'gc, Expression<'gc>>],
        body: &[Gc<'gc, Expression<'gc>>],
    ) -> Result<
        (
            Gc<'gc, crate::value::Lambda<'gc>>,
            Vec<Gc<'gc, Expression<'gc>>>,
        ),
        LispComputerError,
    > {
        let mut params = Vec::new();
        let mut lambda_args = Vec::new();
        for binding in bindings {
            let (name, value) = parse_let_binding(binding, bindings)?;
            params.push(name.to_string());
            lambda_args.push(value);
        }

        let lambda = new_lambda(params, body, env, variables, mc, recursive_name)?;
        Ok((lambda, lambda_args))
    }

    match args {
        // Named let: (let name ((var val) ...) body...)
        [name_expr, bindings_gc, rest @ ..]
            if let (Expression::Variable(name), Expression::List(bindings)) =
                (&**name_expr, &**bindings_gc) =>
        {
            if rest.is_empty() {
                return Err(invalid_arguments("let", args));
            }
            let (lambda, lambda_args) =
                get_lambda_from(env, variables, mc, Some(name), bindings, rest)?;
            // Bind the lambda to the name in the extended environment for recursion
            let mut new_vars = variables.clone();
            new_vars.insert(name.to_string(), Value::Lambda(lambda));
            crate::value::Lambda::call(&lambda, &lambda_args, env, &new_vars, mc)
        }
        // Simple let: (let ((var val) ...) body...)
        [bindings_gc, rest @ ..] => {
            let Expression::List(bindings) = &**bindings_gc else {
                return Err(invalid_arguments("let", args));
            };
            if rest.is_empty() {
                return Err(invalid_arguments("let", args));
            }
            let (lambda, lambda_args) = get_lambda_from(env, variables, mc, None, bindings, rest)?;
            crate::value::Lambda::call(&lambda, &lambda_args, env, variables, mc)
        }
        _ => Err(invalid_arguments("let", args)),
    }
}

/// Imperative looping and sequencing special form (`do`).
///
/// Provides sequential evaluation with initialization, stepping, and termination.
/// Implements a general loop construct similar to Scheme's `do`.
///
/// # Syntax
/// ```lisp
/// (do ((var1 init1 step1) ...) (test result) body...)
/// ```
///
/// # Arguments
/// 1. `bindings`: list of `(var init step)` triples
///    - `var`: variable name
///    - `init`: initial value expression (evaluated once)
///    - `step`: update expression (evaluated after each iteration)
/// 2. `test`: `(test_expr result_expr)` pair
///    - Loop continues until `test_expr` evaluates to truthy
///    - When truthy, returns `result_expr`
/// 3. `body`: expressions evaluated each iteration (before stepping)
///
/// # Returns
/// Result of the `result_expr` when test is satisfied.
///
/// # Errors
/// - `InvalidArguments`: If structure is malformed
///
/// # Example
/// Countdown from 10:
/// ```lisp
/// (do ((i 10 (- i 1))) ((= i 0) i))
/// ```
/// → returns `0` after counting down
///
/// Factorial using do:
/// ```lisp
/// (do ((n 5 (- n 1))
///      (acc 1 (* acc n)))
///     ((= n 0) acc))
/// ```
/// → returns `120`
pub fn do_call<'gc>(
    args: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    let [bindings_gc, test_gc, bodys @ ..] = args else {
        return Err(invalid_arguments("do", args));
    };
    let (Expression::List(bindings), Expression::List(test)) = (&**bindings_gc, &**test_gc) else {
        return Err(invalid_arguments("do", args));
    };

    let mut new_variables = variables.clone();
    let mut steps = Vec::new();
    // Process each binding: evaluate init, store step expression
    for binding in bindings {
        let (name, value_gc, step_expr_gc) = parse_do_binding(binding, args)?;
        let evaluated = value_gc.eval(env, variables, mc)?;
        new_variables.insert(name.to_string(), evaluated);
        steps.push((name.to_string(), step_expr_gc));
    }
    // Extract test and result expressions
    let [test_expr, result_expr] = test.as_slice() else {
        return Err(invalid_arguments("do", args));
    };
    // Main loop
    loop {
        if test_expr.eval(env, &new_variables, mc)?.boolean() {
            return result_expr.eval(env, &new_variables, mc);
        }
        // Evaluate body expressions
        for body in bodys {
            body.eval(env, &new_variables, mc)?;
        }
        // Update loop variables with step expressions
        for (name, step_expr) in &steps {
            let new_value = step_expr.eval(env, &new_variables, mc)?;
            new_variables.insert(name.clone(), new_value);
        }
    }
}

/// Evaluate a list of expressions as a function application.
///
/// This is the main entry point for evaluating function calls. It handles:
/// - Variable callees (lookup and call)
/// - List callees (evaluate to a lambda/processor, then call)
/// - Empty list → `nil`
///
/// # Arguments
/// - `expressions`: `[callee, arg1, arg2, ...]`
/// - `env`: Global environment
/// - `variables`: Local variable bindings
/// - `mc`: GC mutation context
///
/// # Returns
/// - `Ok(Value)` result of function call
/// - `Err(LispComputerError)` on errors (unbound function, type mismatch, etc.)
///
/// # Call Types
/// 1. Variable: `(f arg1 arg2)` → lookup `f` and call
/// 2. Lambda: `((lambda (x) ...) arg)` → evaluate lambda, then call
/// 3. Empty: `()` → `nil`
///
/// # Error Cases
/// - `InvalidExpression`: If callee is neither Variable nor List
/// - `UnboundFunction`: If variable name not found
/// - `TypeMismatch1`: If callee evaluates to non-callable (not Lambda/Processor)
pub fn process_expression_list<'gc>(
    expressions: &[Gc<'gc, Expression<'gc>>],
    env: &'gc LispRoot<'gc>,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    match expressions {
        [] => Ok(Value::Nil),
        [callee, tail @ ..] => match &**callee {
            Expression::Variable(name) => env.process_variable(name, tail, variables, mc),
            Expression::List(_) => {
                let callee_value = callee.eval(env, variables, mc)?;
                match callee_value {
                    Value::Lambda(lambda) => lambda.call(tail, env, variables, mc),
                    Value::Processor(proc, _name) => proc(tail, env, variables, mc),
                    _ => Err(LispComputerError::TypeMismatch1 {
                        operation: "function application".to_string(),
                        left_str: format!("{}", callee_value),
                    }),
                }
            }
            _ => Err(LispComputerError::InvalidExpression(format!(
                "{} is not a valid function expression",
                callee
            ))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;

    fn eval(input: &str) -> Result<String, LispComputerError> {
        let mut arena = new_arena();
        test_utils::eval_str(input, &mut arena)
    }

    fn new_arena() -> crate::root::GcArena<'static> {
        crate::root::GcArena::new(|mc| crate::root::LispRoot::new(mc))
    }

    #[test]
    fn test_addition() {
        assert_eq!(eval("(+ 1 2)"), Ok("3".to_string()));
        assert_eq!(eval("(+ 1 2 3 4)"), Ok("10".to_string()));
        assert_eq!(eval("(+ 1)"), Ok("1".to_string()));
        assert_eq!(eval("(+)"), Ok("0".to_string()));
    }

    #[test]
    fn test_subtraction() {
        assert_eq!(eval("(- 5 2)"), Ok("3".to_string()));
        assert_eq!(eval("(- 5 1 2)"), Ok("2".to_string()));
        assert_eq!(eval("(- 10)"), Ok("-10".to_string()));
    }

    #[test]
    fn test_multiplication() {
        assert_eq!(eval("(* 2 3)"), Ok("6".to_string()));
        assert_eq!(eval("(* 2 3 4)"), Ok("24".to_string()));
        assert_eq!(eval("(*)"), Ok("1".to_string()));
    }

    #[test]
    fn test_division() {
        assert_eq!(eval("(/ 10 2)"), Ok("5".to_string()));
        assert_eq!(eval("(/ 10 2 2)"), Ok("2.5".to_string()));
        let result = eval("(/ 1 0)");
        assert!(result.is_ok());
        if let Ok(s) = result {
            assert_eq!(s, "inf");
        } else {
            panic!("Expected infinite number");
        }
    }

    #[test]
    fn test_string_concatenation() {
        assert_eq!(eval(r#""hello""#), Ok("\"hello\"".to_string()));
        assert_eq!(
            eval(r#"(+ "hello" "world")"#),
            Ok("\"helloworld\"".to_string())
        );
        assert_eq!(eval(r#"(+ "a" "b" "c")"#), Ok("\"abc\"".to_string()));
    }

    #[test]
    fn test_type_mismatch_in_addition() {
        let result = eval(r#"(+ "hello" 1)"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch2 { .. })
        ));
        let result = eval(r#"(+ 1 "hello")"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch2 { .. })
        ));
        let result = eval(r#"(+ #t 1)"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch1 { .. })
        ));
        let result = eval(r#"(+ 0 "hello")"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch2 { .. })
        ));
        let result = eval(r#"(+ 1 -1 "hello")"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch2 { .. })
        ));
    }

    #[test]
    fn test_equality() {
        assert_eq!(eval("(= 1 1)"), Ok("true".to_string()));
        assert_eq!(eval("(= 1 2)"), Ok("false".to_string()));
        assert_eq!(eval("(= 1.0 1.0)"), Ok("true".to_string()));
        assert_eq!(eval("(= 1.0 2.0)"), Ok("false".to_string()));
        assert_eq!(eval("(= #t #t)"), Ok("true".to_string()));
        assert_eq!(eval("(= #t #f)"), Ok("false".to_string()));
    }

    #[test]
    fn test_comparison() {
        assert_eq!(eval("(> 2 1)"), Ok("true".to_string()));
        assert_eq!(eval("(> 1 2)"), Ok("false".to_string()));
        assert_eq!(eval("(< 1 2)"), Ok("true".to_string()));
        assert_eq!(eval("(< 2 1)"), Ok("false".to_string()));
        assert_eq!(eval("(>= 2 2)"), Ok("true".to_string()));
        assert_eq!(eval("(<= 2 2)"), Ok("true".to_string()));
    }

    #[test]
    fn test_logical_operations() {
        assert_eq!(eval("(and #t #t)"), Ok("true".to_string()));
        assert_eq!(eval("(and #t #f)"), Ok("false".to_string()));
        assert_eq!(eval("(and #f #t)"), Ok("false".to_string()));
        assert_eq!(eval("(or #f #f)"), Ok("false".to_string()));
        assert_eq!(eval("(or #f #t)"), Ok("true".to_string()));
        assert_eq!(eval("(or #t #f)"), Ok("true".to_string()));
    }

    #[test]
    fn test_lambda() {
        assert!(eval("(lambda (x) (x))").is_ok());
        assert_eq!(eval("((lambda (x y) (+ x y)) 2 3)"), Ok("5".to_string()));
        assert_eq!(eval("((lambda (x y) (* x y)) 4 5)"), Ok("20".to_string()));
        assert_eq!(eval("((lambda (x) (+ x 1)) 5)"), Ok("6".to_string()));
    }

    #[test]
    fn test_lambda_with_variable_capture() {
        let result = eval("((lambda (x) (lambda (y) (+ x y))) 2)");
        match result {
            Ok(s) => {
                assert!(s.starts_with("<lambda>"));
            }
            Err(e) => {
                panic!("Expected lambda, got error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_lambda_captures_callee_variable() {
        assert_eq!(
            eval("(let ((f (lambda (x) (+ x 1)))) ((lambda () (f 1))))"),
            Ok("2".to_string())
        );
    }

    #[test]
    fn test_closure_capture_is_lexical_not_dynamic() {
        assert_eq!(
            eval("(let ((x 2)) (let ((f (lambda (y) (+ x y)))) (let ((x 100)) (f 3))))"),
            Ok("5".to_string())
        );
    }

    #[test]
    fn test_nested_closure_captures_multiple_outer_bindings() {
        assert_eq!(
            eval("((((lambda (x) (lambda (y) (lambda (z) (+ x (+ y z))))) 1) 2) 3)"),
            Ok("6".to_string())
        );
    }

    #[test]
    fn test_closure_capture_survives_global_redefinition() {
        let mut arena = new_arena();

        assert_eq!(
            test_utils::eval_str("(define x 2)", &mut arena),
            Ok("nil".to_string())
        );
        assert_eq!(
            test_utils::eval_str("(define add-x (lambda (y) (+ x y)))", &mut arena),
            Ok("nil".to_string())
        );
        assert_eq!(
            test_utils::eval_str("(define x 100)", &mut arena),
            Ok("nil".to_string())
        );

        assert_eq!(
            test_utils::eval_str("(add-x 3)", &mut arena),
            Ok("5".to_string())
        );
    }

    #[test]
    fn test_lambda_parameter_shadows_captured_variable() {
        assert_eq!(
            eval("(((lambda (x) (lambda (x) (+ x 1))) 2) 41)"),
            Ok("42".to_string())
        );
    }

    #[test]
    fn test_lambda_captures_outer_variable_used_in_let_initializer() {
        assert_eq!(
            eval("(((lambda (x) (lambda () (let ((y x)) y))) 7))"),
            Ok("7".to_string())
        );
    }

    #[test]
    fn test_named_let_recursion_keeps_outer_lexical_capture() {
        assert_eq!(
            eval("(let ((x 10)) (let loop ((n 2)) (if (= n 0) x (loop (- n 1)))))"),
            Ok("10".to_string())
        );
    }

    #[test]
    fn test_gc_preserves_global_values_across_mutations() {
        let mut arena = new_arena();

        assert_eq!(
            test_utils::eval_str(r#"(define greeting "hello")"#, &mut arena),
            Ok("nil".to_string())
        );

        for i in 0..128 {
            let expr = format!(r#""scratch-{}-{}""#, i, "x".repeat(64));
            assert!(test_utils::eval_str(&expr, &mut arena).is_ok());
        }

        assert_eq!(
            test_utils::eval_str("greeting", &mut arena),
            Ok("\"hello\"".to_string())
        );
    }

    #[test]
    fn test_gc_preserves_captured_closure_across_mutations() {
        let mut arena = new_arena();

        assert_eq!(
            test_utils::eval_str(
                "(define add-base ((lambda (base) (lambda (x) (+ base x))) 40))",
                &mut arena,
            ),
            Ok("nil".to_string())
        );

        for i in 0..128 {
            let expr = format!("(+ {} {})", i, i + 1);
            assert!(test_utils::eval_str(&expr, &mut arena).is_ok());
        }

        assert_eq!(
            test_utils::eval_str("(add-base 2)", &mut arena),
            Ok("42".to_string())
        );
    }

    #[test]
    fn test_higher_order_builtin() {
        assert_eq!(eval("((lambda (f) (f 1 2)) +)"), Ok("3".to_string()));
    }

    #[test]
    fn test_if() {
        assert_eq!(eval("(if #t 1 2)"), Ok("1".to_string()));
        assert_eq!(eval("(if #f 1 2)"), Ok("2".to_string()));
        assert!(matches!(
            eval("(if #t \"then\" \"else\")"),
            Ok(s) if s == "\"then\""
        ));
    }

    #[test]
    fn test_define() {
        assert_eq!(eval("(define x 42)"), Ok("nil".to_string()));
    }

    #[test]
    fn test_let() {
        assert_eq!(eval("(let ((x 1) (y 2)) (+ x y))"), Ok("3".to_string()));
        assert_eq!(eval("(let ((a 10) (b 20)) (- a b))"), Ok("-10".to_string()));
    }

    #[test]
    fn test_cond() {
        assert_eq!(eval("(cond (#t 1) (else 2))"), Ok("1".to_string()));
        assert_eq!(eval("(cond (#f 1) (else 2))"), Ok("2".to_string()));
        assert_eq!(eval("(cond (else 3))"), Ok("3".to_string()));
    }

    #[test]
    fn test_do() {
        assert_eq!(
            eval("(do ((i 1 (+ i 1))) ((= i 5) i))"),
            Ok("5".to_string())
        );
    }

    #[test]
    fn test_function_application_non_lambda() {
        let result = eval("((if #t 1 2) 3)");
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch1 { .. })
        ));
    }

    #[test]
    fn test_invalid_expression() {
        let result = eval("(1 2 3 4)");
        assert!(matches!(
            result,
            Err(LispComputerError::InvalidExpression(_))
        ));
    }

    #[test]
    fn test_unbound_function() {
        let result = eval("(nonexistent 1 2)");
        assert!(matches!(result, Err(LispComputerError::UnboundFunction(_))));
    }

    #[test]
    fn test_arity_mismatch() {
        let result = eval("((lambda (x) (x)) 1 2)");
        assert!(matches!(
            result,
            Err(LispComputerError::ArityMismatch { .. })
        ));
    }

    #[test]
    fn test_variable_not_found() {
        let result = eval("undefined_var");
        assert!(matches!(
            result,
            Err(LispComputerError::NotFoundVariable(_))
        ));
    }

    #[test]
    fn test_nested_expressions() {
        assert_eq!(eval("(+ (* 2 3) (/ 10 2))"), Ok("11".to_string()));
        assert_eq!(eval("(* (+ 1 2) (- 5 2))"), Ok("9".to_string()));
    }

    #[test]
    fn test_boolean_operations() {
        assert_eq!(eval("(and #t #f #t)"), Ok("false".to_string()));
        assert_eq!(eval("(or #f #t #f)"), Ok("true".to_string()));
    }

    #[test]
    fn test_empty_list() {
        assert_eq!(eval("()"), Ok("nil".to_string()));
    }

    #[test]
    fn test_single_expression() {
        assert_eq!(eval("42"), Ok("42".to_string()));
        assert!(matches!(eval("\"test\""), Ok(s) if s == "\"test\""));
        assert_eq!(eval("#t"), Ok("true".to_string()));
        assert_eq!(eval("#f"), Ok("false".to_string()));
    }

    #[test]
    fn test_shadow_builtin_in_closure() {
        // Test that shadowing builtin functions works correctly with lexical scoping
        // (let ((+ (lambda (x y) 42))) (lambda () (+ 1 2))) should return a lambda
        let result = eval("(let ((+ (lambda (x y) 42))) (lambda () (+ 1 2)))");
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with("<lambda>"));
        // Applying the lambda should return 42, not 3
        assert_eq!(
            eval("((let ((+ (lambda (x y) 42))) (lambda () (+ 1 2))))"),
            Ok("42".to_string())
        );
    }

    #[test]
    fn test_shadow_builtin_with_define() {
        // Test define with lambda that captures shadowed builtin
        let mut arena = new_arena();
        assert_eq!(
            test_utils::eval_str(
                "(define f (let ((+ (lambda (x y) 42))) (lambda () (+ 1 2))))",
                &mut arena
            ),
            Ok("nil".to_string())
        );
        assert_eq!(
            test_utils::eval_str("(f)", &mut arena),
            Ok("42".to_string())
        );
    }

    #[test]
    fn test_shadow_builtin_direct_lambda() {
        // Direct lambda in shadowed environment
        assert_eq!(
            eval("(let ((* (lambda (x y) 99))) ((lambda (x y) (* x y)) 2 3))"),
            Ok("99".to_string())
        );
    }
}
