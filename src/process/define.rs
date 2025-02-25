use super::Function;

pub struct DefineProcessor;

impl Function for DefineProcessor {
    fn process(
        &self,
        args: &[crate::parse::Expression],
        env: &crate::environment::Environment,
    ) -> Result<crate::value::Value, crate::errors::LispComputerError> {
        match args {
            [crate::parse::Expression::Variable(name), value] => {
                let value = value.eval(env)?;
                env.set_variable(name.to_string(), value);
                Ok(crate::value::Value::Nil)
            }
            _ => Err(crate::errors::LispComputerError::InvalidArguments(
                self.name().to_string(),
                args.to_vec(),
            )),
        }
    }

    fn name(&self) -> &str {
        "define"
    }
}
