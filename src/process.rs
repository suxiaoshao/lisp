use std::collections::HashMap;

use crate::{
    environment::Environment,
    errors::LispComputerError,
    parse::Expression,
    value::{Lambda, Value},
};

pub trait Function<T>
where
    T: Environment,
{
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError>;
    fn name(&self) -> &str;
}

pub fn process_expression_list<T: Environment>(
    expressions: &[Expression],
    env: &T,
    variables: &HashMap<&str, Value>,
) -> Result<Value, LispComputerError> {
    match expressions {
        [] => Ok(Value::Nil),
        [Expression::Number(data)] => Ok(Value::Number(*data)),
        [Expression::Variable(symbol), tail @ ..] => process_variable(symbol, tail, env, variables),
        [Expression::List(list), tail @ ..] => {
            if let Value::Lambda(func) = process_expression_list(list, env, variables)? {
                func.process(tail, env, variables)
            } else {
                unimplemented!()
            }
        }
        _ => unimplemented!(),
    }
}

fn process_variable<T: Environment>(
    symbol: &str,
    args: &[Expression],
    env: &T,
    variables: &HashMap<&str, Value>,
) -> Result<Value, LispComputerError> {
    env.process_variable(symbol, args, variables)
}

pub struct AdditionProcessor;

impl<T: Environment> Function<T> for AdditionProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        let mut sum = 0.0;
        let mut result_string = String::new();

        for arg in args {
            match arg.eval(env, variables)? {
                Value::Number(n) => {
                    if result_string.is_empty() {
                        sum += n;
                    } else {
                        return Err(LispComputerError::TypeMismatch2 {
                            operation: <AdditionProcessor as Function<T>>::name(self).to_string(),
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
                            operation: <AdditionProcessor as Function<T>>::name(self).to_string(),
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

pub struct DivisionProcessor;

impl<T: Environment> Function<T> for DivisionProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        if let Some((first, rest)) = args.split_first() {
            let initial_value = match first.eval(env, variables)? {
                Value::Number(n) => n,
                value => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <DivisionProcessor as Function<T>>::name(self).to_string(),
                        left: value,
                    });
                }
            };
            let value = rest.iter().try_fold(initial_value, |acc, expr| {
                let value = expr.eval(env, variables)?;
                match value {
                    Value::Number(n) => Ok(acc / n),
                    value => Err(LispComputerError::TypeMismatch2 {
                        operation: <DivisionProcessor as Function<T>>::name(self).to_string(),
                        left: Value::Number(acc),
                        right: value,
                    }),
                }
            })?;
            Ok(Value::Number(value))
        } else {
            Err(LispComputerError::TypeMismatch1 {
                operation: <DivisionProcessor as Function<T>>::name(self).to_string(),
                left: Value::Nil,
            })
        }
    }
    fn name(&self) -> &str {
        "/"
    }
}

pub struct MultiplicationProcessor;

impl<T: Environment> Function<T> for MultiplicationProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        let mut result = 1.0;
        for arg in args {
            match arg.eval(env, variables)? {
                Value::Number(num) => result *= num,
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <MultiplicationProcessor as Function<T>>::name(self).to_string(),
                        left: other,
                    });
                }
            }
        }
        Ok(Value::Number(result))
    }

    fn name(&self) -> &str {
        "*"
    }
}

pub struct SubtractionProcessor;

impl<T: Environment> Function<T> for SubtractionProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        if let Some((first, rest)) = args.split_first() {
            let initial_value = match first.eval(env, variables)? {
                Value::Number(value) => value,
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <SubtractionProcessor as Function<T>>::name(self).to_string(),
                        left: other,
                    });
                }
            };
            let value = rest.iter().try_fold(initial_value, |acc, expr| {
                let value = expr.eval(env, variables)?;
                match value {
                    Value::Number(num) => Ok(acc - num),
                    other => Err(LispComputerError::TypeMismatch1 {
                        operation: <SubtractionProcessor as Function<T>>::name(self).to_string(),
                        left: other,
                    }),
                }
            })?;
            Ok(Value::Number(value))
        } else {
            Err(LispComputerError::TypeMismatch1 {
                operation: <SubtractionProcessor as Function<T>>::name(self).to_string(),
                left: Value::Nil,
            })
        }
    }

    fn name(&self) -> &str {
        "-"
    }
}

