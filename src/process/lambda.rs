use crate::{
    errors::LispComputerError,
    parse::Expression,
    value::{Lambda, Value},
};

use super::Function;

pub struct LambdaProcessor;
impl Function for LambdaProcessor {
    fn process(
        &self,
        args: &[crate::parse::Expression],
        _env: &crate::environment::Environment,
    ) -> Result<crate::value::Value, crate::errors::LispComputerError> {
        match args {
            [Expression::List(params), Expression::List(body)] => {
                let params = params
                    .iter()
                    .map(|param| match param {
                        Expression::Variable(name) => Ok(name.clone()),
                        _ => Err(LispComputerError::InvalidArguments(
                            "lambda-params".to_string(),
                            params.clone(),
                        )),
                    })
                    .collect::<Result<Vec<String>, LispComputerError>>()?;

                let body = body.clone();

                Ok(Value::Lambda(Lambda::new(params, body)))
            }
            _ => Err(LispComputerError::InvalidArguments(
                self.name().to_string(),
                args.to_vec(),
            )),
        }
    }

    fn name(&self) -> &str {
        "lambda"
    }
}
