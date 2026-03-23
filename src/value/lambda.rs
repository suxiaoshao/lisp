use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use crate::{
    environment::Environment, errors::LispComputerError, parse::Expression, process::Function,
};

use super::Value;
use gc_arena::{Gc, Mutation};
use gc_arena_derive::Collect;

#[derive(Debug, Clone, PartialEq, Collect)]
#[collect(no_drop)]
pub struct Lambda<'gc> {
    params: Vec<String>,
    body: Vec<Gc<'gc, Expression<'gc>>>,
    captured: HashMap<String, Value<'gc>>,
}

impl<'gc> Display for Lambda<'gc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(lambda ({})(", self.params.join(" "))?;
        for expr in &self.body {
            write!(f, " {}", expr)?;
        }
        write!(f, "))")
    }
}

impl<'gc> Lambda<'gc> {
    pub fn new(
        params: Vec<String>,
        body: Vec<Gc<'gc, Expression<'gc>>>,
        captured: HashMap<String, Value<'gc>>,
    ) -> Self {
        Lambda {
            params,
            body,
            captured,
        }
    }

    pub fn collect_free_vars(
        expr: &Expression<'gc>,
        bound: &HashSet<String>,
        free: &mut HashSet<String>,
    ) {
        match expr {
            Expression::Number(_) | Expression::String(_) => {}
            Expression::Variable(name) => {
                if !bound.contains(name) {
                    free.insert(name.clone());
                }
            }
            Expression::List(exprs) => {
                if let Some((first, rest)) = exprs.split_first() {
                    match &**first {
                        Expression::Variable(var_name) => {
                            match var_name.as_str() {
                                "lambda" => {
                                    // (lambda (params) body...)
                                    if let Some(params_gc) = rest.first() {
                                        if let Expression::List(params) = &**params_gc {
                                            let param_names: HashSet<String> = params
                                                .iter()
                                                .filter_map(|p| match &**p {
                                                    Expression::Variable(var) => Some(var.clone()),
                                                    _ => None,
                                                })
                                                .collect();
                                            let mut new_bound = bound.clone();
                                            new_bound.extend(param_names);
                                            // Process body expressions (rest[1..])
                                            for body_expr in rest.iter().skip(1) {
                                                Self::collect_free_vars(
                                                    body_expr, &new_bound, free,
                                                );
                                            }
                                        } else {
                                            // malformed lambda, process normally
                                            for e in rest {
                                                Self::collect_free_vars(e, bound, free);
                                            }
                                        }
                                    } else {
                                        // no params, nothing
                                    }
                                }
                                "let" => {
                                    let (recursive_name, bindings, body) = match rest {
                                        [name_gc, bindings_gc, body @ ..]
                                            if matches!(&**name_gc, Expression::Variable(_))
                                                && matches!(
                                                    &**bindings_gc,
                                                    Expression::List(_)
                                                ) =>
                                        {
                                            if let (
                                                Expression::Variable(name),
                                                Expression::List(bindings),
                                            ) = (&**name_gc, &**bindings_gc)
                                            {
                                                (Some(name), bindings, body)
                                            } else {
                                                unreachable!()
                                            }
                                        }
                                        [bindings_gc, body @ ..]
                                            if matches!(&**bindings_gc, Expression::List(_)) =>
                                        {
                                            if let Expression::List(bindings) = &**bindings_gc {
                                                (None, bindings, body)
                                            } else {
                                                unreachable!()
                                            }
                                        }
                                        _ => {
                                            for e in rest {
                                                Self::collect_free_vars(e, bound, free);
                                            }
                                            return;
                                        }
                                    };

                                    let mut new_bound = bound.clone();
                                    if let Some(name) = recursive_name {
                                        new_bound.insert(name.clone());
                                    }

                                    for binding in bindings {
                                        if let Expression::List(binding_list) = &**binding {
                                            match binding_list.as_slice() {
                                                [var_expr, value_expr] => {
                                                    Self::collect_free_vars(
                                                        value_expr, bound, free,
                                                    );
                                                    if let Expression::Variable(var_name) =
                                                        &**var_expr
                                                    {
                                                        new_bound.insert(var_name.clone());
                                                    }
                                                }
                                                _ => {
                                                    for e in binding_list {
                                                        Self::collect_free_vars(e, bound, free);
                                                    }
                                                }
                                            }
                                        } else {
                                            Self::collect_free_vars(binding, bound, free);
                                        }
                                    }

                                    for body_expr in body {
                                        Self::collect_free_vars(body_expr, &new_bound, free);
                                    }
                                }
                                _ => {
                                    // Not a binding special form, process all subexpressions normally
                                    for e in exprs {
                                        Self::collect_free_vars(e, bound, free);
                                    }
                                }
                            }
                        }
                        _ => {
                            // First element not a variable, process all subexpressions normally
                            for e in exprs {
                                Self::collect_free_vars(e, bound, free);
                            }
                        }
                    }
                }
            }
            Expression::NamingList(_, exprs) => {
                for e in exprs {
                    Self::collect_free_vars(e, bound, free);
                }
            }
        }
    }
}

impl<'gc, T: Environment<'gc>> Function<'gc, T> for Lambda<'gc> {
    fn process(
        &self,
        args: &[Gc<'gc, Expression<'gc>>],
        env: &T,
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if args.len() != self.params.len() {
            return Err(LispComputerError::ArityMismatch(
                <Self as Function<'gc, T>>::name(self).to_string(),
                self.params.len(),
                args.len(),
            ));
        }
        // Preserve caller-local bindings that are intentionally threaded through
        // special forms such as named let, then overlay lexical captures.
        let mut new_variables = variables.clone();
        new_variables.extend(self.captured.clone());
        // Bind arguments, allowing them to shadow captured variables
        for (param, arg) in self.params.iter().zip(args) {
            let value = arg.eval(env, variables, mc)?;
            new_variables.insert(param.to_string(), value);
        }
        // Evaluate body expressions sequentially, returning the last result
        let mut result = Value::Nil;
        for expr in &self.body {
            result = expr.eval(env, &new_variables, mc)?;
        }
        Ok(result)
    }

    fn name(&self) -> &str {
        "lambda-function"
    }
}
