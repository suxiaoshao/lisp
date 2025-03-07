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

#[derive(Debug, thiserror::Error)]
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
}
