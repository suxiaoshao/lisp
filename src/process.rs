use crate::{environment::Environment, errors::LispComputerError, parse::Expression, value::Value};

mod addition;
mod define;
mod division;
mod equal;
mod if_function;
mod lambda;
mod multiplication;
mod subtraction;

pub use addition::AdditionProcessor;
pub use define::DefineProcessor;
pub use division::DivisionProcessor;
pub use equal::EqualProcessor;
pub use if_function::IfProcessor;
pub use lambda::LambdaProcessor;
pub use multiplication::MultiplicationProcessor;
pub use subtraction::SubtractionProcessor;

pub trait Function {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError>;
    fn name(&self) -> &str;
}

pub fn process_expression_list(
    expressions: &[Expression],
    env: &Environment,
) -> Result<Value, LispComputerError> {
    match expressions {
        [] => Ok(Value::Nil),
        [Expression::Number(data)] => Ok(Value::Number(*data)),
        [Expression::Variable(symbol), tail @ ..] => process_variable(symbol, tail, env),
        _ => unimplemented!(),
    }
}

fn process_variable(
    symbol: &str,
    args: &[Expression],
    env: &Environment,
) -> Result<Value, LispComputerError> {
    env.process_variable(symbol, args)
}
