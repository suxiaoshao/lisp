mod lambda;

use gc_arena::Gc;
use gc_arena_derive::Collect;
use std::fmt::Display;

pub use lambda::Lambda;

#[derive(Debug, Clone, PartialEq, Collect)]
#[collect(no_drop)]
pub enum Value<'gc> {
    String(Gc<'gc, String>),
    Number(f64),
    Boolean(bool),
    Nil,
    Lambda(Gc<'gc, Lambda<'gc>>),
}

impl<'gc> Display for Value<'gc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::Lambda(l) => write!(f, "<lambda>:{}", l),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{errors::LispComputerError, root::GcArena};
    use gc_arena::Gc;
    use gc_arena::lock::RefLock;
    use std::collections::HashMap;

    #[test]
    fn test_value_display() {
        let arena = GcArena::new(|mc| crate::root::LispRoot {
            variables: Gc::new(mc, RefLock::new(HashMap::new())),
        });
        arena
            .mutate(|mc, _root| -> Result<(), LispComputerError> {
                assert_eq!(format!("{}", Value::Number(42.0)), "42");
                let string_val = Value::String(Gc::new(mc, "hello".to_string()));
                assert_eq!(format!("{}", string_val), "\"hello\"");
                assert_eq!(format!("{}", Value::Boolean(true)), "true");
                assert_eq!(format!("{}", Value::Boolean(false)), "false");
                assert_eq!(format!("{}", Value::Nil), "nil");
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn test_value_boolean() {
        assert!(Value::Boolean(true).boolean());
        assert!(!Value::Boolean(false).boolean());
        assert!(!Value::Nil.boolean());
        assert!(Value::Number(0.0).boolean());
        let arena = GcArena::new(|mc| crate::root::LispRoot {
            variables: Gc::new(mc, RefLock::new(HashMap::new())),
        });
        arena
            .mutate(|mc, _root| -> Result<(), LispComputerError> {
                let string_val = Value::String(Gc::new(mc, "".to_string()));
                assert!(string_val.boolean());
                Ok(())
            })
            .unwrap();
    }
}
