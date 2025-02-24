use super::{Function, process_expression};

pub struct DefineProcessor;

impl Function for DefineProcessor {
    fn process(
        &self,
        args: &[crate::parse::Expression],
        env: &crate::environment::Environment,
    ) -> Result<crate::value::Value, crate::errors::LispComputerError> {
        match args {
            [crate::parse::Expression::Variable(name), value] => {
                let value = process_expression(value, env)?;
                env.set_variable(name.to_string(), value);
                Ok(crate::value::Value::Nil)
            }
            _ => Err(crate::errors::LispComputerError::InvalidArguments(
                "define".to_string(),
                args.to_vec(),
            )),
        }
    }

    fn name(&self) -> &str {
        "define"
    }
}
