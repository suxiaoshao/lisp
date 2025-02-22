use crate::{environment::Environment, errors::LispComputerError, parse::Expression, value::Value};

use super::{Function, process_expression};

pub struct AdditionProcessor;

impl Function for AdditionProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        let mut sum = 0.0;
        let mut result_string = String::new();

        for arg in args {
            match process_expression(arg, env)? {
                Value::Number(n) => {
                    if result_string.is_empty() {
                        sum += n;
                    } else {
                        return Err(LispComputerError::TypeMismatch2 {
                            operation: self.name().to_string(),
                            left: Value::String(result_string),
                            right: Value::Number(n),
                        });
                    }
                }
                Value::String(s) => {
                    if sum == 0.0 {
                        result_string.push_str(&s);
                    } else {
                        return Err(LispComputerError::TypeMismatch2 {
                            operation: self.name().to_string(),
                            left: Value::Number(sum),
                            right: Value::String(s.to_string()),
                        });
                    }
                }
                _ => unimplemented!(),
            }
        }

        if !result_string.is_empty() {
            Ok(Value::String(result_string))
        } else {
            Ok(Value::Number(sum))
        }
    }

    fn name(&self) -> &str {
        "+"
    }
}
