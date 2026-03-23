use rustyline::error::ReadlineError;

use crate::{parse::Expression, value::Value};

#[derive(thiserror::Error, Debug)]
pub enum LispError {
    #[error("Invalid input")]
    InvalidInput,
    #[error("readline error")]
    ReadlineError(#[from] ReadlineError),
    #[error("computer error")]
    ComputerError(#[from] LispComputerError),
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum LispComputerError {
    #[error("Unbound function:{}",.0)]
    UnboundFunction(String),
    #[error("Operation {} mismatch: left {}, right {}",.operation,.left,.right)]
    TypeMismatch2 {
        operation: String,
        left: Value,
        right: Value,
    },
    #[error("Operation {} mismatch: get {}",.operation,.left)]
    TypeMismatch1 { operation: String, left: Value },
    #[error("Invalid arguments for function {}: {}",.0,.1.iter().map(|e| format!("{}", e)).collect::<Vec<String>>().join(" "))]
    InvalidArguments(String, Vec<Expression>),
    #[error("Variable not found: {}",.0)]
    NotFoundVariable(String),
    #[error("Arity mismatch {}: expected {}, got {}",.0,.1,.2)]
    ArityMismatch(String, usize, usize),
    #[error("Let naming not return")]
    LetNamingNotReturn,
    #[error("Invalid expression: {0}")]
    InvalidExpression(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = LispComputerError::UnboundFunction("foo".to_string());
        assert_eq!(format!("{}", err), "Unbound function:foo");

        let err = LispComputerError::TypeMismatch1 {
            operation: "+".to_string(),
            left: Value::Number(1.0),
        };
        assert!(format!("{}", err).contains("Operation + mismatch"));

        let err = LispComputerError::InvalidExpression("test expr".to_string());
        assert_eq!(format!("{}", err), "Invalid expression: test expr");

        let err = LispComputerError::ArityMismatch("func".to_string(), 2, 3);
        assert_eq!(format!("{}", err), "Arity mismatch func: expected 2, got 3");

        let err = LispComputerError::NotFoundVariable("x".to_string());
        assert_eq!(format!("{}", err), "Variable not found: x");
    }
}
