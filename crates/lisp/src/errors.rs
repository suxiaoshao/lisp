use rustyline::error::ReadlineError;

/// Top-level errors that can occur while using the CLI compatibility crate.
#[derive(thiserror::Error, Debug)]
pub enum LispError {
    /// The input could not be parsed as a valid Lisp expression.
    #[error("Invalid input")]
    InvalidInput,
    /// Error from the readline library (I/O, Ctrl-C, Ctrl-D, etc.)
    #[error("readline error")]
    ReadlineError(#[from] ReadlineError),
    /// Runtime error during evaluation (unbound variable, type mismatch, etc.)
    #[error("computer error: {0}")]
    ComputerError(#[from] lisp_core::LispComputerError),
}
