use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{multispace0, multispace1, none_of, one_of},
    combinator::{map, not, peek, recognize},
    multi::{many1, separated_list0},
    number::complete::float,
    sequence::delimited,
    IResult, Parser,
};

#[derive(thiserror::Error, Debug)]
enum LispError {
    #[error("Invalid input")]
    InvalidInput,
    #[error("readline error")]
    ReadlineError(#[from] ReadlineError),
    #[error("computer error")]
    ComputerError,
    #[error("sub error:{}",.0)]
    SubError(String),
    #[error("division error:{}",.0)]
    DivError(String),
}

use rustyline::{error::ReadlineError, DefaultEditor};

fn main() -> Result<(), LispError> {
    // 创建一个 rustyline 编辑器
    let mut rl = DefaultEditor::new()?;

    loop {
        // 读取用户输入
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;

                let (_, expression) =
                    parse_expression(&line).map_err(|_| LispError::InvalidInput)?;

                println!("{expression:?}");

                let result = process_expression(&expression)?;

                println!("Result: {}", result);

                // 自定义退出条件
                if line.trim() == "exit" {
                    break;
                }
            }
            Err(ReadlineError::Interrupted) => {
                // 用户使用 Ctrl-C 中断
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                // 用户使用 Ctrl-D 退出
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                // 其他错误
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}

fn process_expression(expression: &Expression) -> Result<f32, LispError> {
    match expression {
        Expression::Number(data) => Ok(*data),
        Expression::Variable(_) => Err(LispError::ComputerError),
        Expression::List(expressions) => process_expression_list(expressions),
    }
}

fn process_expression_list(expressions: &[Expression]) -> Result<f32, LispError> {
    match expressions {
        [] => Ok(0.0),
        [Expression::Number(data)] => Ok(*data),
        [Expression::Variable(symbol), tail @ ..] => process_variable(symbol, tail),
        _ => unimplemented!(),
    }
}

fn process_variable(symbol: &str, tail: &[Expression]) -> Result<f32, LispError> {
    match symbol {
        "+" => process_addition(tail),
        "-" => process_subtraction(tail),
        "*" => process_multiplication(tail),
        "/" => process_division(tail),
        _ => unimplemented!(),
    }
}

fn process_addition(tail: &[Expression]) -> Result<f32, LispError> {
    tail.iter().try_fold(0.0, |acc, expr| {
        let value = process_expression(expr)?;
        Ok(acc + value)
    })
}

fn process_subtraction(tail: &[Expression]) -> Result<f32, LispError> {
    if let Some((first, rest)) = tail.split_first() {
        let initial_value = process_expression(first)?;
        rest.iter().try_fold(initial_value, |acc, expr| {
            let value = process_expression(expr)?;
            Ok(acc - value)
        })
    } else {
        Err(LispError::SubError("no data".to_string()))
    }
}

fn process_multiplication(tail: &[Expression]) -> Result<f32, LispError> {
    tail.iter().try_fold(1.0, |acc, expr| {
        let value = process_expression(expr)?;
        Ok(acc * value)
    })
}

fn process_division(tail: &[Expression]) -> Result<f32, LispError> {
    if let Some((first, rest)) = tail.split_first() {
        let initial_value = process_expression(first)?;
        rest.iter().try_fold(initial_value, |acc, expr| {
            let value = process_expression(expr)?;
            Ok(acc / value)
        })
    } else {
        Err(LispError::DivError("no data".to_string()))
    }
}

#[derive(Debug, PartialEq)]
enum Expression {
    Number(f32),
    Variable(String),
    List(Vec<Expression>),
}

fn parse_expression(input: &str) -> IResult<&str, Expression> {
    let (input, data) = alt((
        map(float, Expression::Number),
        map(
            (tag("("), parse_expression_inner, tag(")")),
            |(_, data, _)| Expression::List(data),
        ),
        map(parse_lisp_symbol, Expression::Variable),
    ))
    .parse(input)?;
    Ok((input, data))
}

fn parse_expression_inner(input: &str) -> IResult<&str, Vec<Expression>> {
    let (input, data) = delimited(
        multispace0,
        separated_list0(multispace1, parse_expression),
        multispace0,
    )
    .parse(input)?;
    Ok((input, data))
}

fn parse_lisp_symbol(input: &str) -> IResult<&str, String> {
    let valid_char = none_of(" \t\n\r()");
    let (input, data) =
        recognize((not(peek(one_of("0123456789"))), many1(valid_char))).parse(input)?;
    Ok((input, data.to_string()))
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn parse_expression_inner_test() -> anyhow::Result<()> {
        let input = " 1 1 ";
        let result = parse_expression_inner(input);

        assert_eq!(
            result,
            Ok(("", vec![Expression::Number(1.0), Expression::Number(1.0)]))
        );
        Ok(())
    }
    #[test]
    fn parse_expression_test() -> anyhow::Result<()> {
        let input = "(+ 1 1)";
        let result = parse_expression(input);

        assert_eq!(
            result,
            Ok((
                "",
                Expression::List(vec![
                    Expression::Variable("+".to_string()),
                    Expression::Number(1.0),
                    Expression::Number(1.0)
                ])
            ))
        );

        let input = "(+ 1 (* 2 3 (/ 3 1)))";
        let result = parse_expression(input);

        assert_eq!(
            result,
            Ok((
                "",
                Expression::List(vec![
                    Expression::Variable("+".to_string()),
                    Expression::Number(1.0),
                    Expression::List(vec![
                        Expression::Variable("*".to_string()),
                        Expression::Number(2.0),
                        Expression::Number(3.0),
                        Expression::List(vec![
                            Expression::Variable("/".to_string()),
                            Expression::Number(3.0),
                            Expression::Number(1.0)
                        ])
                    ])
                ])
            ))
        );

        Ok(())
    }
    #[test]
    fn parse_lisp_symbol_test() -> anyhow::Result<()> {
        let input = "test";
        let result = parse_lisp_symbol(input);

        assert_eq!(result, Ok(("", "test".to_string())));
        Ok(())
    }
}
