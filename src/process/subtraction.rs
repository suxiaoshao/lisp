use crate::{environment::Environment, errors::LispComputerError, parse::Expression, value::Value};

use super::{Function, process_expression};

pub struct SubtractionProcessor;

impl Function for SubtractionProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        if let Some((first, rest)) = args.split_first() {
            let initial_value = match process_expression(first, env)? {
                Value::Number(value) => value,
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
                        left: other,
                    });
                }
            };
            let value = rest.iter().try_fold(initial_value, |acc, expr| {
                let value = process_expression(expr, env)?;
                match value {
                    Value::Number(num) => Ok(acc - num),
                    other => Err(LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
                        left: other,
                    }),
                }
            })?;
            Ok(Value::Number(value))
        } else {
            Err(LispComputerError::TypeMismatch1 {
                operation: self.name().to_string(),
                left: Value::Nil,
            })
        }
    }

    fn name(&self) -> &str {
        "-"
    }
}
