use super::Function;

pub struct EqualProcessor;
impl Function for EqualProcessor {
    fn process(
        &self,
        args: &[crate::parse::Expression],
        env: &crate::environment::Environment,
    ) -> Result<crate::value::Value, crate::errors::LispComputerError> {
        match args {
            [a, b] => {
                let a = a.eval(env)?;
                let b = b.eval(env)?;
                Ok(crate::value::Value::Boolean(a == b))
            }
            args => Err(crate::errors::LispComputerError::ArityMismatch(
                self.name().to_string(),
                2,
                args.len(),
            )),
        }
    }

    fn name(&self) -> &str {
        "="
    }
}
