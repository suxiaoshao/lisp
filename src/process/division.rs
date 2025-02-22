use crate::{environment::Environment, errors::LispComputerError, value::Value};

use super::{Function, process_expression};

pub struct DivisionProcessor;

impl Function for DivisionProcessor {
    fn process(
        &self,
        args: &[crate::parse::Expression],
        env: &Environment,
    ) -> Result<Value, LispComputerError> {
        if let Some((first, rest)) = args.split_first() {
            let initial_value = match process_expression(first, env)? {
                Value::Number(n) => n,
                value => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
                        left: value,
                    });
                }
            };
            let value = rest.iter().try_fold(initial_value, |acc, expr| {
                let value = process_expression(expr, env)?;
                match value {
                    Value::Number(n) => Ok(acc / n),
                    value => Err(LispComputerError::TypeMismatch2 {
                        operation: self.name().to_string(),
                        left: Value::Number(acc),
                        right: value,
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
        "/"
    }
}
