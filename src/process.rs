use crate::{errors::LispError, parse::Expression};

pub fn process_expression(expression: &Expression) -> Result<f32, LispError> {
    match expression {
        Expression::Number(data) => Ok(*data),
        Expression::Variable(_) => Err(LispError::ComputerError),
        Expression::List(expressions) => process_expression_list(expressions),
        Expression::String(_) => todo!(),
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
