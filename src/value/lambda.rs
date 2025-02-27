use std::collections::HashMap;

use crate::{
    environment::Environment,
    errors::LispComputerError,
    parse::Expression,
    process::{Function, process_expression_list},
};

use super::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Lambda {
    params: Vec<String>,
    body: Vec<Expression>,
}

impl Lambda {
    pub fn new(params: Vec<String>, body: Vec<Expression>) -> Self {
        Lambda { params, body }
    }
}

impl<T: Environment> Function<T> for Lambda {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<super::Value, LispComputerError> {
        if args.len() != self.params.len() {
            return Err(LispComputerError::ArityMismatch(
                <Lambda as Function<T>>::name(self).to_string(),
                self.params.len(),
                args.len(),
            ));
        }
        let mut new_variables = variables.clone();
        for (param, arg) in self.params.iter().zip(args) {
            new_variables.insert(param.as_str(), arg.eval(env, variables)?);
        }
        process_expression_list(&self.body, env, &new_variables)
    }

    fn name(&self) -> &str {
        "lambda-function"
    }
}
