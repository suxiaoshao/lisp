use std::collections::{HashMap, HashSet};

use crate::{
    environment::Environment,
    errors::LispComputerError,
    parse::Expression,
    value::{Lambda, Value},
};
use gc_arena::{lock::RefLock, Gc, Mutation};

pub trait Function<'gc, T: Environment<'gc>> {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError>;
    fn name(&self) -> &str;
}

pub fn process_expression_list<'gc, T: Environment<'gc>>(
    expressions: &[Gc<'gc, Expression<'gc>>],
    env: &T,
    variables: &HashMap<String, Value<'gc>>,
    mc: &'gc Mutation<'gc>,
) -> Result<Value<'gc>, LispComputerError> {
    match expressions {
        [] => Ok(Value::Nil),
        [callee, tail @ ..] => match &**callee {
            Expression::Variable(name) => {
                return env.process_variable(name, tail, variables, mc);
            }
            Expression::List(_) => {
                let callee_value = callee.eval(env, variables, mc)?;
                if let Value::Lambda(func) = callee_value {
                    func.process(tail, env, variables, mc)
                } else {
                    Err(LispComputerError::TypeMismatch1 {
                        operation: "function application".to_string(),
                        left_str: format!("{}", callee_value),
                    })
                }
            }
            _ => Err(LispComputerError::InvalidExpression(format!(
                "{} is not a valid function expression",
                callee
            ))),
        },
    }
}

// Processors

pub struct AdditionProcessor;

impl<'gc, T: Environment<'gc>> Function<'gc, T> for AdditionProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        let mut sum = 0.0;
        let mut result_string = String::new();
        let mut saw_number = false;
        let mut saw_string = false;

        for arg in args {
            match arg.eval(env, variables, mc)? {
                Value::Number(n) => {
                    if !saw_string {
                        saw_number = true;
                        sum += n;
                    } else {
                        return Err(LispComputerError::TypeMismatch2 {
                            operation: <Self as Function<'gc, T>>::name(self).to_string(),
                            left_str: result_string.clone(),
                            right_str: n.to_string(),
                        });
                    }
                }
                Value::String(s) => {
                    if !saw_number {
                        saw_string = true;
                        result_string.push_str(&s);
                    } else {
                        return Err(LispComputerError::TypeMismatch2 {
                            operation: <Self as Function<'gc, T>>::name(self).to_string(),
                            left_str: sum.to_string(),
                            right_str: s.to_string(),
                        });
                    }
                }
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <Self as Function<'gc, T>>::name(self).to_string(),
                        left_str: format!("{}", other),
                    });
                }
            }
        }

        if !result_string.is_empty() {
            Ok(Value::String(Gc::new(mc, result_string)))
        } else {
            Ok(Value::Number(sum))
        }
    }

    fn name(&self) -> &str {
        "+"
    }
}

pub struct SubtractionProcessor;

impl<'gc, T: Environment<'gc>> Function<'gc, T> for SubtractionProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if let Some((first, rest)) = args.split_first() {
            let initial_value = match first.eval(env, variables, mc)? {
                Value::Number(n) => n,
                value => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <Self as Function<'gc, T>>::name(self).to_string(),
                        left_str: format!("{}", value),
                    });
                }
            };
            let value = if rest.is_empty() {
                -initial_value
            } else {
                rest.iter().try_fold(initial_value, |acc, expr| {
                    let value = expr.eval(env, variables, mc)?;
                    match value {
                        Value::Number(num) => Ok(acc - num),
                        other => Err(LispComputerError::TypeMismatch1 {
                            operation: <Self as Function<'gc, T>>::name(self).to_string(),
                            left_str: format!("{}", other),
                        }),
                    }
                })?
            };
            Ok(Value::Number(value))
        } else {
            Err(LispComputerError::TypeMismatch1 {
                operation: <Self as Function<'gc, T>>::name(self).to_string(),
                left_str: "nil".to_string(),
            })
        }
    }

    fn name(&self) -> &str {
        "-"
    }
}

pub struct MultiplicationProcessor;

