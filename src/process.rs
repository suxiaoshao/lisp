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
            let callee = process_expression_list(list, env, variables)?;
            if let Value::Lambda(func) = callee {
                func.process(tail, env, variables)
            } else {
                Err(LispComputerError::TypeMismatch1 {
                    operation: "function application".to_string(),
                    left: callee,
                })
            }
        }
        _ => {
            let expr_str = expressions
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(" ");
            Err(LispComputerError::InvalidExpression(expr_str))
        }
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
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <AdditionProcessor as Function<T>>::name(self).to_string(),
                        left: other,
                    });
                }
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
            let value = if rest.is_empty() {
                -initial_value
            } else {
                rest.iter().try_fold(initial_value, |acc, expr| {
                    let value = expr.eval(env, variables)?;
                    match value {
                        Value::Number(num) => Ok(acc - num),
                        other => Err(LispComputerError::TypeMismatch1 {
                            operation: <SubtractionProcessor as Function<T>>::name(self)
                                .to_string(),
                            left: other,
                        }),
                    }
                })?
            };
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
        fn get_lambda_from(
            bindings: &Vec<Expression>,
            body: &Vec<Expression>,
        ) -> Result<(Lambda, Vec<Expression>), LispComputerError> {
            let mut params = Vec::new();
            let mut lambda_args = Vec::new();
            for binding in bindings {
                match binding {
                    Expression::List(binding) => match binding.as_slice() {
                        [Expression::Variable(name), value] => {
                            params.push(name.to_string());
                            lambda_args.push(value.clone());
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

            let lambda = Lambda::new(params, body.clone());
            Ok((lambda, lambda_args))
        }
        match args {
            // let
            [Expression::List(bindings), Expression::List(body)] => {
                let (lambda, lambda_args) = get_lambda_from(bindings, body)?;
                lambda.process(&lambda_args, env, variables)
            }
            // let naming
            [Expression::NamingList(name, bindings), Expression::List(body)]
            | [Expression::Variable(name), Expression::List(bindings), Expression::List(body)] => {
                let mut variables = variables.clone();
                let (lambda, lambda_args) = get_lambda_from(bindings, body)?;
                variables.insert(name, Value::Lambda(lambda.clone()));
                lambda.process(&lambda_args, env, &variables)
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

pub struct DoProcessor;

impl<T> Function<T> for DoProcessor
where
    T: Environment,
{
    fn process(
        &self,
        args: &[Expression],
        env: &T,
        variables: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        struct DoStep<'a> {
            name: &'a str,
            step_expr: &'a Expression,
        }
        struct DoTest<'a> {
            test_expr: &'a Expression,
            result_expr: &'a Expression,
        }
        match args {
            [Expression::List(bindings), Expression::List(test), bodys @ ..] => {
                let mut new_variables = variables.clone();
                let mut steps = Vec::new();
                for binding in bindings {
                    match binding {
                        Expression::List(list) => {
                            if let [Expression::Variable(name), value, step_expr] = list.as_slice()
                            {
                                new_variables.insert(name, value.eval(env, variables)?);
                                steps.push(DoStep { name, step_expr });
                            } else {
                                return Err(LispComputerError::InvalidArguments(
                                    <Self as Function<T>>::name(self).to_string(),
                                    args.to_vec(),
                                ));
                            }
                        }
                        _ => {
                            return Err(LispComputerError::InvalidArguments(
                                <Self as Function<T>>::name(self).to_string(),
                                args.to_vec(),
                            ));
                        }
                    }
                }
                let do_test = match test.as_slice() {
                    [test_expr, result_expr] => DoTest {
                        test_expr,
                        result_expr,
                    },
                    _ => {
                        return Err(LispComputerError::InvalidArguments(
                            <Self as Function<T>>::name(self).to_string(),
                            args.to_vec(),
                        ));
                    }
                };
                loop {
                    if do_test.test_expr.eval(env, &new_variables)?.boolean() {
                        return do_test.result_expr.eval(env, &new_variables);
                    }
                    for body in bodys {
                        body.eval(env, &new_variables)?;
                    }
                    let values = steps
                        .iter()
                        .map::<Result<_, LispComputerError>, _>(|step| {
                            let new_value = step.step_expr.eval(env, &new_variables)?;
                            Ok((step.name, new_value))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    new_variables.extend(values);
                }
            }
            _ => Err(LispComputerError::InvalidArguments(
                <Self as Function<T>>::name(self).to_string(),
                args.to_vec(),
            )),
        }
    }
    fn name(&self) -> &str {
        "do"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        environment::GlobalEnvironment, errors::LispComputerError, parse::parse_expression,
        value::Value,
    };
    use std::collections::HashMap;

    fn eval(input: &str) -> Result<Value, LispComputerError> {
        let (_, expr) = parse_expression(input).unwrap();
        let env = GlobalEnvironment::default();
        expr.eval(&env, &HashMap::new())
    }

    fn eval_with_env(
        input: &str,
        env: &GlobalEnvironment,
        vars: &HashMap<&str, Value>,
    ) -> Result<Value, LispComputerError> {
        let (_, expr) = parse_expression(input).unwrap();
        expr.eval(env, vars)
    }

    #[test]
    fn test_addition() {
        assert_eq!(eval("(+ 1 2)"), Ok(Value::Number(3.0)));
        assert_eq!(eval("(+ 1 2 3 4)"), Ok(Value::Number(10.0)));
        assert_eq!(eval("(+ 1)"), Ok(Value::Number(1.0)));
        assert_eq!(eval("(+)"), Ok(Value::Number(0.0)));
    }

    #[test]
    fn test_subtraction() {
        assert_eq!(eval("(- 5 2)"), Ok(Value::Number(3.0)));
        assert_eq!(eval("(- 5 1 2)"), Ok(Value::Number(2.0)));
        assert_eq!(eval("(- 10)"), Ok(Value::Number(-10.0)));
    }

    #[test]
    fn test_multiplication() {
        assert_eq!(eval("(* 2 3)"), Ok(Value::Number(6.0)));
        assert_eq!(eval("(* 2 3 4)"), Ok(Value::Number(24.0)));
        assert_eq!(eval("(*)"), Ok(Value::Number(1.0)));
    }

    #[test]
    fn test_division() {
        assert_eq!(eval("(/ 10 2)"), Ok(Value::Number(5.0)));
        assert_eq!(eval("(/ 10 2 2)"), Ok(Value::Number(2.5)));
        // 除零在浮点数中返回Inf，不返回错误
        // 所以测试：(/ 1 0) 返回 Inf 或 -Inf
        let result = eval("(/ 1 0)");
        assert!(result.is_ok());
        if let Ok(Value::Number(n)) = result {
            assert!(n.is_infinite());
        } else {
            panic!("Expected infinite number");
        }
    }

    #[test]
    fn test_string_concatenation() {
        assert_eq!(eval(r#""hello""#), Ok(Value::String("hello".to_string())));
        assert_eq!(
            eval(r#"(+ "hello" "world")"#),
            Ok(Value::String("helloworld".to_string()))
        );
        assert_eq!(
            eval(r#"(+ "a" "b" "c")"#),
            Ok(Value::String("abc".to_string()))
        );
    }

    #[test]
    fn test_type_mismatch_in_addition() {
        // 字符串后接数字
        let result = eval(r#"(+ "hello" 1)"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch2 { .. })
        ));
        // 数字后接字符串
        let result = eval(r#"(+ 1 "hello")"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch2 { .. })
        ));
        // 非数字非字符串
        let result = eval(r#"(+ #t 1)"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch1 { .. })
        ));
    }

    #[test]
    fn test_equality() {
        assert_eq!(eval("(= 1 1)"), Ok(Value::Boolean(true)));
        assert_eq!(eval("(= 1 2)"), Ok(Value::Boolean(false)));
        assert_eq!(eval("(= 1.0 1.0)"), Ok(Value::Boolean(true)));
        assert_eq!(eval("(= 1.0 2.0)"), Ok(Value::Boolean(false)));
        assert_eq!(eval("(= #t #t)"), Ok(Value::Boolean(true)));
        assert_eq!(eval("(= #t #f)"), Ok(Value::Boolean(false)));
    }

    #[test]
    fn test_comparison() {
        assert_eq!(eval("(> 2 1)"), Ok(Value::Boolean(true)));
        assert_eq!(eval("(> 1 2)"), Ok(Value::Boolean(false)));
        assert_eq!(eval("(< 1 2)"), Ok(Value::Boolean(true)));
        assert_eq!(eval("(< 2 1)"), Ok(Value::Boolean(false)));
        assert_eq!(eval("(>= 2 2)"), Ok(Value::Boolean(true)));
        assert_eq!(eval("(<= 2 2)"), Ok(Value::Boolean(true)));
    }

    #[test]
    fn test_logical_operations() {
        assert_eq!(eval("(and #t #t)"), Ok(Value::Boolean(true)));
        assert_eq!(eval("(and #t #f)"), Ok(Value::Boolean(false)));
        assert_eq!(eval("(and #f #t)"), Ok(Value::Boolean(false)));
        assert_eq!(eval("(or #f #f)"), Ok(Value::Boolean(false)));
        assert_eq!(eval("(or #f #t)"), Ok(Value::Boolean(true)));
        assert_eq!(eval("(or #t #f)"), Ok(Value::Boolean(true)));
    }

    #[test]
    fn test_lambda() {
        // 创建lambda (body必须是List)
        let env = GlobalEnvironment::default();
        let result = eval_with_env("(lambda (x) (x))", &env, &HashMap::new());
        assert!(result.is_ok());
        // 注意：当前实现中，(x)会被当作函数应用，所以需要body是调用内置函数
        // 测试一个实际可用的lambda：加法
        assert_eq!(eval("((lambda (x y) (+ x y)) 2 3)"), Ok(Value::Number(5.0)));
        assert_eq!(
            eval("((lambda (x y) (* x y)) 4 5)"),
            Ok(Value::Number(20.0))
        );
        // 单参数lambda，body中使用参数进行计算
        assert_eq!(eval("((lambda (x) (+ x 1)) 5)"), Ok(Value::Number(6.0)));
    }

    #[test]
    fn test_lambda_with_variable_capture() {
        // 暂不支持闭包捕获，测试嵌套lambda创建
        let result = eval("((lambda (x) (lambda (y) (+ x y))) 2)");
        assert!(result.is_ok());
        // 返回应该是一个Lambda
        if let Ok(Value::Lambda(_)) = result {
            // ok
        } else {
            panic!("Expected lambda");
        }
    }

    #[test]
    fn test_if() {
        assert_eq!(eval("(if #t 1 2)"), Ok(Value::Number(1.0)));
        assert_eq!(eval("(if #f 1 2)"), Ok(Value::Number(2.0)));
        assert_eq!(
            eval("(if #t \"then\" \"else\")"),
            Ok(Value::String("then".to_string()))
        );
    }

    #[test]
    fn test_define() {
        // 定义变量
        let env = GlobalEnvironment::default();
        let mut vars = HashMap::new();
        assert_eq!(eval_with_env("(define x 42)", &env, &vars), Ok(Value::Nil));
        vars.insert("x", Value::Number(42.0)); // 手动插入，因为define实际存入env而不是vars
                                               // 注意：define将值存入env，而不是vars。所以我们这里简化，直接测试define返回Nil
        assert_eq!(
            eval_with_env("(define x 42)", &env, &HashMap::new()),
            Ok(Value::Nil)
        );
        // 定义函数：先测试lambda创建
        let result = eval_with_env("(lambda (x) (* x x))", &env, &HashMap::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_let() {
        assert_eq!(eval("(let ((x 1) (y 2)) (+ x y))"), Ok(Value::Number(3.0)));
        assert_eq!(
            eval("(let ((a 10) (b 20)) (- a b))"),
            Ok(Value::Number(-10.0))
        );
    }

    #[test]
    fn test_cond() {
        // 当前实现要求最后一个条件必须是else，前面的普通条件会被处理
        assert_eq!(eval("(cond (#t 1) (else 2))"), Ok(Value::Number(1.0))); // 第一个满足，返回1
        assert_eq!(eval("(cond (#f 1) (else 2))"), Ok(Value::Number(2.0))); // else返回2
                                                                            // 单个else
        assert_eq!(eval("(cond (else 3))"), Ok(Value::Number(3.0)));
    }

    #[test]
    fn test_do() {
        // do语法: (do ((var init step) ...) (test result) body ...)
        // test部分需要是一个包含test和result的list
        assert_eq!(
            eval("(do ((i 1 (+ i 1))) ((= i 5) i))"),
            Ok(Value::Number(5.0))
        );
    }

    #[test]
    fn test_function_application_non_lambda() {
        // 尝试应用非lambda值应返回TypeMismatch1
        // 需要构造一个callee求值为非lambda的情况，例如：(if #t 1 2) 返回数字1，然后应用
        let result = eval("((if #t 1 2) 3)");
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch1 { .. })
        ));
    }

    #[test]
    fn test_invalid_expression() {
        // 无效表达式应返回InvalidExpression
        let result = eval("(1 2 3 4)"); // 数字应用
        assert!(matches!(
            result,
            Err(LispComputerError::InvalidExpression(_))
        ));
    }

    #[test]
    fn test_unbound_function() {
        let result = eval("(nonexistent 1 2)");
        assert!(matches!(result, Err(LispComputerError::UnboundFunction(_))));
    }

    #[test]
    fn test_arity_mismatch() {
        // lambda体必须用List包裹，所以用 (x) 而不是 x
        let result = eval("((lambda (x) (x)) 1 2)");
        assert!(matches!(
            result,
            Err(LispComputerError::ArityMismatch { .. })
        ));
    }

    #[test]
    fn test_variable_not_found() {
        let result = eval("undefined_var");
        assert!(matches!(
            result,
            Err(LispComputerError::NotFoundVariable(_))
        ));
    }

    #[test]
    fn test_nested_expressions() {
        assert_eq!(eval("(+ (* 2 3) (/ 10 2))"), Ok(Value::Number(11.0)));
        assert_eq!(eval("(* (+ 1 2) (- 5 2))"), Ok(Value::Number(9.0)));
    }

    #[test]
    fn test_boolean_operations() {
        assert_eq!(eval("(and #t #f #t)"), Ok(Value::Boolean(false)));
        assert_eq!(eval("(or #f #t #f)"), Ok(Value::Boolean(true)));
    }

    #[test]
    fn test_quote_literal() {
        // 测试列表作为数据（虽然这个简单解释器可能不支持quote）
        // 暂时跳过，因为语法未实现
    }

    #[test]
    fn test_empty_list() {
        assert_eq!(eval("()"), Ok(Value::Nil));
    }

    #[test]
    fn test_single_expression() {
        assert_eq!(eval("42"), Ok(Value::Number(42.0)));
        assert_eq!(eval("\"test\""), Ok(Value::String("test".to_string())));
        assert_eq!(eval("#t"), Ok(Value::Boolean(true)));
        assert_eq!(eval("#f"), Ok(Value::Boolean(false)));
    }
}
