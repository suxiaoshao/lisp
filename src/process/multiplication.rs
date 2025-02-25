use crate::{environment::Environment, value::Value};

use super::Function;

pub struct MultiplicationProcessor;

impl Function for MultiplicationProcessor {
    fn process(
        &self,
        args: &[crate::parse::Expression],
        env: &Environment,
    ) -> Result<crate::value::Value, crate::errors::LispComputerError> {
        let mut result = 1.0;
        for arg in args {
            match arg.eval(env)? {
                Value::Number(num) => result *= num,
                other => {
                    return Err(crate::errors::LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
                        left: other,
                    });
                }
            }
        }
        Ok(crate::value::Value::Number(result))
    }

    fn name(&self) -> &str {
        "*"
    }
}
