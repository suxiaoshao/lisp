mod lambda;

use gc_arena::Gc;
use gc_arena_derive::Collect;
use std::fmt::Display;

pub use lambda::Lambda;

/// 处理器函数指针类型
/// 所有内置函数和特殊形式都使用此类型
pub type ProcessorFunc = for<'gc> fn(
    args: &[Gc<'gc, crate::parse::Expression<'gc>>],
    env: &'gc crate::root::LispRoot<'gc>,
    variables: &std::collections::HashMap<String, Value<'gc>>,
    mc: &'gc gc_arena::Mutation<'gc>,
) -> Result<Value<'gc>, crate::errors::LispComputerError>;

#[derive(Debug, Clone, Collect)]
#[collect(no_drop)]
pub enum Value<'gc> {
    String(Gc<'gc, String>),
    Number(f64),
    Boolean(bool),
    Nil,
    Lambda(Gc<'gc, Lambda<'gc>>),
    Processor(#[collect(require_static)] ProcessorFunc, &'static str),
}

impl<'gc> PartialEq for Value<'gc> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Lambda(a), Value::Lambda(b)) => a == b,
            (Value::Processor(_, name_a), Value::Processor(_, name_b)) => name_a == name_b,
            _ => false,
        }
    }
}

impl<'gc> Display for Value<'gc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::Lambda(l) => write!(f, "<lambda>:{}", l),
            Value::Processor(_, name) => write!(f, "<{}>", name),
        }
    }
}

impl<'gc> Value<'gc> {
    pub fn boolean(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Nil => false,
            _ => true,
        }
    }
}
