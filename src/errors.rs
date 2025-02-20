use rustyline::error::ReadlineError;

#[derive(thiserror::Error, Debug)]
pub enum LispError {
    #[error("Invalid input")]
    InvalidInput,
    #[error("readline error")]
    ReadlineError(#[from] ReadlineError),
    #[error("computer error")]
    ComputerError,
    #[error("sub error:{}",.0)]
    SubError(String),
    #[error("division error:{}",.0)]
    DivError(String),
}