impl<'gc, T: Environment<'gc>> Function<'gc, T> for MultiplicationProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        let mut result = 1.0;
        for arg in args {
            match arg.eval(env, variables, mc)? {
                Value::Number(num) => result *= num,
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <Self as Function<'gc, T>>::name(self).to_string(),
                        left_str: format!("{}", other),
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

pub struct DivisionProcessor;

impl<'gc, T: Environment<'gc>> Function<'gc, T> for DivisionProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if let Some((first, rest)) = args.split_first() {
            let initial_value = match first.eval(env, variables, mc)? {
                Value::Number(n) => n,
                value => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <Self as Function<'gc, T>>::name(self).to_string(),
                        left_str: format!("{}", value),
                    });
                }
            };
            let value = rest.iter().try_fold(initial_value, |acc, expr| {
                let value = expr.eval(env, variables, mc)?;
                match value {
                    Value::Number(n) => Ok(acc / n),
                    value => Err(LispComputerError::TypeMismatch2 {
                        operation: <Self as Function<'gc, T>>::name(self).to_string(),
                        left_str: acc.to_string(),
                        right_str: format!("{}", value),
                    }),
                }
            })?;
            Ok(Value::Number(value))
        } else {
            Err(LispComputerError::TypeMismatch1 {
                operation: <Self as Function<'gc, T>>::name(self).to_string(),
                left_str: "nil".to_string(),
            })
        }
    }

    fn name(&self) -> &str {
        "/"
    }
}

pub struct EqualProcessor;

