mod string;

use std::{collections::HashMap, fmt::Display};

use crate::{
    environment::Environment, errors::LispComputerError, process::process_expression_list,
    value::Value,
};
use gc_arena::{lock::RefLock, Gc, Mutation};
use gc_arena_derive::Collect;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{multispace0, multispace1, none_of, one_of},
    combinator::{map, not, peek, recognize},
    error::Error,
    multi::{many1, separated_list0},
    number::complete::double,
    sequence::delimited,
    IResult, Parser,
};

#[derive(Debug, PartialEq, Clone, Collect)]
#[collect(no_drop)]
pub enum Expression<'gc> {
    Number(f64),
    Variable(String),
    List(Vec<Gc<'gc, Expression<'gc>>>),
    String(Gc<'gc, String>),
    NamingList(String, Vec<Gc<'gc, Expression<'gc>>>),
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
            Expression::NamingList(name, expressions) => write!(
                f,
                "{name}({})",
                expressions
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl<'gc> Expression<'gc> {
    pub fn eval<T: Environment<'gc>>(
        &self,
        env: &T,
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
            Expression::String(s) => Ok(Value::String(s.clone())),
            Expression::NamingList(_, _) => Err(LispComputerError::LetNamingNotReturn),
        }
    }
}

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
        map(
            (
                parse_lisp_variable,
                tag("("),
                |i| parse_expression_inner(mc, i),
                tag(")"),
            ),
            |(name, _, expr, _)| Gc::new(mc, Expression::NamingList(name, expr)),
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

fn parse_lisp_variable(input: &str) -> IResult<&str, String> {
    let valid_char = none_of(" \t\n\r()\"");
    let (input, data) =
        recognize((not(peek(one_of("0123456789"))), many1(valid_char))).parse(input)?;
    Ok((input, data.to_string()))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{errors::LispComputerError, root::GcArena};
    use anyhow::Result;
    use gc_arena::lock::RefLock;
    use gc_arena::Gc;
    use std::collections::HashMap;

    #[test]
    fn parse_expression_inner_test() -> Result<()> {
        let mut arena = GcArena::new(|mc| crate::root::LispRoot {
            variables: Gc::new(mc, RefLock::new(HashMap::new())),
        });
        arena.mutate(|mc, _root| -> Result<(), LispComputerError> {
            let input = "1 1";
            let (remaining, exprs) =
                parse_expression_inner(mc, input).map_err(|_| LispComputerError::InvalidInput)?;
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
        let mut arena = GcArena::new(|mc| crate::root::LispRoot {
            variables: Gc::new(mc, RefLock::new(HashMap::new())),
        });
        arena.mutate(|mc, _root| -> Result<(), LispComputerError> {
            let input = "(+ 1 1)";
            let (remaining, expr) =
                parse_expression(mc, input).map_err(|_| LispComputerError::InvalidInput)?;
            assert_eq!(remaining, "");
            let expected = Expression::List(vec![
                Gc::new(mc, Expression::Variable("+".to_string())),
                Gc::new(mc, Expression::Number(1.0)),
                Gc::new(mc, Expression::Number(1.0)),
            ]);
            assert_eq!(&*expr, &expected);

            let input = "(+ 1 (* 2 3 (/ 3 1)))";
            let (remaining, expr) =
                parse_expression(mc, input).map_err(|_| LispComputerError::InvalidInput)?;
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
            let (remaining, expr) =
                parse_expression(mc, input).map_err(|_| LispComputerError::InvalidInput)?;
            assert_eq!(remaining, "");
            let expected = Expression::String(Gc::new(mc, "hello".to_string()));
            assert_eq!(&*expr, &expected);

            let input = r#""line\nquote\"unicode\u{2764}""#;
            let (remaining, expr) =
                parse_expression(mc, input).map_err(|_| LispComputerError::InvalidInput)?;
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
