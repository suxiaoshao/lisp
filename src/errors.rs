use rustyline::error::ReadlineError;

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
    #[error("Operation {} mismatch: left {}, right {}",.operation,.left_str,.right_str)]
    TypeMismatch2 {
        operation: String,
        left_str: String,
        right_str: String,
    },
    #[error("Operation {} mismatch: get {}",.operation,.left_str)]
    TypeMismatch1 { operation: String, left_str: String },
    #[error("Invalid arguments for function {}: {}",.0,.1.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(" "))]
    InvalidArguments(String, Vec<String>),
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
    use crate::value::Value;

    #[test]
    fn test_error_display() {
        let err = LispComputerError::UnboundFunction("foo".to_string());
        assert_eq!(format!("{}", err), "Unbound function:foo");

        let err = LispComputerError::TypeMismatch1 {
            operation: "+".to_string(),
            left_str: Value::Number(1.0).to_string(),
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