pub struct EqualProcessor;
impl<T: Environment> Function<T> for EqualProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::ArityMismatch(
                <EqualProcessor as Function<T>>::name(self).to_string(),
                2,
                args.len(),
            ));
        }

        let mut evaluated_args = Vec::new();
        for arg in args {
            evaluated_args.push(arg.eval(env, variables)?);
        }

        for pair in evaluated_args.windows(2) {
            if pair[0] != pair[1] {
                return Ok(Value::Boolean(false));
            }
        }

        Ok(Value::Boolean(true))
    }
    fn name(&self) -> &str {
        "="
    }
}

pub struct GreaterThanProcessor;
impl<T: Environment> Function<T> for GreaterThanProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::ArityMismatch(
                <GreaterThanProcessor as Function<T>>::name(self).to_string(),
                2,
                args.len(),
            ));
        }

        let mut evaluated_args = Vec::new();
        for arg in args {
            match arg.eval(env, variables)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <GreaterThanProcessor as Function<T>>::name(self).to_string(),
                        left: other,
                    });
                }
            }
        }

        for pair in evaluated_args.windows(2) {
            if pair[0] <= pair[1] {
                return Ok(Value::Boolean(false));
            }
        }

        Ok(Value::Boolean(true))
    }

    fn name(&self) -> &str {
        ">"
    }
}

pub struct LessThanProcessor;

impl<T: Environment> Function<T> for LessThanProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::InvalidArguments(
                <LessThanProcessor as Function<T>>::name(self).to_string(),
                args.to_vec(),
            ));
        }

        let mut evaluated_args = Vec::new();
        for arg in args {
            match arg.eval(env, variables)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <LessThanProcessor as Function<T>>::name(self).to_string(),
                        left: other,
                    });
                }
            }
        }

        for pair in evaluated_args.windows(2) {
            if pair[0] >= pair[1] {
                return Ok(Value::Boolean(false));
            }
        }

        Ok(Value::Boolean(true))
    }

    fn name(&self) -> &str {
        "<"
    }
}

pub struct GreaterEqualProcessor;

impl<T: Environment> Function<T> for GreaterEqualProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::InvalidArguments(
                <GreaterEqualProcessor as Function<T>>::name(self).to_string(),
                args.to_vec(),
            ));
        }
        let mut evaluated_args = Vec::new();

        for arg in args {
            match arg.eval(env, variables)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <GreaterEqualProcessor as Function<T>>::name(self).to_string(),
                        left: other,
                    });
                }
            }
        }
        for pair in evaluated_args.windows(2) {
            if pair[0] < pair[1] {
                return Ok(Value::Boolean(false));
            }
        }
        Ok(Value::Boolean(true))
    }

    fn name(&self) -> &str {
        ">="
    }
}

pub struct LessEqualProcessor;

impl<T: Environment> Function<T> for LessEqualProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::InvalidArguments(
                <LessEqualProcessor as Function<T>>::name(self).to_string(),
                args.to_vec(),
            ));
        }
        let mut evaluated_args = Vec::new();
        for arg in args {
            match arg.eval(env, variables)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <LessEqualProcessor as Function<T>>::name(self).to_string(),
                        left: other,
                    });
                }
            }
        }
        for pair in evaluated_args.windows(2) {
            if pair[0] > pair[1] {
                return Ok(Value::Boolean(false));
            }
        }
        Ok(Value::Boolean(true))
    }

    fn name(&self) -> &str {
        "<="
    }
}

pub struct IfProcessor;

impl<T: Environment> Function<T> for IfProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        match args {
            [condition, then_branch, else_branch] => {
                let condition = condition.eval(env, variables)?.boolean();
                match condition {
                    true => then_branch.eval(env, variables),
                    false => else_branch.eval(env, variables),
                }
            }
            _ => Err(LispComputerError::ArityMismatch(
                <IfProcessor as Function<T>>::name(self).to_string(),
                3,
                args.len(),
            )),
        }
    }

    fn name(&self) -> &str {
        "if"
    }
}

pub struct OrProcessor;

