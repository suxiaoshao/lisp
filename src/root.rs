use gc_arena::lock::RefLock;
use gc_arena::{Arena, Gc, Mutation, Rootable};
use gc_arena_derive::Collect;
use std::collections::HashMap;

use crate::{
    errors::LispComputerError,
    parse::Expression,
    process::{
        addition_call, and_call, cond_call, define_call, division_call, do_call, equal_call,
        greater_equal_call, greater_than_call, if_call, lambda_call, less_equal_call,
        less_than_call, let_call, multiplication_call, or_call, subtraction_call,
    },
    value::Value,
};

/// Root struct for the GC arena, holding global variables.
#[derive(Collect)]
#[collect(no_drop)]
pub struct LispRoot<'gc> {
    pub variables: Gc<'gc, RefLock<HashMap<String, Value<'gc>>>>,
}

/// A token type that implements Rootable for any lifetime, linking to LispRoot.
pub struct RootToken;
impl<'gc> Rootable<'gc> for RootToken {
    type Root = LispRoot<'gc>;
}

/// Type alias for the GC arena.
pub type GcArena<'gc> = Arena<RootToken>;

impl<'gc> LispRoot<'gc> {
    /// 处理变量调用（函数应用）
    pub fn process_variable(
        &'gc self,
        symbol: &str,
        args: &[Gc<'gc, Expression<'gc>>],
        variables: &HashMap<String, Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if let Some(value) = Self::get_variable(self, symbol, variables) {
            match value {
                Value::Lambda(lambda) => return lambda.call(args, self, variables, mc),
                Value::Processor(proc, _name) => return proc(args, self, variables, mc),
                _ => {}
            }
        }
        Err(LispComputerError::UnboundFunction(symbol.to_string()))
    }

    pub fn set_variable(&self, name: String, value: Value<'gc>, mc: &'gc Mutation<'gc>) {
        self.variables.borrow_mut(mc).insert(name, value);
    }

    pub fn get_variable(
        &self,
        name: &str,
        variables: &HashMap<String, Value<'gc>>,
    ) -> Option<Value<'gc>> {
        if let Some(value) = variables.get(name).cloned() {
            Some(value)
        } else {
            self.variables.borrow().get(name).cloned()
        }
    }

    pub fn new(mc: &'gc Mutation<'gc>) -> Self {
        let mut vars = HashMap::new();
        vars.insert("#f".to_string(), Value::Boolean(false));
        vars.insert("#t".to_string(), Value::Boolean(true));
        // 注册所有内置函数和特殊形式
        vars.insert("+".to_string(), Value::Processor(addition_call, "+"));
        vars.insert("-".to_string(), Value::Processor(subtraction_call, "-"));
        vars.insert("*".to_string(), Value::Processor(multiplication_call, "*"));
        vars.insert("/".to_string(), Value::Processor(division_call, "/"));
        vars.insert("=".to_string(), Value::Processor(equal_call, "="));
        vars.insert(">".to_string(), Value::Processor(greater_than_call, ">"));
        vars.insert("<".to_string(), Value::Processor(less_than_call, "<"));
        vars.insert(">=".to_string(), Value::Processor(greater_equal_call, ">="));
        vars.insert("<=".to_string(), Value::Processor(less_equal_call, "<="));
        vars.insert("if".to_string(), Value::Processor(if_call, "if"));
        vars.insert("or".to_string(), Value::Processor(or_call, "or"));
        vars.insert("and".to_string(), Value::Processor(and_call, "and"));
        vars.insert("cond".to_string(), Value::Processor(cond_call, "cond"));
        vars.insert(
            "define".to_string(),
            Value::Processor(define_call, "define"),
        );
        vars.insert(
            "lambda".to_string(),
            Value::Processor(lambda_call, "lambda"),
        );
        vars.insert("let".to_string(), Value::Processor(let_call, "let"));
        vars.insert("do".to_string(), Value::Processor(do_call, "do"));

        LispRoot {
            variables: Gc::new(mc, RefLock::new(vars)),
        }
    }
}
