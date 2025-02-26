use crate::{
    environment::Environment,
    errors::LispComputerError,
    parse::Expression,
    value::{Lambda, Value},
};

pub trait Function {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError>;
    fn name(&self) -> &str;
}

pub fn process_expression_list(
    expressions: &[Expression],
    env: &Environment,
) -> Result<Value, LispComputerError> {
    match expressions {
        [] => Ok(Value::Nil),
        [Expression::Number(data)] => Ok(Value::Number(*data)),
        [Expression::Variable(symbol), tail @ ..] => process_variable(symbol, tail, env),
        [Expression::List(list), tail @ ..] => {
            if let Value::Lambda(func) = process_expression_list(list, env)? {
                func.process(tail, env)
            } else {
                unimplemented!()
            }
        }
        _ => unimplemented!(),
    }
}

fn process_variable(
    symbol: &str,
    args: &[Expression],
    env: &Environment,
) -> Result<Value, LispComputerError> {
    env.process_variable(symbol, args)
}

pub struct AdditionProcessor;

impl Function for AdditionProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        let mut sum = 0.0;
        let mut result_string = String::new();

        for arg in args {
            match arg.eval(env)? {
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

pub struct DivisionProcessor;

impl Function for DivisionProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        if let Some((first, rest)) = args.split_first() {
            let initial_value = match first.eval(env)? {
                Value::Number(n) => n,
                value => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
                        left: value,
                    });
                }
            };
            let value = rest.iter().try_fold(initial_value, |acc, expr| {
                let value = expr.eval(env)?;
                match value {
                    Value::Number(n) => Ok(acc / n),
                    value => Err(LispComputerError::TypeMismatch2 {
                        operation: self.name().to_string(),
                        left: Value::Number(acc),
                        right: value,
                    }),
                }
            })?;
            Ok(Value::Number(value))
        } else {
            Err(LispComputerError::TypeMismatch1 {
                operation: self.name().to_string(),
                left: Value::Nil,
            })
        }
    }
    fn name(&self) -> &str {
        "/"
    }
}

pub struct MultiplicationProcessor;

impl Function for MultiplicationProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        let mut result = 1.0;
        for arg in args {
            match arg.eval(env)? {
                Value::Number(num) => result *= num,
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
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

impl Function for SubtractionProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        if let Some((first, rest)) = args.split_first() {
            let initial_value = match first.eval(env)? {
                Value::Number(value) => value,
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
                        left: other,
                    });
                }
            };
            let value = rest.iter().try_fold(initial_value, |acc, expr| {
                let value = expr.eval(env)?;
                match value {
                    Value::Number(num) => Ok(acc - num),
                    other => Err(LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
                        left: other,
                    }),
                }
            })?;
            Ok(Value::Number(value))
        } else {
            Err(LispComputerError::TypeMismatch1 {
                operation: self.name().to_string(),
                left: Value::Nil,
            })
        }
    }

    fn name(&self) -> &str {
        "-"
    }
}

pub struct EqualProcessor;
impl Function for EqualProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::ArityMismatch(
                self.name().to_string(),
                2,
                args.len(),
            ));
        }

        let mut evaluated_args = Vec::new();
        for arg in args {
            evaluated_args.push(arg.eval(env)?);
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
impl Function for GreaterThanProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::ArityMismatch(
                self.name().to_string(),
                2,
                args.len(),
            ));
        }

        let mut evaluated_args = Vec::new();
        for arg in args {
            match arg.eval(env)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
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

impl Function for LessThanProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::InvalidArguments(
                self.name().to_string(),
                args.to_vec(),
            ));
        }

        let mut evaluated_args = Vec::new();
        for arg in args {
            match arg.eval(env)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
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

impl Function for GreaterEqualProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::InvalidArguments(
                self.name().to_string(),
                args.to_vec(),
            ));
        }
        let mut evaluated_args = Vec::new();

        for arg in args {
            match arg.eval(env)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
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

impl Function for LessEqualProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::InvalidArguments(
                self.name().to_string(),
                args.to_vec(),
            ));
        }
        let mut evaluated_args = Vec::new();
        for arg in args {
            match arg.eval(env)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: self.name().to_string(),
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

pub struct DefineProcessor;

impl Function for DefineProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        match args {
            [Expression::Variable(name), value] => {
                let value = value.eval(env)?;
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
                    self.name().to_string(),
                    args.to_vec(),
                )),
            },
            _ => Err(LispComputerError::InvalidArguments(
                self.name().to_string(),
                args.to_vec(),
            )),
        }
    }

    fn name(&self) -> &str {
        "define"
    }
}

pub struct IfProcessor;

impl Function for IfProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        match args {
            [condition, then_branch, else_branch] => {
                let condition = condition.eval(env)?.boolean();
                match condition {
                    true => then_branch.eval(env),
                    false => else_branch.eval(env),
                }
            }
            _ => Err(LispComputerError::ArityMismatch(
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

pub struct OrProcessor;

impl Function for OrProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        let mut last_value = Value::Boolean(false);

        for arg in args {
            let value = arg.eval(env)?;
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

impl Function for AndProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        let mut last_value = Value::Boolean(true);

        for arg in args {
            let value = arg.eval(env)?;
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

impl Function for CondProcessor {
    fn process(&self, args: &[Expression], env: &Environment) -> Result<Value, LispComputerError> {
        for arg in args {
            match arg {
                Expression::List(inner_args) => match inner_args.as_slice() {
                    [Expression::Variable(name), result] if name == "else" => {
                        return result.eval(env);
                    }
                    [condition, result] => {
                        let condition_value = condition.eval(env)?;
                        if condition_value.boolean() {
                            return result.eval(env);
                        }
                    }
                    _ => {
                        return Err(LispComputerError::InvalidArguments(
                            self.name().to_string(),
                            args.to_vec(),
                        ));
                    }
                },
                _ => {
                    return Err(LispComputerError::InvalidArguments(
                        self.name().to_string(),
                        args.to_vec(),
                    ));
                }
            }
        }

        Ok(Value::Boolean(false))
    }

    fn name(&self) -> &str {
        "cond"
    }
}

pub struct LambdaProcessor;
impl Function for LambdaProcessor {
    fn process(&self, args: &[Expression], _env: &Environment) -> Result<Value, LispComputerError> {
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
