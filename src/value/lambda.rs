use std::collections::HashMap;

use crate::{
    parse::Expression,
    process::{Function, process_expression_list},
};

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

impl Function for Lambda {
    fn process(
        &self,
        args: &[Expression],
        env: &crate::environment::Environment,
    ) -> Result<super::Value, crate::errors::LispComputerError> {
        if args.len() != self.params.len() {
            return Err(crate::errors::LispComputerError::ArityMismatch(
                self.name().to_string(),
                self.params.len(),
                args.len(),
            ));
        }
        let mut variables = HashMap::new();
        for (param, arg) in self.params.iter().zip(args) {
            variables.insert(param.clone(), arg.eval(env)?);
        }
        let new_env = env.new_child(variables);
        process_expression_list(&self.body, &new_env)
    }

    fn name(&self) -> &str {
        "lambda-function"
    }
}
