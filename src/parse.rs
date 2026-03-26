//! Lisp expression parser using nom combinators.
//!
//! This module provides a complete parser for Lisp syntax, converting
//! string input into abstract syntax trees (AST) represented by the
//! `Expression` enum. The parser handles:
//! - Numbers (floating-point)
//! - Strings with escape sequences
//! - Variables (symbols)
//! - S-expressions (lists)
//! - Special forms with naming syntax (lambda, let, etc.)
//!
//! The parsing is done using the `nom` library for zero-copy, combinator-based parsing.

pub mod string;

use std::collections::HashMap;
use std::fmt::Display;

use crate::{
    errors::LispComputerError, process::process_expression_list, root::LispRoot, value::Value,
};
use gc_arena::{Gc, Mutation};
use gc_arena_derive::Collect;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{multispace0, multispace1, none_of, one_of},
    combinator::{map, not, peek, recognize},
    error::Error,
    multi::{many1, separated_list0},
    number::complete::double,
    sequence::delimited,
};

/// Abstract Syntax Tree (AST) node for Lisp expressions.
///
/// This enum represents all possible Lisp expressions before evaluation.
/// All nodes are GC-allocated using `Gc<'gc, Expression<'gc>>`.
///
/// # Variants
///
/// - `Number(f64)`: A numeric literal (e.g., `42`, `3.14`)
/// - `Variable(String)`: A symbol/variable reference (e.g., `x`, `+`, `if`)
/// - `List(Vec<Gc<Expression>>)`: An S-expression or function call (e.g., `(+ 1 2)`)
/// - `String(Gc<String>)`: A string literal (e.g., `"hello"`)
#[derive(Debug, PartialEq, Clone, Collect)]
#[collect(no_drop)]
pub enum Expression<'gc> {
    /// A numeric literal.
    Number(f64),
    /// A variable or symbol reference.
    Variable(String),
    /// An S-expression (list) representing a function call or special form.
    List(Vec<Gc<'gc, Expression<'gc>>>),
    /// A string literal.
    String(Gc<'gc, String>),
}

impl<'gc> Display for Expression<'gc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Number(number) => write!(f, "{}", number),
            Expression::Variable(name) => write!(f, "{}", name),
            Expression::List(expressions) => write!(
                f,
                "({})",
                expressions
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Expression::String(s) => write!(f, "\"{}\"", &**s),
        }
    }
}

impl<'gc> Expression<'gc> {
    /// Evaluate the expression in the given environment.
    ///
    /// This is the main entry point for evaluation. Each expression variant
    /// evaluates to a `Value`:
    ///
    /// - `Number` → `Value::Number`
    /// - `Variable` → Lookup in environment (errors if unbound)
    /// - `String` → `Value::String`
    /// - `List` → Function application via `process_expression_list`
    ///
    /// # Arguments
    /// - `env`: The global environment (`LispRoot`)
    /// - `variables`: Local variable bindings (from closures/let)
    /// - `mc`: GC mutation context
    ///
    /// # Returns
    /// - `Ok(Value)` on successful evaluation
    /// - `Err(LispComputerError)` on runtime errors (unbound variable, etc.)
    pub fn eval(
        &self,
        env: &'gc LispRoot<'gc>,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        match self {
            Expression::Number(data) => Ok(Value::Number(*data)),
            Expression::Variable(value) => env
                .get_variable(value, variables)
                .ok_or(LispComputerError::NotFoundVariable(value.to_string())),
            Expression::List(expressions) => {
                process_expression_list(expressions, env, variables, mc)
            }
            Expression::String(s) => Ok(Value::String(*s)),
        }
    }
}

/// Parse a Lisp expression from a string.
///
/// This is the top-level parser entry point. It attempts to parse any
/// valid Lisp expression and returns a GC-allocated AST node.
///
/// # Supported Syntax
///
/// - Numbers: `42`, `3.14`, `-5`
/// - Strings: `"hello"`, `"multi\nline"`, with escape sequences
/// - Variables: `x`, `+`, `if`, `lambda` (symbols not starting with digits)
/// - Lists: `(+ 1 2)`, `(define x 42)`, `(lambda (x) x)`
/// - Special forms: `(lambda ...)`, `(let ...)`, etc.
///
/// # Arguments
/// - `mc`: GC mutation context for allocation
/// - `input`: Input string to parse (must be a complete expression)
///
/// # Returns
/// - `Ok((remaining, expr))` on success, where `remaining` is any unparsed input
/// - `Err(nom::Err)` on parse failure
///
/// # Example
/// ```
/// use lisp::GcArena;
/// use lisp::parse_expression;
/// let arena = GcArena::new(|mc| lisp::LispRoot::new(mc));
/// arena.mutate(|mc, _root| {
///     let (remaining, _expr) = parse_expression(mc, "(+ 1 2)").unwrap();
///     assert!(remaining.is_empty());
///     Ok::<(), ()>(())
/// }).unwrap();
/// ```
pub fn parse_expression<'i, 'gc>(
    mc: &'gc Mutation<'gc>,
    input: &'i str,
) -> IResult<&'i str, Gc<'gc, Expression<'gc>>> {
    let (input, data) = alt((
        map(double, |n| Gc::new(mc, Expression::Number(n))),
        map(
            (tag("("), |i| parse_expression_inner(mc, i), tag(")")),
            |(_, data, _)| Gc::new(mc, Expression::List(data)),
        ),
        map(parse_lisp_variable, |name| {
            Gc::new(mc, Expression::Variable(name))
        }),
        map(string::parse_string::<Error<&str>>, |s| {
            Gc::new(mc, Expression::String(Gc::new(mc, s)))
        }),
    ))
    .parse(input)?;
    Ok((input, data))
}