impl<'gc, T: Environment<'gc>> Function<'gc, T> for EqualProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::ArityMismatch(
                <Self as Function<'gc, T>>::name(self).to_string(),
                2,
                args.len(),
            ));
        }

        let mut evaluated_args = Vec::new();
        for arg in args {
            evaluated_args.push(arg.eval(env, variables, mc)?);
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

impl<'gc, T: Environment<'gc>> Function<'gc, T> for GreaterThanProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::ArityMismatch(
                <Self as Function<'gc, T>>::name(self).to_string(),
                2,
                args.len(),
            ));
        }

        let mut evaluated_args = Vec::new();
        for arg in args {
            match arg.eval(env, variables, mc)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <Self as Function<'gc, T>>::name(self).to_string(),
                        left_str: format!("{}", other),
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

impl<'gc, T: Environment<'gc>> Function<'gc, T> for LessThanProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::InvalidArguments(
                <Self as Function<'gc, T>>::name(self).to_string(),
                args.iter().map(|e| format!("{}", e)).collect(),
            ));
        }

        let mut evaluated_args = Vec::new();
        for arg in args {
            match arg.eval(env, variables, mc)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <Self as Function<'gc, T>>::name(self).to_string(),
                        left_str: format!("{}", other),
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

impl<'gc, T: Environment<'gc>> Function<'gc, T> for GreaterEqualProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::InvalidArguments(
                <Self as Function<'gc, T>>::name(self).to_string(),
                args.iter().map(|e| format!("{}", e)).collect(),
            ));
        }
        let mut evaluated_args = Vec::new();

        for arg in args {
            match arg.eval(env, variables, mc)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <Self as Function<'gc, T>>::name(self).to_string(),
                        left_str: format!("{}", other),
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

impl<'gc, T: Environment<'gc>> Function<'gc, T> for LessEqualProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if args.len() < 2 {
            return Err(LispComputerError::InvalidArguments(
                <Self as Function<'gc, T>>::name(self).to_string(),
                args.iter().map(|e| format!("{}", e)).collect(),
            ));
        }
        let mut evaluated_args = Vec::new();
        for arg in args {
            match arg.eval(env, variables, mc)? {
                Value::Number(n) => evaluated_args.push(n),
                other => {
                    return Err(LispComputerError::TypeMismatch1 {
                        operation: <Self as Function<'gc, T>>::name(self).to_string(),
                        left_str: format!("{}", other),
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

impl<'gc, T: Environment<'gc>> Function<'gc, T> for IfProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        match args {
            [condition, then_branch, else_branch] => {
                let condition = condition.eval(env, variables, mc)?.boolean();
                match condition {
                    true => then_branch.eval(env, variables, mc),
                    false => else_branch.eval(env, variables, mc),
                }
            }
            _ => Err(LispComputerError::ArityMismatch(
                <Self as Function<'gc, T>>::name(self).to_string(),
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

impl<'gc, T: Environment<'gc>> Function<'gc, T> for OrProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        let mut last_value = Value::Boolean(false);

        for arg in args {
            let value = arg.eval(env, variables, mc)?;
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

impl<'gc, T: Environment<'gc>> Function<'gc, T> for AndProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        let mut last_value = Value::Boolean(true);

        for arg in args {
            let value = arg.eval(env, variables, mc)?;
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

impl<'gc, T: Environment<'gc>> Function<'gc, T> for CondProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if let Some((last, args)) = args.split_last() {
            for arg in args {
                if let Expression::List(inner_args) = &**arg {
                    if let [condition, result] = inner_args.as_slice() {
                        let condition_value = condition.eval(env, variables, mc)?;
                        if condition_value.boolean() {
                            return result.eval(env, variables, mc);
                        }
                    }
                }
            }
            if let Expression::List(inner_args) = &**last {
                if let [var_gc, result_gc] = inner_args.as_slice() {
                    if let Expression::Variable(name) = &**var_gc {
                        if name == "else" {
                            return result_gc.eval(env, variables, mc);
                        }
                    }
                }
            }
        }
        Err(LispComputerError::InvalidArguments(
            <Self as Function<'gc, T>>::name(self).to_string(),
            args.iter().map(|e| format!("{}", e)).collect(),
        ))
    }

    fn name(&self) -> &str {
        "cond"
    }
}

pub struct DefineProcessor;

impl<'gc, T: Environment<'gc>> Function<'gc, T> for DefineProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        match args {
            [first, second] => {
                if let Expression::Variable(name) = &**first {
                    let value = second.eval(env, variables, mc)?;
                    env.set_variable(name.to_string(), value, mc);
                    Ok(Value::Nil)
                } else if let Expression::List(params) = &**first {
                    if let Expression::List(body) = &**second {
                        match params.as_slice() {
                            [var, tail @ ..] => {
                                if let Expression::Variable(name) = &**var {
                                    let params_vec = tail
                                        .iter()
                                        .map(|param| match &**param {
                                            Expression::Variable(name) => Ok(name.clone()),
                                            _ => Err(LispComputerError::InvalidArguments(
                                                "lambda-params".to_string(),
                                                params.iter().map(|e| format!("{}", e)).collect(),
                                            )),
                                        })
                                        .collect::<Result<Vec<String>, LispComputerError>>()?;
                                    // Compute free variables and capture environment
                                    let bound: HashSet<String> =
                                        params_vec.iter().cloned().collect();
                                    let mut free = HashSet::new();
                                    for expr in body {
                                        Lambda::collect_free_vars(&**expr, &bound, &mut free);
                                    }
                                    let mut captured: HashMap<String, Value<'gc>> = HashMap::new();
                                    for name in &free {
                                        // Skip built-in functions - they are globally accessible
                                        if env.is_builtin(name) {
                                            continue;
                                        }
                                        if let Some(value) = env.get_variable(name, variables) {
                                            captured.insert(name.clone(), value.clone());
                                        } else {
                                            return Err(LispComputerError::NotFoundVariable(
                                                name.clone(),
                                            ));
                                        }
                                    }
                                    let lambda = Lambda::new(params_vec, body.clone(), captured);
                                    env.set_variable(
                                        name.to_string(),
                                        Value::Lambda(Gc::new(mc, lambda)),
                                        mc,
                                    );
                                    Ok(Value::Nil)
                                } else {
                                    Err(LispComputerError::InvalidArguments(
                                        "lambda-params".to_string(),
                                        params.iter().map(|e| format!("{}", e)).collect(),
                                    ))
                                }
                            }
                            _ => Err(LispComputerError::InvalidArguments(
                                <Self as Function<'gc, T>>::name(self).to_string(),
                                args.iter().map(|e| format!("{}", e)).collect(),
                            )),
                        }
                    } else {
                        Err(LispComputerError::InvalidArguments(
                            <Self as Function<'gc, T>>::name(self).to_string(),
                            args.iter().map(|e| format!("{}", e)).collect(),
                        ))
                    }
                } else {
                    Err(LispComputerError::InvalidArguments(
                        <Self as Function<'gc, T>>::name(self).to_string(),
                        args.iter().map(|e| format!("{}", e)).collect(),
                    ))
                }
            }
            _ => Err(LispComputerError::InvalidArguments(
                <Self as Function<'gc, T>>::name(self).to_string(),
                args.iter().map(|e| format!("{}", e)).collect(),
            )),
        }
    }

    fn name(&self) -> &str {
        "define"
    }
}

pub struct LambdaProcessor;

impl<'gc, T: Environment<'gc>> Function<'gc, T> for LambdaProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        // args should be: [params_gc, body1, body2, ...]
        if let [params_gc, rest @ ..] = args {
            if let Expression::List(params) = &**params_gc {
                let param_names: Vec<String> = params
                    .iter()
                    .map(|param| match &**param {
                        Expression::Variable(name) => Ok(name.clone()),
                        _ => Err(LispComputerError::InvalidArguments(
                            "lambda-params".to_string(),
                            params.iter().map(|e| format!("{}", e)).collect(),
                        )),
                    })
                    .collect::<Result<Vec<String>, LispComputerError>>()?;

                // Compute free variables in the body expressions, excluding lambda's own parameters
                let bound: std::collections::HashSet<String> =
                    param_names.iter().cloned().collect();
                let mut free = std::collections::HashSet::new();
                for expr in rest {
                    Lambda::collect_free_vars(&**expr, &bound, &mut free);
                }

                // Capture free variables from the current environment (excluding builtins)
                let mut captured: HashMap<String, Value<'gc>> = HashMap::new();
                for name in &free {
                    if env.is_builtin(name) {
                        continue;
                    }
                    if let Some(value) = env.get_variable(name, variables) {
                        captured.insert(name.clone(), value.clone());
                    } else {
                        return Err(LispComputerError::NotFoundVariable(name.clone()));
                    }
                }

                // Body is the slice `rest` converted to Vec
                let body_vec = rest.to_vec();
                Ok(Value::Lambda(Gc::new(
                    mc,
                    Lambda::new(param_names, body_vec, captured),
                )))
            } else {
                Err(LispComputerError::InvalidArguments(
                    <Self as Function<'gc, T>>::name(self).to_string(),
                    args.iter().map(|e| format!("{}", e)).collect(),
                ))
            }
        } else {
            Err(LispComputerError::InvalidArguments(
                <Self as Function<'gc, T>>::name(self).to_string(),
                args.iter().map(|e| format!("{}", e)).collect(),
            ))
        }
    }

    fn name(&self) -> &str {
        "lambda"
    }
}

pub struct LetProcessor;

impl<'gc, T: Environment<'gc>> Function<'gc, T> for LetProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        fn get_lambda_from<'gc, U: Environment<'gc>>(
            env: &U,
            variables: &HashMap<String, Value<'gc>>,
            mc: &'gc Mutation<'gc>,
            bindings: &[Gc<'gc, Expression<'gc>>],
            body: &[Gc<'gc, Expression<'gc>>],
        ) -> Result<(Gc<'gc, Lambda<'gc>>, Vec<Gc<'gc, Expression<'gc>>>), LispComputerError>
        {
            let mut params = Vec::new();
            let mut lambda_args = Vec::new();
            for binding in bindings {
                match &**binding {
                    Expression::List(list) => match list.as_slice() {
                        [var, value] => {
                            if let Expression::Variable(name) = &**var {
                                params.push(name.to_string());
                                lambda_args.push(value.clone());
                            } else {
                                return Err(LispComputerError::InvalidArguments(
                                    "let-bindings".to_string(),
                                    bindings.iter().map(|e| format!("{}", e)).collect(),
                                ));
                            }
                        }
                        _ => {
                            return Err(LispComputerError::InvalidArguments(
                                "let-bindings".to_string(),
                                bindings.iter().map(|e| format!("{}", e)).collect(),
                            ));
                        }
                    },
                    _ => {
                        return Err(LispComputerError::InvalidArguments(
                            "let-bindings".to_string(),
                            bindings.iter().map(|e| format!("{}", e)).collect(),
                        ));
                    }
                }
            }

            // Compute free variables and capture environment
            let bound: HashSet<String> = params.iter().cloned().collect();
            let mut free = HashSet::new();
            for expr in body {
                Lambda::collect_free_vars(&**expr, &bound, &mut free);
            }
            let mut captured: HashMap<String, Value<'gc>> = HashMap::new();
            for name in &free {
                // Skip built-in functions - they are globally accessible
                if env.is_builtin(name) {
                    continue;
                }
                if let Some(value) = env.get_variable(name, variables) {
                    captured.insert(name.clone(), value.clone());
                } else {
                    return Err(LispComputerError::NotFoundVariable(name.clone()));
                }
            }

            let lambda = Gc::new(mc, Lambda::new(params, body.to_vec(), captured));
            Ok((lambda, lambda_args))
        }
        match args {
            // let naming: (let name ((var val) ...) body...)
            [name_expr, bindings_gc, rest @ ..]
                if matches!(&**name_expr, Expression::Variable(_)) =>
            {
                if let (Expression::Variable(name), Expression::List(bindings)) =
                    (&**name_expr, &**bindings_gc)
                {
                    if rest.is_empty() {
                        return Err(LispComputerError::InvalidArguments(
                            <Self as Function<'gc, T>>::name(self).to_string(),
                            args.iter().map(|e| format!("{}", e)).collect(),
                        ));
                    }
                    let (lambda, lambda_args) =
                        get_lambda_from(env, variables, mc, bindings, rest)?;
                    let mut new_vars = variables.clone();
                    new_vars.insert(name.to_string(), Value::Lambda(lambda.clone()));
                    return lambda.process(&lambda_args, env, &new_vars, mc);
                } else {
                    return Err(LispComputerError::InvalidArguments(
                        <Self as Function<'gc, T>>::name(self).to_string(),
                        args.iter().map(|e| format!("{}", e)).collect(),
                    ));
                }
            }
            // let: (let ((var val) ...) body...)
            [bindings_gc, rest @ ..] => {
                if let Expression::List(bindings) = &**bindings_gc {
                    if rest.is_empty() {
                        return Err(LispComputerError::InvalidArguments(
                            <Self as Function<'gc, T>>::name(self).to_string(),
                            args.iter().map(|e| format!("{}", e)).collect(),
                        ));
                    }
                    let (lambda, lambda_args) =
                        get_lambda_from(env, variables, mc, bindings, rest)?;
                    return lambda.process(&lambda_args, env, variables, mc);
                } else {
                    return Err(LispComputerError::InvalidArguments(
                        <Self as Function<'gc, T>>::name(self).to_string(),
                        args.iter().map(|e| format!("{}", e)).collect(),
                    ));
                }
            }
            _ => {
                return Err(LispComputerError::InvalidArguments(
                    <Self as Function<'gc, T>>::name(self).to_string(),
                    args.iter().map(|e| format!("{}", e)).collect(),
                ))
            }
        }
    }

    fn name(&self) -> &str {
        "let"
    }
}

