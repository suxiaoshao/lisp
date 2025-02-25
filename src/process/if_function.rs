use super::Function;

pub struct IfProcessor;

impl Function for IfProcessor {
    fn process(
        &self,
        args: &[crate::parse::Expression],
        env: &crate::environment::Environment,
    ) -> Result<crate::value::Value, crate::errors::LispComputerError> {
        match args {
            [condition, then_branch, else_branch] => {
                let condition = condition.eval(env)?.boolean();
                match condition {
                    true => then_branch.eval(env),
                    false => else_branch.eval(env),
                }
            }
            _ => Err(crate::errors::LispComputerError::ArityMismatch(
                self.name().to_string(),
                3,
                args.len(),
            )),
        }
    }

    fn name(&self) -> &str {
        "if"
    }
}
