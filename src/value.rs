mod lambda;

use std::fmt::Display;

pub use lambda::Lambda;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Nil,
    Lambda(lambda::Lambda),
}

impl Display for Value {
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

impl Value {
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

    #[test]
    fn test_value_display() {
        assert_eq!(format!("{}", Value::Number(42.0)), "42");
        assert_eq!(
            format!("{}", Value::String("hello".to_string())),
            "\"hello\""
        );
        assert_eq!(format!("{}", Value::Boolean(true)), "true");
        assert_eq!(format!("{}", Value::Boolean(false)), "false");
        assert_eq!(format!("{}", Value::Nil), "nil");
    }

    #[test]
    fn test_value_boolean() {
        assert!(Value::Boolean(true).boolean());
        assert!(!Value::Boolean(false).boolean());
        assert!(!Value::Nil.boolean());
        assert!(Value::Number(0.0).boolean());
        assert!(Value::String("".to_string()).boolean());
    }
}