impl<T: Environment> Function<T> for OrProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        let mut last_value = Value::Boolean(false);

        for arg in args {
            let value = arg.eval(env, variables)?;
            if value.boolean() {
                return Ok(value);
            }
            last_value = value;
        }

        Ok(last_value)
    }

    fn name(&self) -> &str {
        "or"
    }
}

pub struct AndProcessor;

impl<T: Environment> Function<T> for AndProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        let mut last_value = Value::Boolean(true);

        for arg in args {
            let value = arg.eval(env, variables)?;
            if !value.boolean() {
                return Ok(value);
            }
            last_value = value;
        }

        Ok(last_value)
    }

    fn name(&self) -> &str {
        "and"
    }
}

pub struct CondProcessor;

impl<T: Environment> Function<T> for CondProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        if let Some((last, args)) = args.split_last() {
            for arg in args {
                if let Expression::List(inner_args) = arg {
                    if let [condition, result] = inner_args.as_slice() {
                        let condition_value = condition.eval(env, variables)?;
                        if condition_value.boolean() {
                            return result.eval(env, variables);
                        }
                    }
                }
            }
            if let Expression::List(inner_args) = last {
                if let [Expression::Variable(name), result] = inner_args.as_slice() {
                    if name == "else" {
                        return result.eval(env, variables);
                    }
                }
            }
        }
        Err(LispComputerError::InvalidArguments(
            <CondProcessor as Function<T>>::name(self).to_string(),
            args.to_vec(),
        ))
    }

    fn name(&self) -> &str {
        "cond"
    }
}

pub struct DefineProcessor;

impl<T: Environment> Function<T> for DefineProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        match args {
            [Expression::Variable(name), value] => {
                let value = value.eval(env, variables)?;
                env.set_variable(name.to_string(), value);
                Ok(Value::Nil)
            }
            [Expression::List(params), Expression::List(body)] => match params.as_slice() {
                [Expression::Variable(name), tail @ ..] => {
                    let params = tail
                        .iter()
                        .map(|param| match param {
                            Expression::Variable(name) => Ok(name.clone()),
                            _ => Err(LispComputerError::InvalidArguments(
                                "lambda-params".to_string(),
                                params.clone(),
                            )),
                        })
                        .collect::<Result<Vec<String>, LispComputerError>>()?;
                    let lambda = Lambda::new(params, body.clone());
                    env.set_variable(name.to_string(), Value::Lambda(lambda));
                    Ok(Value::Nil)
                }
                _ => Err(LispComputerError::InvalidArguments(
                    <DefineProcessor as Function<T>>::name(self).to_string(),
                    args.to_vec(),
                )),
            },
            _ => Err(LispComputerError::InvalidArguments(
                <DefineProcessor as Function<T>>::name(self).to_string(),
                args.to_vec(),
            )),
        }
    }

    fn name(&self) -> &str {
        "define"
    }
}

pub struct LambdaProcessor;
impl<T: Environment> Function<T> for LambdaProcessor {
    fn process(
        &self,
        args: &[Expression],
        _env: &T,
        _variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
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
                <LambdaProcessor as Function<T>>::name(self).to_string(),
                args.to_vec(),
            )),
        }
    }

    fn name(&self) -> &str {
        "lambda"
    }
}

pub struct LetProcessor;
impl<T: Environment> Function<T> for LetProcessor {
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        match args {
            [Expression::List(bindings), body] => {
                let mut new_variables = variables.clone();
                for binding in bindings {
                    match binding {
                        Expression::List(binding) => match binding.as_slice() {
                            [Expression::Variable(name), value] => {
                                new_variables.insert(name.as_str(), value.eval(env, variables)?);
                            }
                            _ => {
                                return Err(LispComputerError::InvalidArguments(
                                    "let-bindings".to_string(),
                                    binding.clone(),
                                ));
                            }
                        },
                        _ => {
                            return Err(LispComputerError::InvalidArguments(
                                "let-bindings".to_string(),
                                bindings.clone(),
                            ));
                        }
                    }
                }

                body.eval(env, &new_variables)
            }
            _ => Err(LispComputerError::InvalidArguments(
                <LetProcessor as Function<T>>::name(self).to_string(),
                args.to_vec(),
            )),
        }
    }

    fn name(&self) -> &str {
        "let"
    }
}