pub struct DoProcessor;

impl<'gc, T: Environment<'gc>> Function<'gc, T> for DoProcessor {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        match args {
            [bindings_gc, test_gc, bodys @ ..] => {
                if let (Expression::List(bindings), Expression::List(test)) =
                    (&**bindings_gc, &**test_gc)
                {
                    let mut new_variables = variables.clone();
                    let mut steps = Vec::new();
                    for binding in bindings {
                        match &**binding {
                            Expression::List(list) => match list.as_slice() {
                                [var_gc, value_gc, step_expr_gc] => {
                                    if let Expression::Variable(name) = &**var_gc {
                                        let evaluated = value_gc.eval(env, variables, mc)?;
                                        new_variables.insert(name.clone(), evaluated);
                                        steps.push((name.clone(), step_expr_gc.clone()));
                                    } else {
                                        return Err(LispComputerError::InvalidArguments(
                                            <Self as Function<'gc, T>>::name(self).to_string(),
                                            args.iter().map(|e| format!("{}", e)).collect(),
                                        ));
                                    }
                                }
                                _ => {
                                    return Err(LispComputerError::InvalidArguments(
                                        <Self as Function<'gc, T>>::name(self).to_string(),
                                        args.iter().map(|e| format!("{}", e)).collect(),
                                    ));
                                }
                            },
                            _ => {
                                return Err(LispComputerError::InvalidArguments(
                                    <Self as Function<'gc, T>>::name(self).to_string(),
                                    args.iter().map(|e| format!("{}", e)).collect(),
                                ));
                            }
                        }
                    }
                    let (test_expr, result_expr) = match test.as_slice() {
                        [test, result] => (test.clone(), result.clone()),
                        _ => {
                            return Err(LispComputerError::InvalidArguments(
                                <Self as Function<'gc, T>>::name(self).to_string(),
                                args.iter().map(|e| format!("{}", e)).collect(),
                            ));
                        }
                    };
                    loop {
                        if test_expr.eval(env, &new_variables, mc)?.boolean() {
                            return result_expr.eval(env, &new_variables, mc);
                        }
                        for body in bodys {
                            body.eval(env, &new_variables, mc)?;
                        }
                        for (name, step_expr) in &steps {
                            let new_value = step_expr.eval(env, &new_variables, mc)?;
                            new_variables.insert(name.clone(), new_value);
                        }
                    }
                } else {
                    Err(LispComputerError::InvalidArguments(
                        <Self as Function<'gc, T>>::name(self).to_string(),
                        args.iter().map(|e| format!("{}", e)).collect(),
                    ))
                }
            }
            _ => Err(LispComputerError::InvalidArguments(
                <Self as Function<'gc, T>>::name(self).to_string(),
                args.iter().map(|e| format!("{}", e)).collect(),
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
    use crate::test_utils;
    use gc_arena::Gc;
    use std::collections::HashMap;

    fn eval(input: &str) -> Result<String, LispComputerError> {
        let mut arena = new_arena();
        test_utils::eval_str(input, &mut arena)
    }

    fn new_arena() -> crate::root::GcArena<'static> {
        crate::root::GcArena::new(|mc| {
            let mut vars = HashMap::new();
            vars.insert("#f".to_string(), Value::Boolean(false));
            vars.insert("#t".to_string(), Value::Boolean(true));
            crate::root::LispRoot {
                variables: Gc::new(mc, RefLock::new(vars)),
            }
        })
    }

    #[test]
    fn test_addition() {
        assert_eq!(eval("(+ 1 2)"), Ok("3".to_string()));
        assert_eq!(eval("(+ 1 2 3 4)"), Ok("10".to_string()));
        assert_eq!(eval("(+ 1)"), Ok("1".to_string()));
        assert_eq!(eval("(+)"), Ok("0".to_string()));
    }

    #[test]
    fn test_subtraction() {
        assert_eq!(eval("(- 5 2)"), Ok("3".to_string()));
        assert_eq!(eval("(- 5 1 2)"), Ok("2".to_string()));
        assert_eq!(eval("(- 10)"), Ok("-10".to_string()));
    }

    #[test]
    fn test_multiplication() {
        assert_eq!(eval("(* 2 3)"), Ok("6".to_string()));
        assert_eq!(eval("(* 2 3 4)"), Ok("24".to_string()));
        assert_eq!(eval("(*)"), Ok("1".to_string()));
    }

    #[test]
    fn test_division() {
        assert_eq!(eval("(/ 10 2)"), Ok("5".to_string()));
        assert_eq!(eval("(/ 10 2 2)"), Ok("2.5".to_string()));
        let result = eval("(/ 1 0)");
        assert!(result.is_ok());
        if let Ok(s) = result {
            assert_eq!(s, "inf");
        } else {
            panic!("Expected infinite number");
        }
    }

    #[test]
    fn test_string_concatenation() {
        assert_eq!(eval(r#""hello""#), Ok("\"hello\"".to_string()));
        assert_eq!(
            eval(r#"(+ "hello" "world")"#),
            Ok("\"helloworld\"".to_string())
        );
        assert_eq!(eval(r#"(+ "a" "b" "c")"#), Ok("\"abc\"".to_string()));
    }

    #[test]
    fn test_type_mismatch_in_addition() {
        let result = eval(r#"(+ "hello" 1)"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch2 { .. })
        ));
        let result = eval(r#"(+ 1 "hello")"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch2 { .. })
        ));
        let result = eval(r#"(+ #t 1)"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch1 { .. })
        ));
        let result = eval(r#"(+ 0 "hello")"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch2 { .. })
        ));
        let result = eval(r#"(+ 1 -1 "hello")"#);
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch2 { .. })
        ));
    }

    #[test]
    fn test_equality() {
        assert_eq!(eval("(= 1 1)"), Ok("true".to_string()));
        assert_eq!(eval("(= 1 2)"), Ok("false".to_string()));
        assert_eq!(eval("(= 1.0 1.0)"), Ok("true".to_string()));
        assert_eq!(eval("(= 1.0 2.0)"), Ok("false".to_string()));
        assert_eq!(eval("(= #t #t)"), Ok("true".to_string()));
        assert_eq!(eval("(= #t #f)"), Ok("false".to_string()));
    }

    #[test]
    fn test_comparison() {
        assert_eq!(eval("(> 2 1)"), Ok("true".to_string()));
        assert_eq!(eval("(> 1 2)"), Ok("false".to_string()));
        assert_eq!(eval("(< 1 2)"), Ok("true".to_string()));
        assert_eq!(eval("(< 2 1)"), Ok("false".to_string()));
        assert_eq!(eval("(>= 2 2)"), Ok("true".to_string()));
        assert_eq!(eval("(<= 2 2)"), Ok("true".to_string()));
    }

    #[test]
    fn test_logical_operations() {
        assert_eq!(eval("(and #t #t)"), Ok("true".to_string()));
        assert_eq!(eval("(and #t #f)"), Ok("false".to_string()));
        assert_eq!(eval("(and #f #t)"), Ok("false".to_string()));
        assert_eq!(eval("(or #f #f)"), Ok("false".to_string()));
        assert_eq!(eval("(or #f #t)"), Ok("true".to_string()));
        assert_eq!(eval("(or #t #f)"), Ok("true".to_string()));
    }

    #[test]
    fn test_lambda() {
        assert!(eval("(lambda (x) (x))").is_ok());
        assert_eq!(eval("((lambda (x y) (+ x y)) 2 3)"), Ok("5".to_string()));
        assert_eq!(eval("((lambda (x y) (* x y)) 4 5)"), Ok("20".to_string()));
        assert_eq!(eval("((lambda (x) (+ x 1)) 5)"), Ok("6".to_string()));
    }

    #[test]
    fn test_lambda_with_variable_capture() {
        let result = eval("((lambda (x) (lambda (y) (+ x y))) 2)");
        match result {
            Ok(s) => {
                assert!(s.starts_with("<lambda>"));
            }
            Err(e) => {
                panic!("Expected lambda, got error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_lambda_captures_callee_variable() {
        assert_eq!(
            eval("(let ((f (lambda (x) (+ x 1)))) ((lambda () (f 1))))"),
            Ok("2".to_string())
        );
    }

    #[test]
    fn test_closure_capture_is_lexical_not_dynamic() {
        assert_eq!(
            eval("(let ((x 2)) (let ((f (lambda (y) (+ x y)))) (let ((x 100)) (f 3))))"),
            Ok("5".to_string())
        );
    }

    #[test]
    fn test_nested_closure_captures_multiple_outer_bindings() {
        assert_eq!(
            eval("((((lambda (x) (lambda (y) (lambda (z) (+ x (+ y z))))) 1) 2) 3)"),
            Ok("6".to_string())
        );
    }

    #[test]
    fn test_gc_preserves_global_values_across_mutations() {
        let mut arena = new_arena();

        assert_eq!(
            test_utils::eval_str(r#"(define greeting "hello")"#, &mut arena),
            Ok("nil".to_string())
        );

        for i in 0..128 {
            let expr = format!(r#""scratch-{}-{}""#, i, "x".repeat(64));
            assert!(test_utils::eval_str(&expr, &mut arena).is_ok());
        }

        assert_eq!(
            test_utils::eval_str("greeting", &mut arena),
            Ok("\"hello\"".to_string())
        );
    }

    #[test]
    fn test_gc_preserves_captured_closure_across_mutations() {
        let mut arena = new_arena();

        assert_eq!(
            test_utils::eval_str(
                "(define add-base ((lambda (base) (lambda (x) (+ base x))) 40))",
                &mut arena,
            ),
            Ok("nil".to_string())
        );

        for i in 0..128 {
            let expr = format!("(+ {} {})", i, i + 1);
            assert!(test_utils::eval_str(&expr, &mut arena).is_ok());
        }

        assert_eq!(test_utils::eval_str("(add-base 2)", &mut arena), Ok("42".to_string()));
    }

    #[test]
    fn test_if() {
        assert_eq!(eval("(if #t 1 2)"), Ok("1".to_string()));
        assert_eq!(eval("(if #f 1 2)"), Ok("2".to_string()));
        assert!(matches!(
            eval("(if #t \"then\" \"else\")"),
            Ok(s) if s == "\"then\""
        ));
    }

    #[test]
    fn test_define() {
        assert_eq!(eval("(define x 42)"), Ok("nil".to_string()));
    }

    #[test]
    fn test_let() {
        assert_eq!(eval("(let ((x 1) (y 2)) (+ x y))"), Ok("3".to_string()));
        assert_eq!(eval("(let ((a 10) (b 20)) (- a b))"), Ok("-10".to_string()));
    }

    #[test]
    fn test_cond() {
        assert_eq!(eval("(cond (#t 1) (else 2))"), Ok("1".to_string()));
        assert_eq!(eval("(cond (#f 1) (else 2))"), Ok("2".to_string()));
        assert_eq!(eval("(cond (else 3))"), Ok("3".to_string()));
    }

    #[test]
    fn test_do() {
        assert_eq!(
            eval("(do ((i 1 (+ i 1))) ((= i 5) i))"),
            Ok("5".to_string())
        );
    }

    #[test]
    fn test_function_application_non_lambda() {
        let result = eval("((if #t 1 2) 3)");
        assert!(matches!(
            result,
            Err(LispComputerError::TypeMismatch1 { .. })
        ));
    }

    #[test]
    fn test_invalid_expression() {
        let result = eval("(1 2 3 4)");
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
        assert_eq!(eval("(+ (* 2 3) (/ 10 2))"), Ok("11".to_string()));
        assert_eq!(eval("(* (+ 1 2) (- 5 2))"), Ok("9".to_string()));
    }

    #[test]
    fn test_boolean_operations() {
        assert_eq!(eval("(and #t #f #t)"), Ok("false".to_string()));
        assert_eq!(eval("(or #f #t #f)"), Ok("true".to_string()));
    }

    #[test]
    fn test_empty_list() {
        assert_eq!(eval("()"), Ok("nil".to_string()));
    }

    #[test]
    fn test_single_expression() {
        assert_eq!(eval("42"), Ok("42".to_string()));
        assert!(matches!(eval("\"test\""), Ok(s) if s == "\"test\""));
        assert_eq!(eval("#t"), Ok("true".to_string()));
        assert_eq!(eval("#f"), Ok("false".to_string()));
    }
}
