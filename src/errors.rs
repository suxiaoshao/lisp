use rustyline::error::ReadlineError;

use crate::value::Value;

#[derive(thiserror::Error, Debug)]
pub enum LispError {
    #[error("Invalid input")]
    InvalidInput,
    #[error("readline error")]
    ReadlineError(#[from] ReadlineError),
    #[error("computer error")]
    ComputerError(#[from] LispComputerError),
}

#[derive(Debug, thiserror::Error)]
pub enum LispComputerError {
    #[error("Unbound function:{}",.0)]
    UnboundFunction(String),
    #[error("Operation {} mismatch: left {:?}, right {:?}",.operation,.left,.right)]
    TypeMismatch2 {
        operation: String,
        left: Value,
        right: Value,
    },
    #[error("Operation {} mismatch: get {:?}",.operation,.left)]
    TypeMismatch1 { operation: String, left: Value },
}
