use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{multispace0, multispace1, none_of, one_of},
    combinator::{map, not, peek, recognize},
    multi::{many1, separated_list0},
    number::complete::double,
    sequence::delimited,
};
use string::parse_string;

mod string;

#[derive(Debug, PartialEq)]
pub enum Expression {
    Number(f64),
    Variable(String),
    List(Vec<Expression>),
    String(String),
}

pub fn parse_expression(input: &str) -> IResult<&str, Expression> {
    let (input, data) = alt((
        map(double, Expression::Number),
        map(
            (tag("("), parse_expression_inner, tag(")")),
            |(_, data, _)| Expression::List(data),
        ),
        map(parse_lisp_variable, Expression::Variable),
        map(parse_string, Expression::String),
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

fn parse_lisp_variable(input: &str) -> IResult<&str, String> {
    let valid_char = none_of(" \t\n\r()\"");
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

        // test string
        let input = "\"hello\"";
        let result = parse_expression(input);

        assert_eq!(result, Ok(("", Expression::String("hello".to_string()))));

        Ok(())
    }
    #[test]
    fn parse_lisp_symbol_test() -> anyhow::Result<()> {
        let input = "test";
        let result = parse_lisp_variable(input);

        assert_eq!(result, Ok(("", "test".to_string())));
        Ok(())
    }
}