/// Parse contents of a list (inner parser).
///
/// Parses zero or more expressions separated by whitespace within a list.
/// Used internally by `parse_expression` to parse list contents.
///
/// # Arguments
/// - `mc`: GC mutation context
/// - `input`: Input string (inside parentheses)
///
/// # Returns
/// - `Ok((remaining, vec_of_expressions))`
fn parse_expression_inner<'i, 'gc>(
    mc: &'gc Mutation<'gc>,
    input: &'i str,
) -> IResult<&'i str, Vec<Gc<'gc, Expression<'gc>>>> {
    let (input, data) = delimited(
        multispace0,
        separated_list0(multispace1, move |i| parse_expression(mc, i)),
        multispace0,
    )
    .parse(input)?;
    Ok((input, data))
}

/// Parse a Lisp variable/symbol name.
///
/// Symbols are non-empty strings of characters excluding whitespace,
/// parentheses, and quotes. They cannot start with a digit.
///
/// # Examples
/// - Valid: `x`, `+`, `if`, `lambda`, `my-var`
/// - Invalid: `123`, `"hello"`, `(test)`
///
/// # Arguments
/// - `input`: Input string to parse
///
/// # Returns
/// - `Ok((remaining, symbol_name))` on success
fn parse_lisp_variable(input: &str) -> IResult<&str, String> {
    let valid_char = none_of(" \t\n\r()\"");
    let (input, data) =
        recognize((not(peek(one_of("0123456789"))), many1(valid_char))).parse(input)?;
    Ok((input, data.to_string()))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{GcArena, LispComputerError};
    use anyhow::Result;

    #[test]
    fn parse_expression_inner_test() -> Result<()> {
        let arena = GcArena::new(|mc| crate::LispRoot::new(mc));
        arena.mutate(|mc, _root| -> Result<(), LispComputerError> {
            let input = "1 1";
            let (remaining, exprs) = parse_expression_inner(mc, input)
                .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;
            assert_eq!(remaining, "");
            assert_eq!(exprs.len(), 2);
            assert_eq!(&*exprs[0], &Expression::Number(1.0));
            assert_eq!(&*exprs[1], &Expression::Number(1.0));
            Ok(())
        })?;
        Ok(())
    }

    #[test]
    fn parse_expression_test() -> Result<()> {
        let arena = GcArena::new(|mc| crate::LispRoot::new(mc));
        arena.mutate(|mc, _root| -> Result<(), LispComputerError> {
            let input = "(+ 1 1)";
            let (remaining, expr) = parse_expression(mc, input)
                .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;
            assert_eq!(remaining, "");
            let expected = Expression::List(vec![
                Gc::new(mc, Expression::Variable("+".to_string())),
                Gc::new(mc, Expression::Number(1.0)),
                Gc::new(mc, Expression::Number(1.0)),
            ]);
            assert_eq!(&*expr, &expected);

            let input = "(+ 1 (* 2 3 (/ 3 1)))";
            let (remaining, expr) = parse_expression(mc, input)
                .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;
            assert_eq!(remaining, "");
            let expected = Expression::List(vec![
                Gc::new(mc, Expression::Variable("+".to_string())),
                Gc::new(mc, Expression::Number(1.0)),
                Gc::new(
                    mc,
                    Expression::List(vec![
                        Gc::new(mc, Expression::Variable("*".to_string())),
                        Gc::new(mc, Expression::Number(2.0)),
                        Gc::new(mc, Expression::Number(3.0)),
                        Gc::new(
                            mc,
                            Expression::List(vec![
                                Gc::new(mc, Expression::Variable("/".to_string())),
                                Gc::new(mc, Expression::Number(3.0)),
                                Gc::new(mc, Expression::Number(1.0)),
                            ]),
                        ),
                    ]),
                ),
            ]);
            assert_eq!(&*expr, &expected);

            // test string
            let input = "\"hello\"";
            let (remaining, expr) = parse_expression(mc, input)
                .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;
            assert_eq!(remaining, "");
            let expected = Expression::String(Gc::new(mc, "hello".to_string()));
            assert_eq!(&*expr, &expected);

            let input = r#""line\nquote\"unicode\u{2764}""#;
            let (remaining, expr) = parse_expression(mc, input)
                .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;
            assert_eq!(remaining, "");
            let expected = Expression::String(Gc::new(mc, "line\nquote\"unicode❤".to_string()));
            assert_eq!(&*expr, &expected);

            Ok(())
        })?;
        Ok(())
    }

    #[test]
    fn parse_lisp_symbol_test() -> Result<()> {
        let input = "test";
        let result = parse_lisp_variable(input);
        assert_eq!(result, Ok(("", "test".to_string())));
        Ok(())
    }
}
